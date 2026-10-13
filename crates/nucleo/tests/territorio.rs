//! Mundo y territorio sobre el artefacto real (`generador/salida/mundo/mundo-*.bin`); se omiten si no está.
//! Fundar, zonas que crecen y chocan, parcelas de bosque, y que todo sobreviva a una foto y a una repetición del registro.

use base::{Punto, MS_POR_MINUTO};
use mundo::{Mundo, OpcionesValidacion};
use nucleo::{rechazos, Arranque, Ajustes, Comando, Nucleo, Peticion, Reglas, Resultado};
use rutas::Rutas;
use std::sync::{Arc, OnceLock};

const MIN: i64 = MS_POR_MINUTO;

fn mundo() -> Option<&'static (Arc<Mundo>, Arc<Rutas>)> {
    static M: OnceLock<Option<(Arc<Mundo>, Arc<Rutas>)>> = OnceLock::new();
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

fn arranque(m: &(Arc<Mundo>, Arc<Rutas>)) -> Arranque {
    Arranque { reglas: Reglas::default(), ajustes: Ajustes::default(), semilla: 7, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) }
}

fn nuevo(m: &(Arc<Mundo>, Arc<Rutas>)) -> Nucleo {
    Nucleo::nuevo(arranque(m))
}

fn admin(id: &str, c: Comando) -> Peticion {
    Peticion::nueva("admin", id, c).de_admin()
}

fn campamento(m: &Mundo, i: usize) -> Punto {
    let p = m.campamentos()[i].posicion.unwrap();
    Punto::new(p.x, p.y)
}

/// Primer punto fundable (verde si se pide) en anillos de radio `r`, `r + 10`, … alrededor del campamento `i`.
fn sitio(n: &Nucleo, m: &Mundo, i: usize, r: i64, grado0: i64, faccion: u32, exigir_verde: bool) -> Punto {
    let c = campamento(m, i);
    for r in (0..8).map(|k| r + k * 10_000) {
        for g in 0..360 {
            let a = ((grado0 + g) as f64).to_radians();
            let p = Punto::new(c.x + (r as f64 * a.cos()) as i64, c.y + (r as f64 * a.sin()) as i64);
            if let Some(v) = n.viabilidad(p, faccion) {
                if v.motivo.is_none() && (!exigir_verde || v.color == territorio::Viabilidad::Verde) {
                    return p;
                }
            }
        }
    }
    panic!("sin sitio fundable cerca del campamento {i}");
}

/// Una columna de `faccion` en `p` con caravana, que funda; devuelve el id de la plaza.
fn fundar_en(n: &mut Nucleo, t: i64, nombre: &str, faccion: u32, p: Punto) -> u64 {
    let e = n.crear_ejercito(t, nombre, "ana", faccion, p, 500, 100_000, 100_000).unwrap();
    assert!(n.aplicar_en(t, admin(&format!("c-{nombre}"), Comando::DarCaravanaFundacion { ejercito: e })).aceptada());
    let r = n.aplicar_en(t, Peticion::nueva("ana", &format!("f-{nombre}"), Comando::Fundar { ejercito: e }));
    assert!(r.aceptada(), "{r:?}");
    r.creadas[0]
}

fn codigo(r: &nucleo::Respuesta) -> &'static str {
    match &r.resultado {
        Resultado::Rechazada { codigo, .. } => codigo,
        otro => panic!("se esperaba un rechazo: {otro:?}"),
    }
}

fn eventos(n: &Nucleo, codigo: &str) -> Vec<(i64, Vec<u64>)> {
    n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter()).filter(|e| e.codigo == codigo).map(|e| (e.en, e.asentamientos.clone())).collect()
}

#[test]
fn fundar_y_sus_rechazos() {
    let m = con_mundo!();
    let mut n = nuevo(m);
    let p = sitio(&n, &m.0, 0, 60_000, 0, 1, false);
    let e = n.crear_ejercito(0, "h1", "ana", 1, p, 500, 100_000, 100_000).unwrap();

    // Sin caravana no se funda; el que no es el propietario tampoco.
    assert_eq!(codigo(&n.aplicar_en(0, Peticion::nueva("ana", "f0", Comando::Fundar { ejercito: e }))), rechazos::SIN_CARAVANA);
    assert!(n.aplicar_en(0, admin("c", Comando::DarCaravanaFundacion { ejercito: e })).aceptada());
    assert_eq!(codigo(&n.aplicar_en(0, Peticion::nueva("luis", "f1", Comando::Fundar { ejercito: e }))), rechazos::NO_AUTORIZADO);

    // Fundar: la plaza nace con el radio inicial, la caravana se gasta y queda el rastro.
    let v0 = n.version();
    let r = n.aplicar_en(MIN, Peticion::nueva("ana", "f2", Comando::Fundar { ejercito: e }));
    assert!(r.aceptada(), "{r:?}");
    let id = r.creadas[0];
    assert!(n.version() > v0);
    let a = &n.estado().asentamientos[&id];
    assert_eq!((a.faccion, a.nivel, a.posicion, a.fundado_en), (1, 1, p, MIN));
    assert_eq!(n.radio_zona(id), Some(6_000));
    assert!(!n.estado().ejercitos[&e].caravana_fundacion);
    assert_eq!(eventos(&n, "fundacion.asentamiento_fundado"), vec![(MIN, vec![id])]);
    assert_eq!(codigo(&n.aplicar_en(MIN, Peticion::nueva("ana", "f3", Comando::Fundar { ejercito: e }))), rechazos::SIN_CARAVANA);

    // Cerca de una plaza (a 10) y dentro de su zona (encima).
    for (nombre, q, esperado) in [("cerca", Punto::new(p.x + 10_000, p.y), "fundacion.cerca_de_plaza"), ("encima", p, "fundacion.dentro_de_zona")] {
        let e2 = n.crear_ejercito(MIN, nombre, "ana", 2, q, 500, 1, 1).unwrap();
        n.aplicar_en(MIN, admin(&format!("c-{nombre}"), Comando::DarCaravanaFundacion { ejercito: e2 }));
        let r = n.aplicar_en(MIN, Peticion::nueva("ana", &format!("f-{nombre}"), Comando::Fundar { ejercito: e2 }));
        assert_eq!(codigo(&r), esperado, "{nombre}");
    }
    // Encima de un campamento: la exclusión de 20.
    let c0 = campamento(&m.0, 0);
    let e3 = n.crear_ejercito(MIN, "campa", "ana", 3, c0, 500, 1, 1).unwrap();
    n.aplicar_en(MIN, admin("c-campa", Comando::DarCaravanaFundacion { ejercito: e3 }));
    assert_eq!(codigo(&n.aplicar_en(MIN, Peticion::nueva("ana", "f-campa", Comando::Fundar { ejercito: e3 }))), "fundacion.cerca_de_campamento");
    // Fuera del mapa.
    let e4 = n.crear_ejercito(MIN, "fuera", "ana", 4, Punto::new(-5_000, 0), 500, 1, 1).unwrap();
    n.aplicar_en(MIN, admin("c-fuera", Comando::DarCaravanaFundacion { ejercito: e4 }));
    assert_eq!(codigo(&n.aplicar_en(MIN, Peticion::nueva("ana", "f-fuera", Comando::Fundar { ejercito: e4 }))), "fundacion.fuera_del_mapa");
}

#[test]
fn el_cap_de_fundacion_sube_con_el_nivel_de_faccion() {
    let m = con_mundo!();
    let mut n = nuevo(m);
    let primera = sitio(&n, &m.0, 0, 60_000, 0, 1, false);
    fundar_en(&mut n, 0, "a", 1, primera);
    let segunda = sitio(&n, &m.0, 0, 90_000, 90, 1, false);
    let e = n.crear_ejercito(0, "b", "ana", 1, segunda, 500, 1, 1).unwrap();
    n.aplicar_en(0, admin("cb", Comando::DarCaravanaFundacion { ejercito: e }));
    let intento = |n: &mut Nucleo, id: &str| n.aplicar_en(0, Peticion::nueva("ana", id, Comando::Fundar { ejercito: e }));
    assert_eq!(codigo(&intento(&mut n, "i1")), rechazos::CAP_ALCANZADO, "nivel 1 de Facción: una plaza");
    assert!(n.aplicar_en(0, admin("n2", Comando::FijarNivelFaccion { faccion: 1, nivel: 2 })).aceptada());
    assert!(intento(&mut n, "i2").aceptada(), "nivel 2: dos plazas");
    assert_eq!(n.plazas_de(1).count(), 2);
}

#[test]
fn la_zona_crece_cada_minuto_hasta_el_techo_de_su_nivel() {
    let m = con_mundo!();
    let mut n = nuevo(m);
    let p = sitio(&n, &m.0, 1, 60_000, 0, 1, false);
    let id = fundar_en(&mut n, 0, "a", 1, p);
    n.avanzar_hasta(10 * MIN);
    assert_eq!(n.radio_zona(id), Some(8_000), "0,2 u/min");
    assert!(eventos(&n, "zona.techo").is_empty());
    n.avanzar_hasta(25 * MIN);
    assert_eq!(n.radio_zona(id), Some(10_000), "techo del nivel 1");
    assert_eq!(eventos(&n, "zona.techo"), vec![(20 * MIN, vec![id])]);
    // Subir de nivel levanta el techo sin saltos: sigue desde donde estaba.
    assert!(n.aplicar_en(25 * MIN, admin("n2", Comando::FijarNivelAsentamiento { asentamiento: id, nivel: 2 })).aceptada());
    assert_eq!(n.radio_zona(id), Some(10_000));
    n.avanzar_hasta(35 * MIN);
    assert_eq!(n.radio_zona(id), Some(12_000));
    n.avanzar_hasta(60 * MIN);
    assert_eq!(n.radio_zona(id), Some(15_000));
    assert_eq!(eventos(&n, "zona.techo").len(), 2);
    // Un edificio completado agranda 1 unidad de golpe, sin pasar del techo.
    assert!(n.aplicar_en(60 * MIN, admin("n3", Comando::FijarNivelAsentamiento { asentamiento: id, nivel: 3 })).aceptada());
    assert!(n.aplicar_en(60 * MIN, admin("g1", Comando::AgrandarZona { asentamiento: id, milesimas: 1_000 })).aceptada());
    assert_eq!(n.radio_zona(id), Some(16_000));
    // El nivel no baja ni se sale de la tabla.
    assert_eq!(codigo(&n.aplicar_en(60 * MIN, admin("n1", Comando::FijarNivelAsentamiento { asentamiento: id, nivel: 1 }))), rechazos::NIVEL_INVALIDO);
    assert_eq!(codigo(&n.aplicar_en(60 * MIN, admin("n9", Comando::FijarNivelAsentamiento { asentamiento: id, nivel: 9 }))), rechazos::NIVEL_INVALIDO);
}

/// Dos plazas de distinta Facción a `d` (en un anillo del mismo campamento, buscando un par fundable).
fn par(n: &mut Nucleo, m: &Mundo, d: i64, f: (u32, u32), t: i64) -> (u64, u64, Punto, Punto) {
    let pa = sitio(n, m, 2, 70_000, 0, f.0, false);
    let a = fundar_en(n, t, "pa", f.0, pa);
    for g in 0..360 {
        let r = (g as f64).to_radians();
        let pb = Punto::new(pa.x + (d as f64 * r.cos()) as i64, pa.y + (d as f64 * r.sin()) as i64);
        if n.viabilidad(pb, f.1).is_some_and(|v| v.motivo.is_none()) {
            let b = fundar_en(n, t, "pb", f.1, pb);
            return (a, b, pa, pb);
        }
    }
    panic!("sin sitio para la segunda plaza");
}

#[test]
fn las_zonas_de_dos_facciones_se_tocan_en_el_instante_exacto() {
    let m = con_mundo!();
    let mut n = nuevo(m);
    let (a, b, pa, pb) = par(&mut n, &m.0, 32_000, (1, 2), 0);
    let d = pa.dist(pb);
    // Con el techo del nivel 1 (10+10 < 32) no se tocan nunca...
    n.avanzar_hasta(3 * 60 * MIN);
    assert!(eventos(&n, "zona.frontera").is_empty());
    // ...pero con el nivel 3 (techo 20) sí. Radios en 10+10 = 20; faltan d−20 a 0,4 por minuto.
    for (id, x) in [(a, "na"), (b, "nb")] {
        assert!(n.aplicar_en(3 * 60 * MIN, admin(x, Comando::FijarNivelAsentamiento { asentamiento: id, nivel: 3 })).aceptada());
    }
    let esperado = 3 * 60 * MIN + ((d - 20_000) * MIN + 399) / 400;
    n.avanzar_hasta(esperado - 1);
    assert!(eventos(&n, "zona.frontera").is_empty(), "un milisegundo antes aún no");
    let punto_a = Punto::new((pa.x * 7 + pb.x * 3) / 10, (pa.y * 7 + pb.y * 3) / 10); // al 70 % del camino hacia b
    assert!(n.en_zona(a, punto_a) == (pa.dist(punto_a) <= n.radio_zona(a).unwrap()), "sin contacto, la zona es el disco");
    n.avanzar_hasta(esperado + 5);
    let ev = eventos(&n, "zona.frontera");
    assert_eq!(ev.len(), 1);
    assert!((ev[0].0 - esperado).abs() <= 1, "contacto en {} y se esperaba {esperado}", ev[0].0);
    assert_eq!(ev[0].1, vec![a.min(b), a.max(b)]);
    // Ahora la frontera recorta: ningún punto de la zona de a pasa del plano de reparto; el polígono lo respeta.
    n.avanzar_hasta(esperado + 20 * MIN);
    let (ra, rb) = (n.radio_zona(a).unwrap(), n.radio_zona(b).unwrap());
    let k = d * ra / (ra + rb); // distancia de a a la frontera
    assert!(n.en_zona(a, Punto::new(pa.x + (pb.x - pa.x) * (k - 100) / d, pa.y + (pb.y - pa.y) * (k - 100) / d)));
    assert!(!n.en_zona(a, Punto::new(pa.x + (pb.x - pa.x) * (k + 300) / d, pa.y + (pb.y - pa.y) * (k + 300) / d)));
    let poligono = n.poligono_zona(a);
    assert_eq!(poligono.len() >= 3, true);
    let ux = (pb.x - pa.x) as f64 / d as f64;
    let uy = (pb.y - pa.y) as f64 / d as f64;
    assert!(poligono.iter().all(|q| ((q.x - pa.x) as f64 * ux + (q.y - pa.y) as f64 * uy) <= k as f64 + 2.0));
    // No se repite el aviso aunque cambien los ritmos.
    n.aplicar_en(esperado + 20 * MIN, admin("g", Comando::AgrandarZona { asentamiento: a, milesimas: 500 }));
    n.avanzar_hasta(esperado + 60 * MIN);
    assert_eq!(eventos(&n, "zona.frontera").len(), 1);
}

#[test]
fn las_zonas_de_la_misma_faccion_se_solapan_sin_frontera() {
    let m = con_mundo!();
    let mut n = nuevo(m);
    n.aplicar_en(0, admin("nf", Comando::FijarNivelFaccion { faccion: 1, nivel: 2 }));
    let (a, b, ..) = par(&mut n, &m.0, 32_000, (1, 1), 0);
    for (id, x) in [(a, "na"), (b, "nb")] {
        n.aplicar_en(0, admin(x, Comando::FijarNivelAsentamiento { asentamiento: id, nivel: 3 }));
    }
    n.avanzar_hasta(2 * 60 * MIN);
    assert!(eventos(&n, "zona.frontera").is_empty());
    assert!(n.estado().contactos.is_empty());
}

#[test]
fn las_parcelas_respetan_zona_cupo_y_exclusividad() {
    let m = con_mundo!();
    let mut n = nuevo(m);
    // Un sitio verde: con bosque libre en la zona inicial.
    let p = sitio(&n, &m.0, 0, 50_000, 0, 1, true);
    let id = fundar_en(&mut n, 0, "a", 1, p);
    // Para tener alcance, sube la plaza a nivel 3 y deja crecer la zona.
    n.aplicar_en(0, admin("n3", Comando::FijarNivelAsentamiento { asentamiento: id, nivel: 3 }));
    n.avanzar_hasta(80 * MIN);
    let mut tomadas = Vec::new();
    loop {
        let r = n.aplicar_en(80 * MIN, admin(&format!("p{}", tomadas.len()), Comando::ReclamarParcela { asentamiento: id }));
        match &r.resultado {
            Resultado::Aceptada => tomadas.push(r.creadas[0] as u32),
            Resultado::Rechazada { codigo, .. } => {
                assert!(*codigo == rechazos::PARCELA_SIN_LIBRES || *codigo == rechazos::PARCELA_CUPO_AGOTADO, "{codigo}");
                break;
            }
            otro => panic!("{otro:?}"),
        }
    }
    assert!(!tomadas.is_empty(), "un sitio verde tiene al menos una parcela");
    // Todas caen en la zona, ninguna se repite, y por masa no pasa del cupo.
    let parcelas = m.0.parcelas();
    let mut por_masa = std::collections::BTreeMap::new();
    for &i in &tomadas {
        let c = parcelas[i as usize].centro.unwrap();
        assert!(n.en_zona(id, Punto::new(c.x, c.y)));
        *por_masa.entry(parcelas[i as usize].bosque_id.clone()).or_insert(0usize) += 1;
    }
    assert_eq!(tomadas.len(), tomadas.iter().collect::<std::collections::BTreeSet<_>>().len());
    for (bosque, n_tomadas) in por_masa {
        let total = parcelas.iter().filter(|q| q.bosque_id == bosque).count();
        assert!(n_tomadas <= (total * 50 / 100).max(1), "{bosque}: {n_tomadas} de {total}");
    }
    // Soltar devuelve la parcela; la ajena no se puede soltar.
    let primera = tomadas[0];
    assert!(n.aplicar_en(80 * MIN, admin("s", Comando::SoltarParcela { asentamiento: id, parcela: primera })).aceptada());
    assert!(!n.estado().parcelas_ocupadas.contains_key(&primera));
    assert_eq!(codigo(&n.aplicar_en(80 * MIN, admin("s2", Comando::SoltarParcela { asentamiento: id, parcela: primera }))), rechazos::PARCELA_AJENA);
    assert_eq!(codigo(&n.aplicar_en(80 * MIN, Peticion::nueva("ana", "p-usuario", Comando::ReclamarParcela { asentamiento: id }))), rechazos::NO_AUTORIZADO);
}

#[test]
fn la_viabilidad_avisa_pero_no_impide() {
    let m = con_mundo!();
    let n = nuevo(m);
    let c = campamento(&m.0, 0);
    let en_campamento = n.viabilidad(c, 1).unwrap();
    assert_eq!(en_campamento.motivo, Some(territorio::Motivo::CercaDeCampamento));
    assert_eq!(en_campamento.color, territorio::Viabilidad::Rojo);
    let verde = sitio(&n, &m.0, 0, 50_000, 0, 1, true);
    let v = n.viabilidad(verde, 1).unwrap();
    assert!(v.parcelas_libres > 0 && v.piedra > 0 && v.motivo.is_none());
}

#[test]
fn el_territorio_sobrevive_a_la_foto_y_al_registro() {
    let m = con_mundo!();
    let dir = tempfile::tempdir().unwrap();
    let ajustes = Ajustes { foto_cada: 0, ..Ajustes::default() };
    let con = |ajustes: Ajustes| Arranque { ajustes, ..arranque(m) };
    let hash = {
        let mut n = Nucleo::con_persistencia(con(ajustes.clone()), dir.path(), Default::default()).unwrap();
        let (a, b, ..) = par(&mut n, &m.0, 32_000, (1, 2), 0);
        for (id, x) in [(a, "na"), (b, "nb")] {
            n.aplicar_en(MIN, admin(x, Comando::FijarNivelAsentamiento { asentamiento: id, nivel: 3 }));
        }
        n.aplicar_en(MIN, admin("p", Comando::ReclamarParcela { asentamiento: a }));
        n.avanzar_hasta(30 * MIN);
        n.foto_ahora(); // a mitad: la foto lleva ritmos, contactos pendientes y parcelas
        n.avanzar_hasta(60 * MIN);
        n.apagar().unwrap();
        n.hash_estado()
    };
    let n = Nucleo::con_persistencia(con(ajustes), dir.path(), Default::default()).unwrap();
    assert_eq!(n.hash_estado(), hash, "foto + registro reconstruyen el mismo mundo");
    assert_eq!(n.estado().asentamientos.len(), 2);
    assert_eq!(n.estado().contactos.len(), 1);
}

#[test]
fn la_plaza_se_proyecta_al_contrato_con_su_radio_como_ritmo() {
    use contrato::v1 as pb;
    let m = con_mundo!();
    let mut n = nuevo(m);
    let p = sitio(&n, &m.0, 0, 60_000, 0, 1, false);
    let id = fundar_en(&mut n, 5 * MIN, "a", 1, p);
    let a = &n.estado().asentamientos[&id];
    let Some(pb::entidad::Cuerpo::Asentamiento(e)) = nucleo::proyeccion::entidad_asentamiento(a, &n.poligono_zona(id)).cuerpo else { panic!("no es un asentamiento") };
    assert_eq!((e.id.as_str(), e.faccion_id.as_str(), e.nivel), (format!("as-{id}").as_str(), "f1", 1));
    let r = e.radio_potencial.unwrap();
    assert_eq!((r.valor_en_t0, r.t0_ms, r.por_minuto, r.maximo), (6_000, 5 * MIN, 200, Some(10_000)));
    assert_eq!(e.zona.len(), 48, "sin vecinas, el polígono es el disco entero");
}
