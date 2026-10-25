//! La anexión, la fusión, el traslado de una Facción y los títulos de prestigio en el núcleo (sistema 5, tanda 5F; Diplomacia_Facciones §5.7, §6 y §7): proponer, contestar y retirar con sus
//! rechazos y su revalidación (quién puede, una sola pendiente, la caducidad, el Rey que ya no es el que propuso, la batalla abierta); el traslado de ciudadanos, columnas, plazas,
//! relaciones, propuestas, memoria, reputación y tecnologías sin perder ni duplicar nada; los trueques que siguen y ya no mueven la reputación (Q5); quien ya no puede estar dentro y sale; los
//! títulos con su cadencia, su Gran Rey y su crónica; la foto `BAN33` (hoy `BAN36`) y el determinismo. Casos de `typescript-final:src/session/__tests__/{comandosAnexion,comandosFusion}.test.ts` y
//! `engine/__tests__/eventosDominioTitulos.test.ts`. Las pruebas con plazas usan el artefacto de mundo real y se omiten si no está.

mod comun;

use asentamiento::Recurso;
use base::{Instante, Punto, MS_POR_MINUTO};
use comercio::trueque::EstadoTrueque;
use comun::{c, nucleo};
use militar::presencia::Ubicacion;
use nucleo::proyeccion::propios::{proyectar, Espectador};
use nucleo::{rechazos, Clase, Comando, Estado, Nucleo, ObjetivoAtaque, Peticion, Reglas, Respuesta, Resultado, TipoCreado};
use politica::titulos::TituloId;

const MIN: Instante = MS_POR_MINUTO;
const DIA: Instante = 24 * 60 * MIN;

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

fn codigo(r: &Respuesta) -> &'static str {
    match &r.resultado {
        Resultado::Rechazada { codigo, .. } => codigo,
        otro => panic!("se esperaba un rechazo: {otro:?}"),
    }
}

/// Un núcleo con su contador de peticiones.
struct E {
    n: Nucleo,
    k: u64,
}

impl E {
    fn de(n: Nucleo) -> Self {
        E { n, k: 0 }
    }
    fn ord(&mut self, actor: &str, c: Comando) -> Respuesta {
        self.k += 1;
        let p = Peticion::nueva(actor, &format!("o{}", self.k), c);
        // Las órdenes de plaza de quien mandaba antes del sistema 5 van como administración (andamiaje de 5E, `comun::mandaba`).
        let p = if actor == "admin" || comun::mandaba(&self.n, actor, &p.comando) { p.de_admin() } else { p };
        let t = self.n.ahora();
        self.n.aplicar_en(t, p)
    }
    fn ok(&mut self, actor: &str, c: Comando) -> Respuesta {
        let r = self.ord(actor, c);
        assert!(r.aceptada(), "{r:?}");
        r
    }
    /// Un rechazo que no deja rastro: ni versión, ni hash, ni ids gastados.
    fn no(&mut self, actor: &str, c: Comando) -> &'static str {
        let antes = (self.n.version(), self.n.hash_estado(), self.n.estado().siguiente_id);
        let r = self.ord(actor, c);
        assert_eq!((self.n.version(), self.n.hash_estado(), self.n.estado().siguiente_id), antes, "el rechazo {r:?} dejó rastro");
        codigo(&r)
    }
    fn avanzar(&mut self, ms: Instante) {
        let t = self.n.ahora() + ms;
        self.n.avanzar_hasta(t);
    }
    /// La columna de la cuenta (su héroe de campo es ciudadano de la Facción y, si es el primero, su Rey): (columna, héroe).
    fn columna(&mut self, cuenta: &str, faccion: u32, p: Punto) -> (u64, u64) {
        let t = self.n.ahora();
        let id = self.n.crear_ejercito(t, cuenta, faccion, p, 500, 350_000).unwrap();
        (id, self.n.ejercito(id).unwrap().columna.lider)
    }
    fn rey(&self, f: u32) -> u64 {
        self.n.faccion(f).unwrap().rey.unwrap()
    }
    fn hechos(&self, codigo: &str) -> Vec<nucleo::EventoNucleo> {
        self.n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter()).filter(|e| e.codigo == codigo).cloned().collect()
    }
    fn relacion(&mut self, cuenta_a: &str, a: u32, cuenta_b: &str, b: u32, vasallaje: bool) -> u64 {
        let (ra, rb) = (self.rey(a), self.rey(b));
        let tributo = vasallaje.then_some((Recurso::Madera, 2_000));
        let p = self.ok(cuenta_a, Comando::ProponerRelacion { heroe: ra, a, b, vasallaje, tributo }).creadas[0];
        self.ok(cuenta_b, Comando::ResponderRelacion { heroe: rb, propuesta: p, aceptar: true });
        self.n.relacion_activa(a, b).unwrap().id
    }
    fn anexion(&mut self, cuenta_a: &str, a: u32, cuenta_b: &str, b: u32) {
        let (ra, rb) = (self.rey(a), self.rey(b));
        let p = self.ok(cuenta_a, Comando::ProponerAnexion { heroe: ra, absorbente: a, absorbida: b }).creadas[0];
        self.ok(cuenta_b, Comando::ResponderAnexion { heroe: rb, propuesta: p, aceptar: true });
    }
    fn existe_en_admin(&self, clase: Clase, id: u64) -> bool {
        !proyectar(&self.n, Espectador::Admin, clase, id).existen.is_empty()
    }
}

fn dato(e: &nucleo::EventoNucleo, k: &str) -> i64 {
    e.dato(k).unwrap_or_else(|| panic!("{} no lleva «{k}»", e.codigo))
}

/// Facciones 1..=n con una columna cada una (cuentas `c1`...`cn`); sus héroes de campo son sus Reyes.
fn facciones(n: u32) -> E {
    let mut e = E::de(nucleo(60, 5));
    for f in 1..=n {
        e.columna(&format!("c{f}"), f, c(10 * f as i64, 2));
    }
    e
}

// ───────────────────────────── 1. La anexión ─────────────────────────────

#[test]
fn una_anexion_se_propone_se_rechaza_se_retira_y_caduca_con_sus_rechazos() {
    let mut e = facciones(3);
    let (r1, r2, r3) = (e.rey(1), e.rey(2), e.rey(3));
    let propone = |heroe: u64, absorbente: u32, absorbida: u32| Comando::ProponerAnexion { heroe, absorbente, absorbida };
    // La propone la autoridad diplomática de la absorbente (Rey o Embajador), a otra que existe, con Rey y que no es vasalla de una tercera.
    assert_eq!(e.no("c2", propone(r2, 1, 2)), rechazos::NO_AUTORIZADO, "el Rey de F2 no habla por F1");
    assert_eq!(e.no("c1", propone(r1, 1, 1)), rechazos::RELACION_CONSIGO_MISMA);
    assert_eq!(e.no("c1", propone(r1, 1, 9)), rechazos::FACCION_NO_EXISTE);
    e.relacion("c3", 3, "c2", 2, true);
    assert_eq!(e.no("c1", propone(r1, 1, 2)), rechazos::DIPLOMACIA_VASALLA_DE_TERCERA, "F2 es vasalla de F3");
    let p = e.ok("c3", propone(r3, 3, 2));
    assert_eq!(p.tipo_creadas, Some(TipoCreado::Propuesta), "su propia señora sí puede absorberla");
    let propuesta = p.creadas[0];
    assert_eq!(e.n.propuesta(propuesta).unwrap().expira_en, e.n.ahora() + 3 * DIA, "ANEXION.caducidadDias");
    let h = &e.hechos("diplomacia.anexion_propuesta")[0];
    assert_eq!((dato(h, "propuesta"), dato(h, "faccionA"), dato(h, "faccionB"), h.facciones.clone()), (propuesta as i64, 3, 2, vec![2, 3]));
    assert_eq!(e.no("c2", propone(r2, 2, 3)), rechazos::DIPLOMACIA_YA_PENDIENTE, "una sola pendiente por par, en cualquier sentido");
    // Contesta el Rey de la absorbida, nadie más; una propuesta de otra clase no vale.
    assert_eq!(e.no("c3", Comando::ResponderAnexion { heroe: r3, propuesta, aceptar: true }), rechazos::NO_AUTORIZADO);
    assert_eq!(e.no("c2", Comando::ResponderFusion { heroe: r2, propuesta, aceptar: true }), rechazos::DIPLOMACIA_PROPUESTA_DESCONOCIDA, "no es de fusión");
    assert_eq!(e.no("c2", Comando::ResponderAnexion { heroe: r2, propuesta: 9_999, aceptar: true }), rechazos::DIPLOMACIA_PROPUESTA_DESCONOCIDA);
    e.ok("c2", Comando::ResponderAnexion { heroe: r2, propuesta, aceptar: false });
    assert!(e.n.propuesta(propuesta).is_none() && e.n.faccion(2).is_some(), "rechazar la quita y no pasa nada más");
    assert_eq!(e.hechos("diplomacia.anexion_rechazada").len(), 1);
    // Retirar: la autoridad de la absorbente, mientras siga vigente.
    let p = e.ok("c3", propone(r3, 3, 2)).creadas[0];
    assert_eq!(e.no("c2", Comando::RetirarAnexion { heroe: r2, propuesta: p }), rechazos::NO_AUTORIZADO);
    e.ok("c3", Comando::RetirarAnexion { heroe: r3, propuesta: p });
    assert_eq!(e.hechos("diplomacia.anexion_retirada").len(), 1);
    assert_eq!(e.no("c3", Comando::RetirarAnexion { heroe: r3, propuesta: p }), rechazos::DIPLOMACIA_PROPUESTA_DESCONOCIDA);
    // La caducada no se contesta y no deja nada en la cola.
    let p = e.ok("c3", propone(r3, 3, 2)).creadas[0];
    e.avanzar(3 * DIA);
    assert_eq!(e.no("c2", Comando::ResponderAnexion { heroe: r2, propuesta: p, aceptar: true }), rechazos::DIPLOMACIA_PROPUESTA_CADUCADA);
    let citas: Vec<String> = e.n.estado().cola.volcar().into_iter().filter(|ev| e.n.estado().generaciones.es_vigente(ev.clave)).map(|ev| format!("{:?}", ev.carga)).collect();
    assert!(citas.iter().all(|c| !c.contains("Propuesta") && !c.contains("Anexion")), "{citas:?}");
}

#[test]
fn la_anexion_pasa_ciudadanos_columnas_vasallas_y_propuestas_y_rompe_lo_demas_sin_reputacion() {
    // F2 (beto) absorbida por F1: F2 tiene otro ciudadano (dora), es señora de F4 con tributo, aliada de F3 y F5 le propone una alianza; F5 también se la propone a F1.
    let mut e = facciones(5);
    let (_, dora) = e.columna("dora", 2, c(25, 3));
    let vasallaje = e.relacion("c2", 2, "c4", 4, true);
    let alianza = e.relacion("c2", 2, "c3", 3, false);
    let (r5, r1, r2) = (e.rey(5), e.rey(1), e.rey(2));
    let a_f2 = e.ok("c5", Comando::ProponerRelacion { heroe: r5, a: 5, b: 2, vasallaje: false, tributo: None }).creadas[0];
    let a_f1 = e.ok("c5", Comando::ProponerRelacion { heroe: r5, a: 5, b: 1, vasallaje: false, tributo: None }).creadas[0];
    e.ok("admin", Comando::FijarTecnologias { faccion: 2, tecnologias: vec!["escudos_ligeros".into()] });
    let (rep3, xp1) = (e.n.reputacion(3), e.n.faccion(1).unwrap().experiencia);
    let columnas_f2: Vec<u64> = e.n.estado().ejercitos.values().filter(|x| x.columna.faccion == Some(2)).map(|x| x.id).collect();
    assert_eq!(columnas_f2.len(), 2);

    e.anexion("c1", 1, "c2", 2);
    let t = e.n.ahora();
    // F2 desaparece; sus ciudadanos, en su orden, al final de los de F1, que conserva su Rey, su experiencia y su nombre.
    assert!(e.n.faccion(2).is_none() && !e.existe_en_admin(Clase::Faccion, 2), "F2 ya no existe (sale con QUITA)");
    let f1 = e.n.faccion(1).unwrap();
    assert_eq!((f1.ciudadanos.clone(), f1.rey, f1.experiencia), (vec![r1, r2, dora], Some(r1), xp1));
    assert!([r2, dora].iter().all(|&h| e.n.heroe(h).unwrap().faccion == Some(1)));
    assert!(columnas_f2.iter().all(|&x| e.n.ejercito(x).unwrap().columna.faccion == Some(1)), "sus columnas son de F1");
    // El vasallaje pasa a F1 (mismo id y tributo); la alianza con F3 se rompe sin reputación; la propuesta a F2 se descarta y la de F1 sigue.
    let v = e.n.relacion_activa(1, 4).unwrap();
    assert_eq!((v.id, v.a, v.b, v.tributo.map(|x| x.cantidad_por_minuto)), (vasallaje, 1, 4, Some(2_000)));
    assert!(!e.n.relacion_politica(alianza).unwrap().activa() && e.n.relacion(1, 3).is_none());
    assert_eq!(e.n.reputacion(3), rep3, "romper por traslado no cuesta reputación");
    assert_eq!(e.n.estado().comercio.reputacion[&3].alianzas(), 0, "el ritmo de F3 ya no cuenta la alianza");
    assert!(e.n.propuesta(a_f2).is_none() && e.n.propuesta(a_f1).is_some());
    assert!(!e.n.estado().comercio.reputacion.contains_key(&2) && !e.n.estado().militar.tecnologias.contains_key(&2), "la reputación y las tecnologías de F2 se pierden");
    // El hecho y la crónica.
    let h = &e.hechos("diplomacia.anexion")[0];
    assert_eq!((dato(h, "faccionA"), dato(h, "faccionB"), h.en), (1, 2, t));
    let cronica: Vec<_> = e.hechos("cronica.entrada").into_iter().filter(|x| x.valores.iter().any(|(k, v)| *k == "tipo" && *v == nucleo::Dato::Texto("diplomacia.anexion".into()))).collect();
    assert_eq!(cronica.len(), 1);
    // La propuesta que se aceptó ya no está.
    assert_eq!(e.no("c2", Comando::ResponderAnexion { heroe: r2, propuesta: 1, aceptar: true }), rechazos::DIPLOMACIA_PROPUESTA_DESCONOCIDA);
}

// ───────────────────────────── 2. La fusión ─────────────────────────────

#[test]
fn la_fusion_hace_nacer_una_faccion_con_su_nombre_su_rey_el_sigilo_de_a_y_lo_mayor_de_las_dos() {
    let mut e = facciones(4);
    let (r1, r2, r3) = (e.rey(1), e.rey(2), e.rey(3));
    let (_, ana2) = e.columna("ana2", 1, c(11, 3));
    let propone = |heroe: u64, nombre: &str, rey: u64| Comando::ProponerFusion { heroe, a: 1, b: 2, nombre: nombre.into(), rey };
    assert_eq!(e.no("c2", propone(r2, "Argos", r2)), rechazos::NO_AUTORIZADO, "solo el Rey de A");
    assert_eq!(e.no("c1", propone(r1, "Argos", r3)), rechazos::DIPLOMACIA_REY_NO_VALIDO);
    assert_eq!(e.no("c1", propone(r1, "   ", r2)), rechazos::FACCION_NOMBRE_VACIO);
    let tercera = e.n.faccion(3).unwrap().nombre.to_uppercase();
    assert_eq!(e.no("c1", propone(r1, &tercera, r2)), rechazos::FACCION_NOMBRE_DUPLICADO, "sin mirar mayúsculas");
    // El Rey de A ya no es quien la propuso: su consentimiento no vale.
    let p = e.ok("c1", propone(r1, " Argos ", r2)).creadas[0];
    assert_eq!(e.no("c2", Comando::ProponerFusion { heroe: r2, a: 2, b: 1, nombre: "Otra".into(), rey: r1 }), rechazos::DIPLOMACIA_YA_PENDIENTE);
    e.ok("c1", Comando::AsignarRey { heroe: r1, faccion: 1, designado: ana2 });
    assert_eq!(e.no("c2", Comando::ResponderFusion { heroe: r2, propuesta: p, aceptar: true }), rechazos::DIPLOMACIA_PROPONENTE_YA_NO_ES_REY);
    e.ok("ana2", Comando::AsignarRey { heroe: ana2, faccion: 1, designado: r1 });
    assert_eq!(e.no("ana2", Comando::RetirarFusion { heroe: ana2, propuesta: p }), rechazos::NO_AUTORIZADO, "retira el Rey de A");

    // Lo de cada una: F2 con más experiencia (nivel 4), F1 con mejor reputación (+6 por liberar a su vasalla F4), y una tecnología cada una.
    e.ok("admin", Comando::FijarNivelFaccion { faccion: 2, nivel: 4 });
    let v = e.relacion("c1", 1, "c4", 4, true);
    e.ok("c1", Comando::RomperRelacion { heroe: r1, relacion: v, iniciador: 1 });
    let rep1 = e.n.reputacion(1);
    assert!(rep1 > 0);
    e.ok("admin", Comando::FijarTecnologias { faccion: 1, tecnologias: vec!["escudos_ligeros".into()] });
    e.ok("admin", Comando::FijarTecnologias { faccion: 2, tecnologias: vec!["metalurgia_cobre".into()] });
    let (sigilo_a, xp2) = (e.n.faccion(1).unwrap().sigilo.clone(), e.n.faccion(2).unwrap().experiencia);

    let r = e.ok("c2", Comando::ResponderFusion { heroe: r2, propuesta: p, aceptar: true });
    assert_eq!(r.tipo_creadas, Some(TipoCreado::Faccion));
    let c = r.creadas[0] as u32;
    assert_eq!(c, 5, "el id siguiente a la mayor Facción del mundo");
    let f = e.n.faccion(c).unwrap();
    assert_eq!((f.nombre.as_str(), f.rey, f.embajador, f.sigilo.clone(), f.experiencia), ("Argos", Some(r2), None, sigilo_a, xp2));
    assert_eq!(f.ciudadanos, vec![r1, ana2, r2], "primero los de A, luego los de B; el otro Rey queda de ciudadano");
    assert_eq!(e.n.reputacion(c), rep1, "la reputación mayor de las dos");
    assert_eq!(e.n.estado().militar.tecnologias[&c].iter().cloned().collect::<Vec<_>>(), vec!["escudos_ligeros".to_string(), "metalurgia_cobre".to_string()]);
    assert!(e.n.faccion(1).is_none() && e.n.faccion(2).is_none());
    assert!(e.n.estado().ejercitos.values().all(|x| x.columna.faccion != Some(1) && x.columna.faccion != Some(2)));
    let h = &e.hechos("diplomacia.fusion")[0];
    assert_eq!((dato(h, "faccionA"), dato(h, "faccionB"), dato(h, "faccion"), dato(h, "rey")), (1, 2, i64::from(c), r2 as i64));
    assert_eq!(h.facciones, vec![1, 2, c]);
}

#[test]
fn el_id_de_una_faccion_que_desaparece_no_se_vuelve_a_dar() {
    // F1 absorbe a F3, la de id mayor, que no deja nada que la nombre; la Facción que nace de la fusión de F1 y F2 no es la «3».
    let mut e = facciones(3);
    e.anexion("c1", 1, "c3", 3);
    let (r1, r2) = (e.rey(1), e.rey(2));
    let p = e.ok("c1", Comando::ProponerFusion { heroe: r1, a: 1, b: 2, nombre: "Nueva".into(), rey: r1 }).creadas[0];
    let c = e.ok("c2", Comando::ResponderFusion { heroe: r2, propuesta: p, aceptar: true }).creadas[0];
    assert_eq!((c, e.n.estado().politica.mayor_desaparecida), (4, 3));
}

// ───────────────────────────── 3. Con plazas ─────────────────────────────

/// Dos Facciones con una plaza cada una cerca del campamento 0 (F1 en A, F2 en B), sin bandidos que molesten y sin mantenimiento por distancia a la capital (se mide el traslado, no la distancia).
fn dos_plazas(m: &comun::real::M) -> (E, u64, u64) {
    let mut reglas = Reglas::default();
    let b = &mut std::sync::Arc::make_mut(&mut reglas.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    std::sync::Arc::make_mut(&mut reglas.economia.datos).mantenimiento.escala_distancia = i64::MAX / 4;
    let arranque = nucleo::Arranque { reglas, ajustes: nucleo::Ajustes::default(), semilla: 5, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) };
    let mut e = E::de(Nucleo::nuevo(arranque));
    let pa = comun::real::sitio_rico(&e.n, &m.0, 0, 1);
    let a = comun::real::fundar(&mut e.n, "ana", 1, pa);
    let pb = comun::real::sitio_rico(&e.n, &m.0, 0, 2);
    let b = comun::real::fundar(&mut e.n, "beto", 2, pb);
    (e, a, b)
}

/// Todo lo que el traslado no puede perder ni duplicar, sin las Facciones ni las versiones: héroes con sus escuadras, almacén personal, carro, residencia y dónde están; columnas con sus
/// participantes, escuadras, carro, víveres y plan; plazas con su economía; flota, trueques y Caravanas de Fundación.
fn conservado(e: &Estado) -> String {
    let heroes: Vec<_> = e.militar.heroes.values().map(|h| (h.heroe.id, &h.heroe.escuadras, &h.almacen_personal, &h.carro, h.heroe.residencia, h.heroe.presencia.ubicacion, h.columna)).collect();
    let columnas: Vec<_> = e.ejercitos.values().map(|x| (x.id, &x.columna.participantes, &x.columna.escuadras, &x.columna.carga, x.columna.viveres, &x.plan, &x.caravanas_adjuntas)).collect();
    let plazas: Vec<_> = e.asentamientos.values().map(|a| (a.id, &a.economia, a.posicion, a.radio, &a.parcelas)).collect();
    let fundacion: Vec<_> = e.fundacion.caravanas.values().map(|k| (k.id, k.titular, &k.situacion)).collect();
    format!("{heroes:?}\n{columnas:?}\n{plazas:?}\n{:?}\n{:?}\n{fundacion:?}\n{:?}", e.comercio.flota, e.comercio.acuerdos, e.fundacion.fondos)
}

#[test]
fn la_anexion_con_plazas_no_pierde_ni_duplica_nada_y_los_trueques_siguen_sin_mover_la_reputacion() {
    let m = con_mundo!();
    let (mut e, a, b) = dos_plazas(m);
    // beto gobierna B: la plaza se va a F1 con sus cargos locales (5E).
    let beto = e.rey(2);
    comun::gobierna(&mut e.n, b, beto);
    // Un trueque activo entre las dos (Q5) y una Caravana de Fundación de una columna de F2.
    let tr = e.ok("ana", Comando::ProponerTrueque { a, b, lineas_a: vec![(Recurso::Madera, 50_000)], lineas_b: vec![(Recurso::Piedra, 30_000)] }).creadas[0];
    e.ok("beto", Comando::AceptarTrueque { acuerdo: tr });
    let pb = e.n.estado().asentamientos[&b].posicion;
    let (col, _) = e.columna("dora", 2, Punto::new(pb.x + 5_000, pb.y));
    e.ok("admin", Comando::DarCaravanaFundacion { ejercito: col });
    e.avanzar(30 * MIN);
    let vence = e.n.acuerdo(tr).unwrap().expira_en;
    let (antes, explorado_f2) = (conservado(e.n.estado()), e.n.estado().memoria.exploracion.get(&2).cloned());
    let ciudadanos: Vec<u64> = [1, 2].iter().flat_map(|&f| e.n.faccion(f).unwrap().ciudadanos.clone()).collect();

    e.anexion("ana", 1, "beto", 2);
    let t = e.n.ahora();
    assert_eq!(conservado(e.n.estado()), antes, "el traslado no pierde ni duplica ninguna entidad, recurso, tropa ni héroe");
    assert_eq!(e.n.faccion(1).unwrap().ciudadanos, ciudadanos, "los mismos héroes, sin repetir");
    // La plaza B es de F1 desde ahora (la capital de F1 sigue siendo A), con su residente y su zona; la Caravana de Fundación y la columna de dora son de F1.
    let pb = &e.n.estado().asentamientos[&b];
    assert_eq!((pb.faccion, pb.de_la_faccion_desde, pb.resto_tributo), (1, t, 0));
    assert_eq!(e.n.plazas_de(1).map(|x| x.id).collect::<Vec<_>>(), vec![a, b]);
    assert_eq!(e.n.capital_de(1), Some(a));
    assert!(e.n.estado().asentamientos[&b].cargos.es_titular(asentamiento::Cargo::Gobernador, beto), "beto sigue de Gobernador de B: es ciudadano de F1 y reside en ella");
    assert!(e.n.estado().fundacion.caravanas.values().all(|k| k.faccion == Some(1)));
    assert_eq!(e.n.ejercito(col).unwrap().columna.faccion, Some(1));
    // La memoria de F2 se funde en la de F1: lo explorado por F2 lo conoce F1; F2 ya no tiene memoria.
    let mem = &e.n.estado().memoria;
    assert!(!mem.exploracion.contains_key(&2) && !mem.fichas.contains_key(&2));
    if let Some(x) = explorado_f2 {
        let f1 = &mem.exploracion[&1];
        assert!(x.bytes().iter().zip(f1.bytes()).all(|(o, d)| o & !d == 0), "lo explorado por F2 está en F1");
    }
    assert!(!mem.fichas.get(&1).is_some_and(|f| f.contains_key(&nucleo::memoria::Sitio::Plaza(b))), "F1 no recuerda como ajena una plaza suya");
    // El trueque sigue entre las dos plazas, ahora de la misma Facción: al vencer sin entregas no penaliza a nadie (Q5).
    assert_eq!(e.n.acuerdo(tr).unwrap().estado, EstadoTrueque::Activo);
    let n0 = e.hechos("reputacion.cambio").len();
    e.n.avanzar_hasta(vence);
    assert_eq!(e.n.acuerdo(tr).unwrap().estado, EstadoTrueque::Expirado);
    assert_eq!(dato(&e.hechos("comercio.trueque_expirado")[0], "incumplido"), 1);
    assert_eq!(e.hechos("reputacion.cambio").len(), n0, "dentro de la misma Facción, vencer no mueve la reputación");
    assert_eq!(e.n.reputacion(1), 0);
}

#[test]
fn no_se_acepta_con_una_batalla_abierta_y_quien_ya_no_puede_estar_dentro_sale() {
    let m = con_mundo!();
    // F3 (gus), aliada de F2 y en guerra con F1, está dentro de la plaza B. F2 se deja anexionar por F1: gus está dentro de una plaza de su enemiga y sale al mapa.
    let (mut e, _a, b) = dos_plazas(m);
    let pb = e.n.estado().asentamientos[&b].posicion;
    let (_, gus) = e.columna("gus", 3, pb);
    e.relacion("beto", 2, "gus", 3, false);
    let r3 = e.rey(3);
    e.ok("gus", Comando::DeclararGuerra { heroe: r3, a: 3, b: 1 });
    e.ok("gus", Comando::Guarnecer { asentamiento: b, heroe: gus });
    assert_eq!(e.n.heroe(gus).unwrap().heroe.presencia.ubicacion, Ubicacion::Asentamiento(b as u32));
    // Una batalla abierta de alguna de las dos: F4 asedia la plaza B.
    let (r1, r2) = (e.rey(1), e.rey(2));
    let p = e.ok("ana", Comando::ProponerAnexion { heroe: r1, absorbente: 1, absorbida: 2 }).creadas[0];
    e.avanzar_hasta_que_se_pueda(b);
    let (_, h4) = e.columna("hugo", 4, Punto::new(pb.x + 2_000, pb.y));
    let arqueros = militar::catalogo::Tropa::ArquerosCompuesto;
    e.ok("admin", Comando::DarEscuadra { heroe: h4, tropa: arqueros, cantidad: militar::Reglas::cargar().tropas.tropa(arqueros).unidades });
    e.ok("hugo", Comando::Atacar { heroe: h4, objetivo: ObjetivoAtaque::Asentamiento(b), declara_guerra: true });
    assert!(e.n.batallas().any(|x| x.activa(e.n.ahora())), "el asedio es una batalla con convocatoria");
    assert_eq!(e.no("beto", Comando::ResponderAnexion { heroe: r2, propuesta: p, aceptar: true }), rechazos::DIPLOMACIA_BATALLA_ABIERTA);
    // Rechazar sí se puede; y sin batalla, aceptar.
    let (mut e, _a, b) = dos_plazas(m);
    let pb = e.n.estado().asentamientos[&b].posicion;
    let (_, gus) = e.columna("gus", 3, pb);
    e.relacion("beto", 2, "gus", 3, false);
    let r3 = e.rey(3);
    e.ok("gus", Comando::DeclararGuerra { heroe: r3, a: 3, b: 1 });
    e.ok("gus", Comando::Guarnecer { asentamiento: b, heroe: gus });
    e.anexion("ana", 1, "beto", 2);
    assert!(matches!(e.n.heroe(gus).unwrap().heroe.presencia.ubicacion, Ubicacion::Columna(_)), "gus salió al mapa");
    let h = e.hechos("heroe.expulsado").into_iter().find(|h| dato(h, "heroe") == gus as i64).expect("expulsado");
    assert_eq!(dato(&h, "motivo"), 2, "sale con el motivo de la conquista, `cambia_de_dueno` (5F-9)");
}

/// 5F-3 (decisión del autor, 2026-10-25): tampoco se acepta si un héroe de alguna de las dos combate en una batalla abierta de otros, incorporado a ella (cambiaría de Facción a mitad de la
/// batalla). Una vez acabada, sí.
#[test]
fn no_se_acepta_si_un_heroe_de_las_dos_combate_incorporado_en_una_batalla_ajena() {
    let m = con_mundo!();
    let (mut e, _a, _b) = dos_plazas(m);
    // F3 (gus) tiene la plaza C y es aliada de F2; dora, de F2, está dentro de C. F4 (hugo) asedia C y dora se une a la defensa.
    let pc = comun::real::sitio_rico(&e.n, &m.0, 0, 3);
    let c = comun::real::fundar(&mut e.n, "gus", 3, pc);
    e.relacion("beto", 2, "gus", 3, false);
    let (_, dora) = e.columna("dora", 2, pc);
    e.ok("dora", Comando::Guarnecer { asentamiento: c, heroe: dora });
    let (r1, r2) = (e.rey(1), e.rey(2));
    let p = e.ok("ana", Comando::ProponerAnexion { heroe: r1, absorbente: 1, absorbida: 2 }).creadas[0];
    e.avanzar_hasta_que_se_pueda(c);
    let (_, h4) = e.columna("hugo", 4, Punto::new(pc.x + 2_000, pc.y));
    let arqueros = militar::catalogo::Tropa::ArquerosCompuesto;
    e.ok("admin", Comando::DarEscuadra { heroe: h4, tropa: arqueros, cantidad: militar::Reglas::cargar().tropas.tropa(arqueros).unidades });
    e.ok("hugo", Comando::Atacar { heroe: h4, objetivo: ObjetivoAtaque::Asentamiento(c), declara_guerra: true });
    let id = e.n.batallas().find(|x| x.activa(e.n.ahora())).expect("el asedio es una batalla con convocatoria").id;
    let tk = &e.n.batalla(id).unwrap().ticket;
    assert_eq!((tk.atacante.faccion, tk.defensor.faccion), (Some(4), Some(3)), "ni F1 ni F2 son bando");
    e.ok("dora", Comando::UnirseABatalla { heroe: dora, batalla: id, lado: None });
    assert!(e.n.batalla(id).unwrap().heroes_que_combaten().any(|h| h == dora), "dora combate incorporada");
    assert_eq!(e.no("beto", Comando::ResponderAnexion { heroe: r2, propuesta: p, aceptar: true }), rechazos::DIPLOMACIA_BATALLA_ABIERTA);
    // Con la batalla acabada, se acepta.
    let fin = e.n.batalla(id).unwrap().expira_en.max(e.n.batalla(id).unwrap().convocatoria_hasta.unwrap_or(0));
    e.n.avanzar_hasta(fin + MIN);
    assert!(!e.n.batallas().any(|x| x.activa(e.n.ahora())), "la batalla acabó");
    e.ok("beto", Comando::ResponderAnexion { heroe: r2, propuesta: p, aceptar: true });
}

impl E {
    fn avanzar_hasta_que_se_pueda(&mut self, plaza: u64) {
        if let Some(t) = self.n.protegida_hasta(plaza).filter(|&t| t > self.n.ahora()) {
            self.n.avanzar_hasta(t);
        }
    }
}

// ───────────────────────────── 4. Los títulos ─────────────────────────────

#[test]
fn los_titulos_nacen_a_su_cadencia_cambian_de_manos_y_el_gran_rey_sale_de_la_liga() {
    let m = con_mundo!();
    let (mut e, a, b) = dos_plazas(m);
    // Nadie los arma sin que pase algo; pasa algo y vencen a los diez minutos (TITULOS.cadenciaMinutos), con un solo plazo para todo el mundo.
    let armado = e.n.estado().titulos_en.expect("lo armó lo que pasó al fundar");
    e.n.avanzar_hasta(armado);
    let nacen: Vec<String> = e.hechos("titulo.nace").iter().map(|h| h.valores.iter().find(|(k, _)| *k == "titulo").map(|(_, v)| format!("{v:?}")).unwrap()).collect();
    assert_eq!(nacen.len(), 3, "la más grande, la más rica y el mayor ejército; sin Gran Rey ni Mecenas: {nacen:?}");
    assert!(e.hechos("titulo.nace").iter().all(|h| h.publico));
    let grande = e.n.titulos().iter().find(|x| x.id == TituloId::FaccionMasGrande).copied().unwrap();
    assert_eq!(grande.valor, 1);
    assert!(e.existe_en_admin(Clase::Titulo, TituloId::FaccionMasGrande.indice()) && !e.existe_en_admin(Clase::Titulo, TituloId::GranRey.indice()));
    let titulos_en_cola = |e: &E| e.n.estado().cola.volcar().into_iter().filter(|ev| e.n.estado().generaciones.es_vigente(ev.clave)).filter(|ev| format!("{:?}", ev.carga) == "Titulos").count();
    assert!(titulos_en_cola(&e) <= 1);
    // F2 vasalla de F1: una Liga con su Gran Rey.
    e.relacion("ana", 1, "beto", 2, true);
    let liga = &e.n.ligas()[0];
    assert_eq!((liga.miembros.clone(), liga.gran_rey), (vec![1, 2], Some(1)));
    e.avanzar(10 * MIN);
    assert_eq!(e.hechos("titulo.nace").len(), 4, "nace el Gran Rey");
    assert_eq!(e.n.titulos().iter().find(|x| x.id == TituloId::GranRey).map(|x| x.poseedor), Some(1));
    // La señora absorbe a su vasalla: con dos plazas, F1 es la más grande; el vasallaje termina y el Gran Rey desaparece sin cantar nada.
    e.anexion("ana", 1, "beto", 2);
    e.avanzar(10 * MIN);
    let grande = e.n.titulos().iter().find(|x| x.id == TituloId::FaccionMasGrande).copied().unwrap();
    assert_eq!((grande.poseedor, grande.valor), (1, 2));
    assert!(e.n.titulos().iter().all(|x| x.id != TituloId::GranRey));
    let cambios = e.hechos("titulo.cambia_manos");
    assert!(cambios.iter().all(|h| dato(h, "faccion") == 1 && dato(h, "faccionAnterior") == 2), "{cambios:?}");
    let _ = (a, b);
    // La crónica canta cada título que nace o cambia de manos.
    let cantados = e.hechos("cronica.entrada").into_iter().filter(|x| x.valores.iter().any(|(k, v)| *k == "tipo" && matches!(v, nucleo::Dato::Texto(t) if t.starts_with("titulo.")))).count();
    assert_eq!(cantados, e.hechos("titulo.nace").len() + cambios.len());
    // Sin nada que pase en el mundo no se rearma: un mundo sin Facciones no tiene plazo.
    let quieto = E::de(nucleo(5, 5));
    assert_eq!(quieto.n.estado().titulos_en, None);
}

// ───────────────────────────── 5. Foto y determinismo ─────────────────────────────

fn guion(e: &mut E, paso: &dyn Fn(&mut E, Instante)) {
    let (r1, r2, r3, r4) = (e.rey(1), e.rey(2), e.rey(3), e.rey(4));
    let p = e.ok("c1", Comando::ProponerAnexion { heroe: r1, absorbente: 1, absorbida: 2 }).creadas[0];
    paso(e, 3 * MIN + 17);
    e.ok("c2", Comando::ResponderAnexion { heroe: r2, propuesta: p, aceptar: true });
    paso(e, 11 * MIN);
    let p = e.ok("c3", Comando::ProponerFusion { heroe: r3, a: 3, b: 4, nombre: "Tebas".into(), rey: r4 }).creadas[0];
    paso(e, 7 * MIN + 3);
    e.ok("c4", Comando::ResponderFusion { heroe: r4, propuesta: p, aceptar: true });
    paso(e, 13 * MIN);
}

#[test]
fn la_anexion_y_la_fusion_son_deterministas_y_los_titulos_viajan_en_la_foto_ban33() {
    let montar = || {
        let mut e = facciones(4);
        e.relacion("c2", 2, "c3", 3, false);
        e
    };
    let mut normal = montar();
    guion(&mut normal, &|e, ms| e.avanzar(ms));
    let mut trozos = montar();
    guion(&mut trozos, &|e, ms| {
        for _ in 0..4 {
            e.avanzar(ms / 4);
        }
        e.avanzar(ms % 4);
    });
    assert_eq!((normal.n.hash_estado(), normal.n.version()), (trozos.n.hash_estado(), trozos.n.version()));
    assert!(normal.n.faccion(5).is_some_and(|f| f.nombre == "Tebas") && normal.n.titulos().len() == 3);
    let (reglas, hash) = (normal.n.estado().reglas.clone(), normal.n.mundo_hash().to_string());
    let bytes = normal.n.estado().serializar();
    assert_eq!(&bytes[..5], b"BAN36");
    let s = Estado::deserializar(&bytes, &reglas, &hash).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!((s.politica.clone(), s.titulos_en, s.hash()), (normal.n.estado().politica.clone(), normal.n.estado().titulos_en, normal.n.hash_estado()));
    let vieja = [b"BAN32".as_slice(), &bytes[5..]].concat();
    let error = Estado::deserializar(&vieja, &reglas, &hash).err().expect("BAN32 se rechaza");
    assert!(error.0.contains("BAN32") && error.0.contains("títulos"), "{}", error.0);
    // Los títulos y su plazo mueven el hash.
    let mut otro = Estado::deserializar(&bytes, &reglas, &hash).unwrap();
    otro.politica.titulos[0].valor += 1;
    assert_ne!(otro.hash(), s.hash());
    let mut otro = Estado::deserializar(&bytes, &reglas, &hash).unwrap();
    otro.titulos_en = otro.titulos_en.map_or(Some(1), |t| Some(t + 1));
    assert_ne!(otro.hash(), s.hash());
    // En la cola, a lo sumo un plazo de títulos; nada por Facción, propuesta o relación.
    let citas: Vec<String> = normal.n.estado().cola.volcar().into_iter().filter(|ev| normal.n.estado().generaciones.es_vigente(ev.clave)).map(|ev| format!("{:?}", ev.carga)).collect();
    assert!(citas.iter().filter(|c| *c == "Titulos").count() <= 1 && citas.iter().all(|c| !c.contains("Propuesta") && !c.contains("Relacion")), "{citas:?}");
}

// ───────────────────────────── 6. Coste ─────────────────────────────

/// El coste de una fusión grande: dos Facciones con miles de columnas y ciudadanos (en release, `cargo test --release -p nucleo --test anexion_fusion coste -- --ignored --nocapture`).
#[test]
#[ignore]
fn coste_de_una_fusion_grande() {
    let n = std::env::var("BRONZEAGE_FUSION_COLUMNAS").ok().and_then(|x| x.parse().ok()).unwrap_or(2_000u32);
    let mut e = E::de(nucleo(200, 200));
    for f in [1u32, 2, 3] {
        e.columna(&format!("rey{f}"), f, c(f as i64, 1));
    }
    for i in 0..n {
        for f in [1u32, 2] {
            e.columna(&format!("c{f}-{i}"), f, c((i % 190) as i64 + 5, (i / 190) as i64 % 190 + 5));
        }
    }
    e.relacion("rey2", 2, "rey3", 3, true);
    let (r1, r2) = (e.rey(1), e.rey(2));
    let p = e.ok("rey1", Comando::ProponerFusion { heroe: r1, a: 1, b: 2, nombre: "Grande".into(), rey: r1 }).creadas[0];
    let antes = conservado(e.n.estado());
    let reloj = std::time::Instant::now();
    e.ok("rey2", Comando::ResponderFusion { heroe: r2, propuesta: p, aceptar: true });
    let dura = reloj.elapsed();
    assert_eq!(conservado(e.n.estado()), antes);
    let c = e.n.faccion(4).unwrap();
    assert_eq!(c.ciudadanos.len() as u32, 2 * n + 2);
    eprintln!("fusión de {} columnas y {} ciudadanos: {:.2} ms", 2 * n + 2, c.ciudadanos.len(), dura.as_secs_f64() * 1e3);
}
