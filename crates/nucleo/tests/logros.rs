//! La tecnología del mundo en el núcleo (sistema 6, tanda 6B; Tecnologia_Aedas §1-§3): el estado en la foto `BAN37` (hoy `BAN40`, con las citas de 6C), la cita global de la Era, los récords de plazas por nivel y
//! `FijarTecnologias` sobre las reglas reales. Cada vía de los contadores la prueba el binario de su sistema, con su hecho (D18: un hecho cuenta una vez y nada más):
//!
//! | Vía | Prueba |
//! |---|---|
//! | extracción (pueblo, D17) | `pueblos.rs`, `el_pueblo_extrae_contra_su_almacen…` |
//! | fabricación | `asentamiento/tests/simulacion.rs`, `el_paso_de_minuto_le_dice_al_mundo…` (el gancho) y `tecnologia::contador` (las cuatro recetas) |
//! | combate con números (columnas, caravana, acoso, campamento) | `combate.rs` |
//! | asedio con números tras su convocatoria | `murallas_asedio.rs` (con combate y sin él, con muralla y sin ella; también el primer logro) |
//! | batallas de Unity | `batallas.rs` (campo abierto, asedio resistido y conquistado, campamento, caravana) |
//! | reclutar en plaza y en campamento (Q4) | `heroes.rs` |
//! | animales | `combate.rs` y `batallas.rs` (las caravanas que se arman para las pruebas) |

mod comun;

use base::{Instante, MS_POR_MINUTO};
use comun::c;
use nucleo::{Carga, Comando, Dato, Estado, EventoNucleo, Nucleo, Peticion};
use tecnologia::estado::Primero;
use tecnologia::{Conjunto, Contador};

const DIA: Instante = 24 * 60 * MS_POR_MINUTO;
const SEMANA: Instante = 7 * DIA;

fn admin(n: &mut Nucleo, id: &str, c: Comando) {
    let r = n.aplicar_en(n.ahora(), Peticion::nueva("admin", id, c).de_admin());
    assert!(r.aceptada(), "{r:?}");
}

fn hechos(n: &Nucleo, codigo: &str) -> Vec<EventoNucleo> {
    n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter()).filter(|e| e.codigo == codigo).cloned().collect()
}

/// Un mundo sin plazas con una columna de la Facción 1 (la que hace existir a la Facción).
fn mundo_quieto() -> Nucleo {
    let mut n = comun::nucleo(10, 3);
    n.crear_ejercito(0, "ana", 1, c(2, 1), 500, 100_000).unwrap();
    n
}

#[test]
fn la_era_cambia_por_plazo_en_su_cita_global_y_la_ultima_no_tiene_cita() {
    let mut n = mundo_quieto();
    let r = n.estado().reglas.economia.tecnologia.clone();
    assert_eq!((n.tecnologia().era, n.orden_era()), (0, 1));
    // La cita se arma con lo primero que pasa en un mundo con alguna Facción.
    admin(&mut n, "t", Comando::FijarTecnologias { faccion: 1, tecnologias: vec!["escudos_ligeros".into()] });
    assert_eq!(n.estado().era_en, Some(r.eras[0].plazo));
    let plazo_1 = r.eras[0].plazo;
    n.avanzar_hasta(plazo_1 - 1);
    assert!(hechos(&n, "era.comienza").is_empty());
    n.avanzar_hasta(plazo_1);
    let h = hechos(&n, "era.comienza");
    assert_eq!(h.len(), 1);
    assert!(h[0].publico && h[0].en == plazo_1);
    assert_eq!((h[0].dato("orden"), h[0].dato("porPlazo"), h[0].valores.clone()), (Some(2), Some(1), vec![("era", Dato::Texto(r.eras[1].id.clone()))]));
    assert_eq!((n.tecnologia().era, n.tecnologia().era_desde, n.orden_era()), (1, plazo_1, 2));
    // La siguiente cita cuenta desde que empezó la Era II.
    assert_eq!(n.estado().era_en, Some(plazo_1 + r.eras[1].plazo));
    n.avanzar_hasta(plazo_1 + r.eras[1].plazo);
    assert_eq!((n.tecnologia().era, hechos(&n, "era.comienza").len()), (2, 2));
    // La III es la última mientras no existan los bloques de la IV y la V: sin cita, y la cola se queda vacía.
    assert_eq!(n.estado().era_en, None);
    n.avanzar_hasta(30 * SEMANA);
    assert_eq!(n.eventos_en_cola(), 0);
    assert_eq!(n.tecnologia().era, 2);
}

#[test]
fn un_mundo_sin_facciones_no_arma_la_cita_de_la_era() {
    let mut n = comun::nucleo(10, 3);
    n.avanzar_hasta(6 * SEMANA);
    assert_eq!((n.estado().era_en, n.eventos_en_cola(), n.tecnologia().era), (None, 0, 0));
}

/// Las mismas órdenes, avanzando de una vez, a trozos o con una foto a mitad: el mismo mundo.
fn guion(avanzar: &dyn Fn(&mut Nucleo, Instante)) -> Nucleo {
    let mut n = mundo_quieto();
    admin(&mut n, "t", Comando::FijarTecnologias { faccion: 1, tecnologias: vec!["escudos_ligeros".into(), "metalurgia_cobre".into()] });
    avanzar(&mut n, 5 * SEMANA + DIA);
    n
}

#[test]
fn la_era_es_la_misma_normal_acelerada_y_a_trozos() {
    let de_una = guion(&|n, t| n.avanzar_hasta(t));
    let a_trozos = guion(&|n, t| {
        let mut x = n.ahora();
        while x < t {
            x = (x + 7 * DIA / 3).min(t);
            n.avanzar_hasta(x);
        }
    });
    assert_eq!(de_una.hash_estado(), a_trozos.hash_estado());
    assert_eq!(de_una.tecnologia().era, 1);
}

#[test]
fn la_foto_ban37_lleva_el_estado_de_tecnologia_cada_campo_cambia_el_hash_y_ban36_se_rechaza() {
    let mut n = mundo_quieto();
    admin(&mut n, "t", Comando::FijarTecnologias { faccion: 1, tecnologias: vec!["escudos_ligeros".into()] });
    let reglas = n.estado().reglas.clone();
    let mundo_hash = n.estado().mundo_hash.clone();
    let bytes = n.estado().serializar();
    assert_eq!(&bytes[..5], b"BAN40");
    let vuelta = Estado::deserializar(&bytes, &reglas, &mundo_hash).unwrap();
    assert_eq!(vuelta.serializar(), bytes);
    assert_eq!((vuelta.tecnologia.clone(), vuelta.era_en), (n.estado().tecnologia.clone(), n.estado().era_en));
    assert!(vuelta.cola.volcar().iter().any(|e| e.carga == Carga::Era), "la cita de la Era viaja en la cola");

    let vieja = [b"BAN36".as_slice(), &bytes[5..]].concat();
    let error = Estado::deserializar(&vieja, &reglas, &mundo_hash).err().expect("BAN36 se rechaza");
    assert!(error.0.contains("BAN36") && error.0.contains("tecnología"), "{}", error.0);

    type Cambio = (&'static str, Box<dyn Fn(&mut Estado)>);
    let cambios: Vec<Cambio> = vec![
        ("era", Box::new(|s| s.tecnologia.era += 1)),
        ("era_desde", Box::new(|s| s.tecnologia.era_desde += 1)),
        ("contador", Box::new(|s| s.tecnologia.contadores[Contador::BatallasLibradas.indice()] += 1)),
        ("otro contador", Box::new(|s| s.tecnologia.contadores[Contador::PlazasEnNivel3.indice()] += 1)),
        ("resto", Box::new(|s| s.tecnologia.restos[Contador::ExtraidoCobre.indice()] += 1)),
        ("logro", Box::new(|s| s.tecnologia.logros[3] = Some(9))),
        ("primero", Box::new(|s| s.tecnologia.primeros[3] = Some(Primero { faccion: 2, en: 1 }))),
        ("primero.en", Box::new(|s| s.tecnologia.primeros.iter_mut().flatten().for_each(|p| p.en += 1))),
        ("cantadas", Box::new(|s| s.tecnologia.cantadas.poner(4))),
        ("aparecidas", Box::new(|s| s.tecnologia.por_faccion.get_mut(&1).unwrap().aparecidas.poner(5))),
        ("adoptadas", Box::new(|s| s.tecnologia.por_faccion.get_mut(&1).unwrap().adoptadas.quitar(0))),
        ("reveladas", Box::new(|s| s.tecnologia.por_faccion.get_mut(&1).unwrap().reveladas.poner(6))),
        ("otra Facción", Box::new(|s| {
            s.tecnologia.por_faccion.insert(2, tecnologia::TecnologiasFaccion { aparecidas: Conjunto(3), adoptadas: Conjunto(3), reveladas: Conjunto::VACIO });
        })),
        ("era_en", Box::new(|s| s.era_en = None)),
    ];
    let mut vistos = std::collections::HashSet::from([vuelta.hash()]);
    for (nombre, f) in cambios {
        let mut s = Estado::deserializar(&bytes, &reglas, &mundo_hash).unwrap();
        f(&mut s);
        assert!(vistos.insert(s.hash()), "el hash no cambió (o coincidió con otro cambio) al tocar {nombre}");
        let de_nuevo = Estado::deserializar(&s.serializar(), &reglas, &mundo_hash).unwrap();
        assert_eq!(de_nuevo.serializar(), s.serializar(), "{nombre} da la vuelta");
    }
}

#[test]
fn fijar_tecnologias_las_hace_aparecer_y_adoptar_sin_quitar_lo_aparecido_y_la_descubridora_es_la_primera() {
    let mut n = mundo_quieto();
    n.crear_ejercito(0, "beto", 2, c(5, 1), 500, 100_000).unwrap();
    let rt = n.estado().reglas.economia.tecnologia.clone();
    let cobre = rt.buscar("metalurgia_cobre").unwrap();
    admin(&mut n, "t1", Comando::FijarTecnologias { faccion: 1, tecnologias: vec!["metalurgia_cobre".into()] });
    n.avanzar_hasta(MS_POR_MINUTO);
    admin(&mut n, "t2", Comando::FijarTecnologias { faccion: 2, tecnologias: vec!["metalurgia_cobre".into()] });
    assert_eq!(n.tecnologia().primeros[cobre as usize], Some(Primero { faccion: 1, en: 0 }), "la primera que la tuvo");
    // Quitársela a la Facción 1 deja de adoptarla, pero aparecer es para siempre (adoptadas ⊆ aparecidas).
    admin(&mut n, "t3", Comando::FijarTecnologias { faccion: 1, tecnologias: vec![] });
    let f1 = n.tecnologia().de(&rt, 1);
    assert!(f1.aparecidas.contiene(cobre) && !f1.adoptadas.contiene(cobre) && f1.adoptadas == rt.de_arranque);
    assert!(f1.adoptadas.es_subconjunto_de(f1.aparecidas));
    assert!(!n.tiene_tecnologia(1, "metalurgia_cobre") && n.tiene_tecnologia(2, "metalurgia_cobre") && n.tiene_tecnologia(1, "leva_comunal"));
    // Ninguna sin logro ni Era: la administración no fija logros ni cambia la Era.
    assert!(n.tecnologia().logros.iter().all(Option::is_none) && n.tecnologia().era == 0);
}

#[test]
fn el_record_de_plazas_por_nivel_actual_sube_con_el_indice_y_no_baja() {
    let Some(m) = comun::real::mundo() else { return };
    let a = nucleo::Arranque { reglas: comun::reglas(), ajustes: nucleo::Ajustes::default(), semilla: 7, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) };
    let mut n = Nucleo::nuevo(a);
    let p1 = comun::real::sitio_rico(&n, &m.0, 0, 1);
    let x = comun::real::fundar(&mut n, "ana", 1, p1);
    let p2 = comun::real::sitio_rico(&n, &m.0, 3, 2);
    let y = comun::real::fundar(&mut n, "beto", 2, p2);
    let record = |n: &Nucleo, c: Contador| n.tecnologia().contador(c);
    assert_eq!((n.plazas_en_nivel(1), n.plazas_en_nivel(2), record(&n, Contador::PlazasEnNivel2)), (2, 0, 0));
    admin(&mut n, "n1", Comando::FijarNivelAsentamiento { asentamiento: x, nivel: 3 });
    assert_eq!((n.plazas_en_nivel(2), n.plazas_en_nivel(3), record(&n, Contador::PlazasEnNivel2), record(&n, Contador::PlazasEnNivel3)), (1, 1, 1, 1));
    admin(&mut n, "n2", Comando::FijarNivelAsentamiento { asentamiento: y, nivel: 2 });
    assert_eq!((n.plazas_en_nivel(2), n.plazas_en_nivel(3), record(&n, Contador::PlazasEnNivel2), record(&n, Contador::PlazasEnNivel3)), (2, 1, 2, 1));
    // Una foto reconstruye el índice (no va en ella) y conserva el récord.
    let reglas = n.estado().reglas.clone();
    let vuelta = Estado::deserializar(&n.estado().serializar(), &reglas, &m.0.hash().to_string()).unwrap();
    assert_eq!(vuelta.tecnologia.contador(Contador::PlazasEnNivel2), 2);
}
