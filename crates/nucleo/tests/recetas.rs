//! La receta de la bodega y la prensa (`TALLERES_DE_UNA_RECETA`, 2026-11-09): la orden `ElegirReceta`, sus rechazos, el tope de tres edificios por plaza,
//! la proyección de `receta` y `receta_parada`. Sobre el artefacto de mundo real (`generador/salida/mundo/mundo-*.bin`); las pruebas se omiten si no está.

mod comun;
use asentamiento::catalogo::Tipo;
use asentamiento::Recurso::{self, *};
use base::{Punto, MS_POR_MINUTO};
use contrato::v1 as pb;
use mundo::{Mundo, OpcionesValidacion};
use nucleo::{rechazos, Ajustes, Arranque, Comando, Nucleo, Peticion, Reglas, Resultado};
use rutas::Rutas;
use std::sync::{Arc, OnceLock};

const HORA: i64 = 60 * MS_POR_MINUTO;

type M = (Arc<Mundo>, Arc<Rutas>);
/// Un hecho de crónica: las plazas que lo oyen y sus datos.
type Hecho = (Vec<u64>, Vec<(&'static str, i64)>);

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

/// Una plaza de Ana fundada, con la auto-construcción parada (solo se levanta lo que se pide) y recursos de sobra; `orden` numera las peticiones.
struct Plaza {
    n: Nucleo,
    id: u64,
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
    /// Una plaza del nivel alcanzado `nivel`, con `materiales` de madera y de piedra más de los que trae.
    fn fundar(m: &M, nivel: u32, materiales: i64) -> Plaza {
        let mut reglas = Reglas::default();
        let b = &mut Arc::make_mut(&mut reglas.economia.combate).bandidos;
        (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
        let mut n = Nucleo::nuevo(Arranque { reglas, ajustes: Ajustes::default(), semilla: 7, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) });
        let p = sitio(&n, &m.0);
        let e = n.crear_ejercito(0, "ana", 1, p, 500, 100_000).unwrap();
        assert!(n.aplicar_en(0, admin("c", Comando::DarCaravanaFundacion { ejercito: e })).aceptada());
        let r = n.aplicar_en(0, comun::como_dueno(Peticion::nueva("ana", "f", Comando::Fundar { ejercito: e })));
        assert!(r.aceptada(), "{r:?}");
        let mut plaza = Plaza { n, id: r.creadas[0], orden: 0 };
        let id = plaza.id;
        assert!(plaza.como_admin(Comando::AlternarAutoConstruccion { asentamiento: id, pausada: true }).aceptada());
        if nivel > 1 {
            assert!(plaza.como_admin(Comando::FijarNivelAsentamiento { asentamiento: id, nivel }).aceptada());
        }
        plaza.dar(Madera, materiales);
        plaza.dar(Piedra, materiales);
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

    fn levantar(&mut self, tipo: Tipo) -> u32 {
        let r = self.manda(Comando::AnadirEdificio { asentamiento: self.id, tipo });
        assert!(r.aceptada(), "{tipo:?}: {r:?}");
        r.creadas[0] as u32
    }

    fn elegir(&mut self, edificio: u32, insumo: Recurso) -> nucleo::Respuesta {
        self.manda(Comando::ElegirReceta { asentamiento: self.id, edificio, insumo })
    }

    fn receta(&self, edificio: u32) -> Option<Recurso> {
        self.n.economia(self.id).unwrap().edificio(edificio).unwrap().receta
    }

    /// Lo que proyecta el interior del edificio: `(receta, receta_parada)`.
    fn proyectado(&self, edificio: u32) -> (Option<String>, bool) {
        let p: pb::Asentamiento = nucleo::proyeccion::interior::asentamiento(&self.n, self.id).unwrap();
        let e = p.edificios.iter().find(|e| e.id == nucleo::proyeccion::ids::edificio(self.id, edificio)).expect("el edificio está en el interior");
        (e.receta.clone(), e.receta_parada)
    }

    fn hechos(&self) -> Vec<Hecho> {
        self.n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter()).filter(|e| e.codigo == "construccion.receta_elegida").map(|e| (e.asentamientos.clone(), e.datos.clone())).collect()
    }
}

#[test]
fn elegir_la_receta_de_una_bodega_la_cambia_la_proyecta_y_deja_su_hecho() {
    let m = con_mundo!();
    let mut p = Plaza::fundar(m, 5, 2_000_000);
    let bodega = p.levantar(Tipo::Bodega);
    assert_eq!((p.receta(bodega), p.proyectado(bodega)), (Some(Uva), (Some("uva".into()), false)), "nace con la primera receta del catálogo");
    let r = p.elegir(bodega, Cebada);
    assert!(r.aceptada(), "{r:?}");
    assert_eq!((p.receta(bodega), p.proyectado(bodega)), (Some(Cebada), (Some("cebada".into()), false)));
    assert_eq!(p.hechos(), vec![(vec![p.id], vec![("edificio", bodega as i64), ("insumo", Cebada.indice() as i64)])]);
    // Elegir la misma otra vez no cambia nada y no deja otro hecho.
    let r = p.elegir(bodega, Cebada);
    assert_eq!(r.resultado, Resultado::SinCambios);
    assert_eq!(p.hechos().len(), 1);
    // Una plaza de nivel 5 trabaja vino y cerveza a la vez; la nueva bodega nace con la receta que la plaza ya trabaja.
    let otra = p.levantar(Tipo::Bodega);
    assert_eq!(p.receta(otra), Some(Cebada), "la que trabaja la plaza");
    assert!(p.elegir(otra, Uva).aceptada());
    assert_eq!((p.receta(bodega), p.receta(otra)), (Some(Cebada), Some(Uva)));
}

#[test]
fn cada_rechazo_de_elegir_receta_lleva_su_codigo_y_no_deja_rastro() {
    let m = con_mundo!();
    let mut p = Plaza::fundar(m, 4, 2_000_000);
    let (b1, b2, prensa) = (p.levantar(Tipo::Bodega), p.levantar(Tipo::Bodega), p.levantar(Tipo::Prensa));
    let granja = p.n.economia(p.id).unwrap().edificios.iter().find(|e| e.tipo == Tipo::Granja).unwrap().id;
    let rastro = p.n.hash_estado();
    assert_eq!(codigo(&p.elegir(9_999, Uva)), rechazos::EDIFICIO_DESCONOCIDO);
    assert_eq!(codigo(&p.elegir(granja, Trigo)), rechazos::RECETA_NO_SE_ELIGE, "la granja no elige");
    assert_eq!(codigo(&p.elegir(b1, Aceituna)), rechazos::RECETA_DESCONOCIDA, "la bodega no consume aceituna");
    assert_eq!(codigo(&p.elegir(prensa, Uva)), rechazos::RECETA_DESCONOCIDA);
    // Nivel 4: una sola bebida distinta, y una sola fuente de aceite.
    assert_eq!(codigo(&p.elegir(b1, Cebada)), rechazos::RECETA_TOPE_POR_PLAZA, "con dos bodegas de vino, cambiar una a cerveza son dos bebidas");
    assert_eq!(p.n.hash_estado(), rastro, "ningún rechazo toca el estado");
    // Sin permiso en la plaza.
    let t = p.n.ahora();
    let r = p.n.aplicar_en(t, Peticion::nueva("luis", "x", Comando::ElegirReceta { asentamiento: p.id, edificio: b2, insumo: Uva }));
    assert_eq!(codigo(&r), rechazos::NO_AUTORIZADO);
    // Con una sola bodega de vino sí cabe cambiarla: se cancela la segunda y la primera pasa a cerveza.
    assert!(p.manda(Comando::CancelarConstruccion { asentamiento: p.id, edificio: b2 }).aceptada());
    assert!(p.elegir(b1, Cebada).aceptada());
    // La prensa, una sola fuente de aceite.
    let prensa2 = p.levantar(Tipo::Prensa);
    assert_eq!(codigo(&p.elegir(prensa2, Sesamo)), rechazos::RECETA_TOPE_POR_PLAZA, "aceituna y sésamo a la vez, nunca");
    assert_eq!(p.receta(prensa2), Some(Aceituna));
}

#[test]
fn la_plaza_admite_tres_bodegas_y_tres_prensas() {
    let m = con_mundo!();
    let mut p = Plaza::fundar(m, 5, 2_000_000);
    for tipo in [Tipo::Bodega, Tipo::Prensa] {
        for _ in 0..3 {
            p.levantar(tipo);
            // La cola admite pocas obras a la vez: se espera a que acaben.
            let t = p.n.ahora() + 30 * MS_POR_MINUTO;
            p.n.avanzar_hasta(t);
        }
        let r = p.manda(Comando::AnadirEdificio { asentamiento: p.id, tipo });
        assert_eq!(codigo(&r), rechazos::OBRA_TOPE_POR_PLAZA, "{tipo:?}: la cuarta, no");
    }
}

#[test]
fn al_bajar_de_nivel_la_receta_que_sobra_se_proyecta_parada() {
    let m = con_mundo!();
    let mut p = Plaza::fundar(m, 5, 300_000);
    let (vino, cerveza) = (p.levantar(Tipo::Bodega), p.levantar(Tipo::Bodega));
    assert!(p.elegir(cerveza, Cebada).aceptada());
    assert_eq!((p.proyectado(vino), p.proyectado(cerveza)), ((Some("uva".into()), false), (Some("cebada".into()), false)), "en el nivel 5 trabajan las dos");
    // Sin auto-construcción ni más materiales, el mantenimiento acaba por colapsar y baja el nivel operativo de la plaza (el primer día y medio).
    for _ in 0..24 * 12 {
        let t = p.n.ahora() + HORA;
        p.n.avanzar_hasta(t);
        let nivel = p.n.estado().asentamientos.get(&p.id).expect("la plaza no cae en ruinas antes de bajar de nivel").nivel_actual();
        if nivel < 5 {
            break;
        }
    }
    assert_eq!(p.n.estado().asentamientos[&p.id].nivel_actual(), 4);
    assert_eq!((p.proyectado(vino), p.proyectado(cerveza)), ((Some("uva".into()), false), (Some("cebada".into()), true)), "con una sola bebida cabida, para la del mayor id");
    assert_eq!(p.receta(cerveza), Some(Cebada), "parar no cambia la receta elegida");
}
