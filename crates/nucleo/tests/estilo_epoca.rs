//! El estilo de época de cada plaza (Tecnologia_Aedas §4.2, decisión del autor del 2026-11-16): el máximo (Era, grado) al que llegó por las tecnologías adoptadas de su Facción. Sube al adoptar
//! (también por administración), nace con el de su Facción, en un cambio de dueño toma el del nuevo si va por delante y nunca baja; viaja en la foto y a quien ve la plaza por dentro. Sobre el
//! artefacto de mundo real (se omiten si no está).

mod comun;

use nucleo::proyeccion::propios::{proyectar, Espectador};
use nucleo::{Ajustes, Arranque, Clase, Comando, Estado, Nucleo, Peticion, Reglas, Respuesta};
use tecnologia::estilo::Estilo;

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

struct E {
    n: Nucleo,
    k: u64,
}

impl E {
    fn nuevo(m: &comun::real::M) -> Self {
        let mut reglas = Reglas::default();
        let b = &mut std::sync::Arc::make_mut(&mut reglas.economia.combate).bandidos;
        (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
        let datos = std::sync::Arc::make_mut(&mut reglas.economia.datos);
        datos.mantenimiento.escala_distancia = i64::MAX / 4;
        // Almacenes grandes: caben las tarifas de adopción.
        datos.capacidad_inicial = 100_000_000;
        let arranque = Arranque { reglas, ajustes: Ajustes::default(), semilla: 5, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) };
        E { n: Nucleo::nuevo(arranque), k: 0 }
    }

    fn fundar(&mut self, m: &comun::real::M, cuenta: &str, faccion: u32) -> u64 {
        let p = comun::real::sitio_rico(&self.n, &m.0, 0, faccion);
        comun::real::fundar(&mut self.n, cuenta, faccion, p)
    }

    fn ok(&mut self, actor: &str, c: Comando) -> Respuesta {
        self.k += 1;
        let p = Peticion::nueva(actor, &format!("o{}", self.k), c);
        let p = if actor == "admin" { p.de_admin() } else { p };
        let r = self.n.aplicar_en(self.n.ahora(), p);
        assert!(r.aceptada(), "{r:?}");
        r
    }

    /// Las tecnologías de la Era `orden` que no son de arranque, por su id, en el orden del catálogo.
    fn de_la_era(&self, orden: u32) -> Vec<String> {
        let r = &self.n.estado().reglas.economia.tecnologia;
        r.catalogo.iter().enumerate().filter(|(i, d)| !d.de_arranque && r.orden_era(*i as tecnologia::TecId) == orden).map(|(_, d)| d.id.clone()).collect()
    }

    /// La Facción `f` tiene adoptadas, además de las de arranque, `k` de cada Era de `por_era` (`(orden, k)`), por administración.
    fn fijar(&mut self, f: u32, por_era: &[(u32, usize)]) {
        let tecnologias = por_era.iter().flat_map(|&(orden, k)| self.de_la_era(orden).into_iter().take(k)).collect();
        self.ok("admin", Comando::FijarTecnologias { faccion: f, tecnologias });
    }

    fn estilo(&self, plaza: u64) -> Estilo {
        self.n.estado().asentamientos[&plaza].estilo
    }

    fn rey(&self, f: u32) -> u64 {
        self.n.faccion(f).unwrap().rey.unwrap()
    }
}

fn est(era: u8, grado: u8) -> Estilo {
    Estilo { era, grado }
}

#[test]
fn sube_al_adoptar_por_administracion_y_nunca_baja() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let a = e.fundar(m, "ana", 1);
    assert_eq!(e.estilo(a), Estilo::BASE, "solo con las de arranque: la Era I entera");
    // Más tecnologías de la Era I no cambian nada.
    e.fijar(1, &[(1, 2)]);
    assert_eq!(e.estilo(a), Estilo::BASE);
    // De la Era II: primeros atisbos, más cambiada y del todo; una cuarta no pasa del 3.
    for (k, grado) in [(1, 1), (2, 2), (3, 3), (4, 3)] {
        e.fijar(1, &[(2, k)]);
        assert_eq!(e.estilo(a), est(2, grado), "{k} de la Era II");
    }
    // Una de la Era III va por delante de toda la II.
    e.fijar(1, &[(2, 1), (3, 1)]);
    assert_eq!(e.estilo(a), est(3, 1));
    // Quitarle tecnologías por administración no lo baja: guarda el máximo.
    e.fijar(1, &[]);
    assert_eq!(e.estilo(a), est(3, 1), "nunca baja");
    // Viaja en la foto.
    let (reglas, hash) = (e.n.estado().reglas.clone(), e.n.estado().mundo_hash.clone());
    let vuelta = Estado::deserializar(&e.n.estado().serializar(), &reglas, &hash).unwrap();
    assert_eq!(vuelta.asentamientos[&a].estilo, est(3, 1));
}

#[test]
fn nace_con_el_de_su_faccion_y_la_orden_de_adoptar_sube_cada_plaza_a_su_maximo() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    // La Facción 1 nace con su primera columna; antes de fundar tiene dos de la Era II.
    let p = comun::real::sitio_rico(&e.n, &m.0, 0, 1);
    let t = e.n.ahora();
    let col = e.n.crear_ejercito(t, "ana", 1, p, 500, 100_000).unwrap();
    e.fijar(1, &[(2, 2)]);
    e.ok("admin", Comando::DarCaravanaFundacion { ejercito: col });
    let a = e.ok("ana", Comando::Fundar { ejercito: col }).creadas[0];
    assert_eq!(e.estilo(a), est(2, 2), "nace con el de su Facción");
    // Se las quita la administración (siguen aparecidas): la plaza conserva el suyo, y la que se funda ahora nace en la Era I.
    e.fijar(1, &[]);
    e.ok("admin", Comando::FijarNivelFaccion { faccion: 1, nivel: 4 });
    let b = e.fundar(m, "bea", 1);
    assert_eq!((e.estilo(a), e.estilo(b)), (est(2, 2), Estilo::BASE));
    // El Rey, en la capital (la fundó y está dentro), adopta una con la tarifa de su Era: cada plaza toma el máximo entre el suyo y el de la Facción.
    let x = e.de_la_era(2)[0].clone();
    let r = e.n.estado().reglas.economia.tecnologia.clone();
    let x = r.buscar(&x).unwrap();
    for &(recurso, cantidad) in r.tarifa(x) {
        e.ok("admin", Comando::DarRecursos { asentamiento: a, recurso, cantidad });
    }
    let rey = e.rey(1);
    let (va, vb) = (e.n.version_de(Clase::Asentamiento, a), e.n.version_de(Clase::Asentamiento, b));
    e.ok("ana", Comando::AdoptarTecnologia { heroe: rey, faccion: 1, tecnologia: x });
    assert_eq!((e.estilo(a), e.estilo(b)), (est(2, 2), est(2, 1)));
    assert!(e.n.version_de(Clase::Asentamiento, b) > vb, "la que sube se marca");
    assert!(e.n.version_de(Clase::Asentamiento, a) > va, "la capital paga la tarifa: también cambia");
}

#[test]
fn en_la_anexion_se_queda_el_mayor_y_solo_se_ve_por_dentro() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let a = e.fundar(m, "ana", 1);
    let b = e.fundar(m, "beto", 2);
    let c = e.fundar(m, "cris", 3);
    // F1 va por la Era II (2 de ella), F2 ya pisa la III y F3 sigue en la I.
    e.fijar(1, &[(2, 2)]);
    e.fijar(2, &[(3, 1)]);
    assert_eq!((e.estilo(a), e.estilo(b), e.estilo(c)), (est(2, 2), est(3, 1), Estilo::BASE));
    // F1 se anexiona F2: la plaza de F2 conserva el suyo, que va por delante; la de F1 no cambia (sus tecnologías siguen siendo las suyas).
    let (r1, r2) = (e.rey(1), e.rey(2));
    let p = e.ok("ana", Comando::ProponerAnexion { heroe: r1, absorbente: 1, absorbida: 2 }).creadas[0];
    e.ok("beto", Comando::ResponderAnexion { heroe: r2, propuesta: p, aceptar: true });
    assert_eq!(e.n.estado().asentamientos[&b].faccion, 1);
    assert_eq!((e.estilo(a), e.estilo(b)), (est(2, 2), est(3, 1)));
    // F1 se anexiona F3: su plaza sube a la de F1.
    let r3 = e.rey(3);
    let p = e.ok("ana", Comando::ProponerAnexion { heroe: r1, absorbente: 1, absorbida: 3 }).creadas[0];
    e.ok("cris", Comando::ResponderAnexion { heroe: r3, propuesta: p, aceptar: true });
    assert_eq!(e.estilo(c), est(2, 2), "la anexionada toma el del nuevo dueño, que va por delante");
    // Lo que se ve: su Facción y quien la mira por dentro, sí; la Facción de fuera, no.
    let interior = nucleo::proyeccion::interior::asentamiento(&e.n, c).unwrap();
    assert_eq!(interior.estilo_epoca, Some(contrato::v1::EstiloEpoca { era: 2, grado: 2 }));
    let propia = |n: &Nucleo, h: u64, id: u64| match proyectar(n, Espectador::Heroe(Some(h)), Clase::Asentamiento, id).existen.pop().map(|x| x.2) {
        Some(contrato::v1::entidad::Cuerpo::Asentamiento(p)) => p,
        otro => panic!("{otro:?}"),
    };
    assert_eq!(propia(&e.n, r1, b).estilo_epoca, Some(contrato::v1::EstiloEpoca { era: 3, grado: 1 }));
    let (t, p) = (e.n.ahora(), comun::real::sitio_rico(&e.n, &m.0, 0, 4));
    let dora = e.n.crear_ejercito(t, "dora", 4, p, 500, 100_000).unwrap();
    let dora = e.n.ejercito(dora).unwrap().columna.lider;
    assert_eq!(propia(&e.n, dora, b).estilo_epoca, None, "desde fuera no se sabe");
}

#[test]
fn en_la_fusion_cada_plaza_toma_el_de_la_union_si_va_por_delante() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let a = e.fundar(m, "ana", 1);
    let b = e.fundar(m, "beto", 2);
    // Cada una con una distinta de la Era III: la Facción nueva tiene las dos, más que cualquiera de ellas.
    let iii = e.de_la_era(3);
    e.ok("admin", Comando::FijarTecnologias { faccion: 1, tecnologias: vec![iii[0].clone()] });
    e.ok("admin", Comando::FijarTecnologias { faccion: 2, tecnologias: vec![iii[1].clone()] });
    assert_eq!((e.estilo(a), e.estilo(b)), (est(3, 1), est(3, 1)));
    let (r1, r2) = (e.rey(1), e.rey(2));
    let p = e.ok("ana", Comando::ProponerFusion { heroe: r1, a: 1, b: 2, nombre: "Argos".into(), rey: r1 }).creadas[0];
    let c = e.ok("beto", Comando::ResponderFusion { heroe: r2, propuesta: p, aceptar: true }).creadas[0] as u32;
    assert_eq!((e.n.estado().asentamientos[&a].faccion, e.n.estado().asentamientos[&b].faccion), (c, c));
    assert_eq!((e.estilo(a), e.estilo(b)), (est(3, 2), est(3, 2)));
}
