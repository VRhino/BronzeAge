//! El consumo mezclado de bienes en el núcleo (tanda 69 §9): la guarnición come primero los bienes de `BIENES_DE_CONSUMO.guarnicion` (la carne) y después
//! trigo hasta su ración, y lo que come cuenta en la tendencia «+N/min» de cada recurso, como el trigo. Sobre el artefacto de mundo real
//! (`generador/salida/mundo/mundo-*.bin`); las pruebas se omiten si no está.

mod comun;
use asentamiento::Recurso::{self, *};
use base::{Mil, MS_POR_MINUTO};
use comun::real::{fundar, mundo, sitio_rico, M};
use militar::catalogo::Tropa;
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use nucleo::{Ajustes, Arranque, Comando, Nucleo, Peticion, Reglas};

const MIN: i64 = MS_POR_MINUTO;
const HORA: i64 = 60 * MIN;

/// Ración de trigo por soldado y minuto de la prueba (la de las pruebas de guarnición de `heroes.rs`): con la de serie la Granja llenaba el almacén.
const RACION: Mil = 4_000;

/// Una plaza de «ana» con 55 soldados en guarnición (la milicia y los leñadores que reclutó su héroe), un día después de fundarse.
fn plaza_con_guarnicion(m: &M) -> (Nucleo, u64) {
    let mut reglas = Reglas::default();
    // Sin acoso ni anillo de bandidos: esto mide otra cosa.
    let b = &mut std::sync::Arc::make_mut(&mut reglas.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    std::sync::Arc::make_mut(&mut reglas.economia.militar).militar.racion_por_soldado = RACION;
    let mut n = Nucleo::nuevo(Arranque { reglas, ajustes: Ajustes::default(), semilla: 7, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) });
    let p = sitio_rico(&n, &m.0, 0, 1);
    let a = fundar(&mut n, "ana", 1, p);
    n.avanzar_hasta(24 * HORA);
    let mut cuenta = 0;
    let mut orden = |n: &mut Nucleo, c: Comando| {
        cuenta += 1;
        let t = n.ahora();
        let r = n.aplicar_en(t, Peticion::nueva("ana", &format!("o{cuenta}"), c).de_admin());
        assert!(r.aceptada(), "{r:?}");
        r
    };
    orden(&mut n, Comando::AlternarAutoConstruccion { asentamiento: a, pausada: true });
    let h = orden(&mut n, Comando::CrearHeroe { cuenta: "ana".into(), campamento: 0, controlador: Controlador::Humano, nombre: None }).creadas[0];
    orden(&mut n, Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(1) });
    orden(&mut n, Comando::FijarResidencia { heroe: h, residencia: Some(Residencia::Asentamiento(a as u32)) });
    orden(&mut n, Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Asentamiento(a as u32) });
    orden(&mut n, Comando::ReclutarTropa { asentamiento: a, heroe: h, tropa: Tropa::MiliciaLanceros });
    orden(&mut n, Comando::ReclutarTropa { asentamiento: a, heroe: h, tropa: Tropa::Lenadores });
    (n, a)
}

fn dar(n: &mut Nucleo, plaza: u64, recurso: Recurso, cantidad: Mil) {
    let t = n.ahora();
    let r = n.aplicar_en(t, Peticion::nueva("admin", &format!("dar-{recurso:?}"), Comando::DarRecursos { asentamiento: plaza, recurso, cantidad }).de_admin());
    assert!(r.aceptada(), "{r:?}");
}

#[test]
fn la_guarnicion_come_primero_carne_y_despues_trigo_y_la_tendencia_lo_refleja() {
    let Some(m) = mundo() else {
        eprintln!("sin artefacto de mundo real: se omite");
        return;
    };
    // Dos mundos idénticos hasta que a uno se le da carne (1 000 unidades: para varios minutos de guarnición).
    let ((mut sin, a), (mut con, _)) = (plaza_con_guarnicion(m), plaza_con_guarnicion(m));
    dar(&mut con, a, Carne, 1_000_000);
    let soldados = con.soldados_del_campamento(a);
    assert_eq!(soldados, 55);
    let racion = soldados * RACION;
    // La población, que come antes, también toma carne (todas las clases la comen): su parte es sus habitantes por la ración del bloque BIENES_DE_CONSUMO.
    let reglas = Reglas::default();
    let racion_de_carne = reglas.economia.datos.poblacion.bienes.iter().find(|b| b.recurso == Carne).expect("la carne está en BIENES_DE_CONSUMO").racion;
    let habitantes = con.economia(a).unwrap().poblacion.total();
    let (t, carne_antes) = (con.ahora(), con.economia(a).unwrap().almacen.disponible(Carne));
    sin.avanzar_hasta(t + MIN);
    con.avanzar_hasta(t + MIN);
    let (ts, tc) = (sin.economia(a).unwrap().tendencia, con.economia(a).unwrap().tendencia);
    let comida_de_carne = tc.consumido[Carne.indice()];
    // La guarnición come su ración entera de carne (no trigo) y la población lo suyo, con el factor de consumo de la política vigente (el de serie: 1).
    assert_eq!(comida_de_carne, racion + habitantes * racion_de_carne / 1000, "la guarnición toma su ración entera de carne, y la población la suya");
    assert_eq!(carne_antes - con.economia(a).unwrap().almacen.disponible(Carne), comida_de_carne, "y sale del almacén");
    assert_eq!(tc.almacen[Carne.indice()], -comida_de_carne, "la tendencia de la carne es lo que se come");
    // Lo que se come de carne no se come de trigo, uno a uno: la guarnición y la población.
    assert_eq!(ts.consumido[Trigo.indice()] - tc.consumido[Trigo.indice()], comida_de_carne);
    assert_eq!(tc.almacen[Trigo.indice()] - ts.almacen[Trigo.indice()], comida_de_carne, "y la tendencia del trigo sube lo mismo");
    // Cuando la carne se acaba, la guarnición sigue comiendo su ración, ya de trigo: no pasa hambre.
    con.avanzar_hasta(t + 40 * MIN);
    let e = con.economia(a).unwrap();
    assert_eq!(e.almacen.disponible(Carne), 0, "se comió toda");
    assert_eq!(e.tendencia.consumido[Carne.indice()], 0);
    assert!(e.tendencia.consumido[Trigo.indice()] >= racion, "sin carne, el trigo pone la ración de la guarnición");
}
