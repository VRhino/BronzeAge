//! La proyección de las recompensas de misión (Misiones §5 y Misiones_Recurrentes §8): la fija de cada tipo y la lista, en las misiones únicas (héroe, plaza y Facción) y en el catálogo del
//! tablón de las recurrentes. Sobre el artefacto de mundo real; las pruebas se omiten si no está.

mod comun;
use comun::real::{self, sitio_rico};
use contrato::v1::RecompensaMision;
use base::UNIDAD;
use militar::heroe::Controlador;
use nucleo::misiones::Dueno;
use nucleo::proyeccion::{interior, propios};
use nucleo::recurrentes::Catalogo;
use nucleo::{Ajustes, Arranque, Comando, Nucleo, Peticion, Reglas};
use std::sync::Arc;

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

/// Una recurrente de cada ámbito con su fija y su lista, y una individual que da una tropa.
const CATALOGO: &str = r#"{
  "asentamiento": [
    {"id":"P1","duracion":"dia","tipo":"grupal","condiciones":{"escuadrones":2},"participacion":21,"recompensas":[{"tipo":"recurso","recurso":"piedra","cantidad":4},{"tipo":"oro","cantidad":2}],"titulo":"t","texto":"t","explicacion":"e"},
    {"id":"P2","duracion":"dia","tipo":"individual","limiteSemanal":1,"condiciones":{"escuadrones":1},"participacion":10,"recompensas":[{"tipo":"xp","cantidad":2},{"tipo":"tropa","tropa":"milicia_lanceros","cantidad":5}],"titulo":"t","texto":"t","explicacion":"e"}
  ],
  "faccion": [
    {"id":"FG","duracion":"semana","tipo":"grupal","condiciones":{"escuadrones":2},"xp":7,"recompensas":[{"tipo":"recurso","recurso":"madera","cantidad":3}],"titulo":"t","texto":"t","explicacion":"e"}
  ]
}"#;

fn nucleo(m: &real::M) -> Nucleo {
    let mut reglas = Reglas::default();
    reglas.economia.recurrentes = Arc::new(Catalogo::desde_texto(CATALOGO, &[], &reglas.economia.militar.tropas).unwrap());
    Nucleo::nuevo(Arranque { reglas, ajustes: Ajustes::default(), semilla: 7, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) })
}

fn oro(cantidad: i64) -> RecompensaMision {
    RecompensaMision { tipo: "oro".into(), clave: String::new(), cantidad }
}

fn recurso(clave: &str, cantidad: i64) -> RecompensaMision {
    RecompensaMision { tipo: "recurso".into(), clave: clave.into(), cantidad }
}

#[test]
fn la_cadena_de_heroe_lleva_su_lista_y_ninguna_fija() {
    let m = con_mundo!();
    let mut n = nucleo(m);
    let t = n.ahora();
    let r = n.aplicar_en(t, Peticion::nueva("ana", "h", Comando::CrearHeroe { cuenta: "ana".into(), campamento: 0, controlador: Controlador::Humano, nombre: None, clase: "espada_escudo_mediano".into() }));
    assert!(r.aceptada(), "{r:?}");
    let vistas = propios::misiones(&n, Dueno::Heroe(r.creadas[0]));
    let h1 = vistas.iter().find(|v| v.id == "H1").expect("H1 está activa");
    assert_eq!((h1.participacion, h1.recompensa_experiencia), (0, 0));
    assert_eq!(h1.recompensas, [oro(10 * UNIDAD)]);
}

#[test]
fn la_plaza_y_la_faccion_llevan_su_fija_y_su_lista() {
    let m = con_mundo!();
    let mut n = nucleo(m);
    let p = sitio_rico(&n, &m.0, 0, 1);
    let a = real::fundar(&mut n, "ana", 1, p);
    // Una de plaza: la participación y su recurso; sin experiencia fija.
    let plaza = interior::asentamiento(&n, a).unwrap();
    let a1 = plaza.misiones.iter().find(|v| v.id == "A1").expect("A1 está activa");
    assert_eq!((a1.participacion, a1.recompensa_experiencia), (10, 0));
    assert_eq!(a1.recompensas, [recurso("trigo", 300 * UNIDAD)]);
    // Una de Facción: su experiencia y la lista vacía; sin participación.
    let faccion = propios::faccion(&n, 1);
    let f1 = faccion.misiones.iter().find(|v| v.id == "F1").expect("F1 está activa");
    assert_eq!((f1.participacion, f1.recompensa_experiencia), (0, 10 * UNIDAD));
    assert!(f1.recompensas.is_empty());
}

#[test]
fn el_catalogo_del_tablon_lleva_la_fija_de_su_ambito_y_la_lista_con_la_tropa() {
    let m = con_mundo!();
    let mut n = nucleo(m);
    let p = sitio_rico(&n, &m.0, 0, 1);
    let a = real::fundar(&mut n, "ana", 1, p);
    // De plaza: la participación fija; la grupal con su recurso y su oro, la individual con su xp y su tropa (soldados, no milésimas).
    let catalogo = interior::asentamiento(&n, a).unwrap().tablon.expect("la plaza lleva su tablón").catalogo;
    assert_eq!(catalogo.iter().map(|d| d.id.as_str()).collect::<Vec<_>>(), ["P1", "P2"]);
    assert_eq!((catalogo[0].participacion, catalogo[0].experiencia), (21, 0));
    assert_eq!(catalogo[0].recompensas, [recurso("piedra", 4 * UNIDAD), oro(2 * UNIDAD)]);
    assert_eq!((catalogo[1].participacion, catalogo[1].experiencia), (10, 0));
    assert_eq!(catalogo[1].recompensas, [RecompensaMision { tipo: "xp".into(), clave: String::new(), cantidad: 2 * UNIDAD }, RecompensaMision { tipo: "tropa".into(), clave: "milicia_lanceros".into(), cantidad: 5 }]);
    // De Facción: la experiencia fija y su lista.
    let catalogo = propios::faccion(&n, 1).tablon.expect("la Facción lleva su tablón").catalogo;
    assert_eq!(catalogo.len(), 1);
    assert_eq!((catalogo[0].participacion, catalogo[0].experiencia), (0, 7 * UNIDAD));
    assert_eq!(catalogo[0].recompensas, [recurso("madera", 3 * UNIDAD)]);
}
