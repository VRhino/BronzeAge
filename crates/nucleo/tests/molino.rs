//! El buey del molino (tanda 69 §10): la orden `AsignarBuey` cobra el buey de una caravana, marca el molino y se pierde con él. Sobre el artefacto de mundo real
//! (`generador/salida/mundo/mundo-*.bin`); las pruebas se omiten si no está.

mod comun;
use asentamiento::catalogo::Tipo;
use asentamiento::edificio::Estado;
use asentamiento::Recurso;
use base::{Instante, Punto, MS_POR_MINUTO};
use mundo::{Mundo, OpcionesValidacion};
use nucleo::{rechazos, Ajustes, Arranque, Comando, Nucleo, Peticion, Reglas, Resultado};
use rutas::Rutas;
use std::sync::{Arc, OnceLock};

const DIA: i64 = 24 * 60 * MS_POR_MINUTO;

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

/// Una plaza recién fundada (nivel 1) con un molino ya encargado a mano; `orden` numera las peticiones.
struct Plaza {
    n: Nucleo,
    id: u64,
    molino: u32,
    orden: u32,
}

fn admin(id: &str, c: Comando) -> Peticion {
    Peticion::nueva("admin", id, c).de_admin()
}

fn codigo(r: &nucleo::Respuesta) -> &'static str {
    match &r.resultado {
        Resultado::Rechazada { codigo, .. } => codigo,
        otro => panic!("se esperaba un rechazo: {otro:?}"),
    }
}

/// Primer punto fundable y verde en anillos alrededor del primer campamento.
fn sitio(n: &Nucleo, m: &Mundo) -> Punto {
    let p = m.campamentos()[0].posicion.unwrap();
    let c = Punto::new(p.x, p.y);
    for r in (0..8).map(|k| 50_000 + k * 10_000) {
        for g in 0..360 {
            let a = (g as f64).to_radians();
            let p = Punto::new(c.x + (r as f64 * a.cos()) as i64, c.y + (r as f64 * a.sin()) as i64);
            if n.viabilidad(p, 1).is_some_and(|v| v.motivo.is_none() && v.color == territorio::Viabilidad::Verde) {
                return p;
            }
        }
    }
    panic!("sin sitio fundable cerca del campamento 0");
}

impl Plaza {
    fn fundar(m: &M) -> Plaza {
        // Sin bandidos: esta prueba mide otra cosa.
        let mut reglas = Reglas::default();
        let b = &mut Arc::make_mut(&mut reglas.economia.combate).bandidos;
        (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
        let mut n = Nucleo::nuevo(Arranque { reglas, ajustes: Ajustes::default(), semilla: 7, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) });
        let p = sitio(&n, &m.0);
        let e = n.crear_ejercito(0, "ana", 1, p, 500, 100_000).unwrap();
        assert!(n.aplicar_en(0, admin("c", Comando::DarCaravanaFundacion { ejercito: e })).aceptada());
        let r = n.aplicar_en(0, comun::como_dueno(Peticion::nueva("ana", "f", Comando::Fundar { ejercito: e })));
        assert!(r.aceptada(), "{r:?}");
        let mut plaza = Plaza { n, id: r.creadas[0], molino: 0, orden: 0 };
        plaza.dar(Recurso::Madera, 500_000);
        plaza.dar(Recurso::Piedra, 500_000);
        plaza.molino = plaza.levantar_molino();
        plaza
    }

    fn como_admin(&mut self, c: Comando) -> nucleo::Respuesta {
        let t = self.n.ahora();
        self.orden += 1;
        self.n.aplicar_en(t, admin(&format!("a{}", self.orden), c))
    }

    fn dar(&mut self, recurso: Recurso, cantidad: i64) {
        let c = Comando::DarRecursos { asentamiento: self.id, recurso, cantidad };
        assert!(self.como_admin(c).aceptada());
    }

    /// Da la orden como la dueña de la plaza (la administración, andamiaje de 5E).
    fn manda(&mut self, c: Comando) -> nucleo::Respuesta {
        let t = self.n.ahora();
        self.orden += 1;
        self.n.aplicar_en(t, comun::como_dueno(Peticion::nueva("ana", &format!("o{}", self.orden), c)))
    }

    fn levantar_molino(&mut self) -> u32 {
        let r = self.manda(Comando::AnadirEdificio { asentamiento: self.id, tipo: Tipo::Molino });
        assert!(r.aceptada(), "{r:?}");
        r.creadas[0] as u32
    }

    fn oro(&self) -> i64 {
        self.n.economia(self.id).unwrap().almacen.cantidad(Recurso::Oro)
    }

    fn buey(&self, edificio: u32) -> bool {
        self.n.economia(self.id).unwrap().edificio(edificio).unwrap().buey
    }

    fn asignar(&mut self, edificio: u32) -> nucleo::Respuesta {
        self.manda(Comando::AsignarBuey { asentamiento: self.id, edificio })
    }
}

/// Lo que cuesta el buey de una caravana, que es lo que cuesta el del molino (`ANIMAL_CATALOGO.buey.costo`): 12 de oro.
fn precio_del_buey(p: &Plaza) -> i64 {
    let costo = &p.n.estado().reglas.economia.comercio.flota.buey.costo;
    assert_eq!(costo.len(), 1, "el buey cuesta solo oro");
    assert_eq!(costo[0].0, Recurso::Oro);
    costo[0].1
}

#[test]
fn el_buey_cobra_el_oro_del_buey_de_una_caravana_marca_el_molino_y_no_se_compra_dos_veces() {
    let m = con_mundo!();
    let mut p = Plaza::fundar(m);
    let precio = precio_del_buey(&p);
    assert_eq!(precio, 12_000, "12 de oro, como el buey de una caravana");
    p.dar(Recurso::Oro, 50_000);
    let oro = p.oro();
    let molino = p.molino;
    let r = p.asignar(molino);
    assert!(r.aceptada(), "{r:?}");
    assert_eq!((p.oro(), p.buey(molino)), (oro - precio, true), "cobra el buey y marca el molino");
    let hechos: Vec<_> = p.n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter()).filter(|e| e.codigo == "construccion.buey_asignado").map(|e| (e.asentamientos.clone(), e.datos.clone())).collect();
    assert_eq!(hechos, vec![(vec![p.id], vec![("edificio", molino as i64)])]);
    // Una segunda vez, rechazada y sin cobrar.
    let r = p.asignar(molino);
    assert_eq!(codigo(&r), rechazos::MOLINO_YA_TIENE_BUEY);
    assert_eq!(p.oro(), oro - precio, "no se cobra dos veces");
}

#[test]
fn sin_oro_bastante_el_buey_se_rechaza_y_no_marca_nada() {
    let m = con_mundo!();
    let mut p = Plaza::fundar(m);
    let precio = precio_del_buey(&p);
    // La plaza nace con oro para unos cuantos bueyes: se gasta comprándolos a molinos cuya obra se cancela, hasta que no llega para uno más.
    let mut molino = p.molino;
    while p.oro() >= precio {
        assert!(p.asignar(molino).aceptada());
        assert!(p.manda(Comando::CancelarConstruccion { asentamiento: p.id, edificio: molino }).aceptada());
        p.dar(Recurso::Madera, 100_000);
        p.dar(Recurso::Piedra, 100_000);
        molino = p.levantar_molino();
    }
    let oro = p.oro();
    assert_eq!(codigo(&p.asignar(molino)), rechazos::OBRA_RECURSOS_INSUFICIENTES);
    assert_eq!((p.oro(), p.buey(molino)), (oro, false), "sin oro no se cobra ni se marca");
}

#[test]
fn el_buey_solo_se_asigna_a_un_molino_que_existe() {
    let m = con_mundo!();
    let mut p = Plaza::fundar(m);
    p.dar(Recurso::Oro, 50_000);
    let oro = p.oro();
    let granja = p.n.economia(p.id).unwrap().edificios.iter().find(|e| e.tipo == Tipo::Granja).unwrap().id;
    assert_eq!(codigo(&p.asignar(granja)), rechazos::MOLINO_NO_ES_MOLINO);
    assert_eq!(codigo(&p.asignar(9_999)), rechazos::EDIFICIO_DESCONOCIDO);
    assert_eq!(p.oro(), oro, "ningún rechazo cobra");
}

#[test]
fn sin_permiso_en_la_plaza_el_buey_se_rechaza_como_las_demas_ordenes_de_obra() {
    let m = con_mundo!();
    let mut p = Plaza::fundar(m);
    p.dar(Recurso::Oro, 50_000);
    let t = p.n.ahora();
    let r = p.n.aplicar_en(t, Peticion::nueva("luis", "x", Comando::AsignarBuey { asentamiento: p.id, edificio: p.molino }));
    assert_eq!(codigo(&r), rechazos::NO_AUTORIZADO);
    assert!(!p.buey(p.molino));
}

#[test]
fn el_buey_se_pierde_con_el_molino_y_el_que_se_levanta_despues_nace_sin_el() {
    let m = con_mundo!();
    let mut p = Plaza::fundar(m);
    p.dar(Recurso::Oro, 50_000);
    let molino = p.molino;
    assert_eq!(p.n.economia(p.id).unwrap().edificio(molino).unwrap().estado, Estado::EnConstruccion);
    assert!(p.asignar(molino).aceptada());
    // Cancelar la obra lo derriba, con su buey y sin devolver nada.
    let oro = p.oro();
    assert!(p.manda(Comando::CancelarConstruccion { asentamiento: p.id, edificio: molino }).aceptada());
    assert!(p.n.economia(p.id).unwrap().edificio(molino).is_none());
    assert_eq!(p.oro(), oro, "el buey no se devuelve");
    let otro = p.levantar_molino();
    assert!(!p.buey(otro), "el molino nuevo nace sin buey");
    assert!(p.asignar(otro).aceptada(), "y se le puede comprar otro");
}

#[test]
fn la_auto_construccion_nunca_levanta_ninguno_de_los_cinco_edificios_de_la_comida() {
    let m = con_mundo!();
    let mut p = Plaza::fundar(m);
    let molino = p.molino;
    assert!(p.manda(Comando::CancelarConstruccion { asentamiento: p.id, edificio: molino }).aceptada());
    for r in [Recurso::Madera, Recurso::Piedra, Recurso::Oro, Recurso::Trigo] {
        p.dar(r, 300_000);
    }
    let hasta: Instante = 3 * DIA;
    p.n.avanzar_hasta(hasta);
    let eco = p.n.economia(p.id).unwrap();
    assert!(eco.edificios.len() > 4, "la plaza sí construyó sola: {} edificios", eco.edificios.len());
    let nuevos = [Tipo::Molino, Tipo::Horno, Tipo::Quesera, Tipo::Bodega, Tipo::Prensa];
    assert!(eco.edificios.iter().all(|e| !nuevos.contains(&e.tipo)), "solo se hacen a mano");
}
