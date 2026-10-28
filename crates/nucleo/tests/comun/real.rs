//! El artefacto de mundo real (`generador/salida/mundo/mundo-*.bin`) y lo que hace falta para montar escenarios sobre él. Las pruebas que lo usan se omiten si no está.

use base::{Mil, Punto};
use mundo::{Mundo, OpcionesValidacion};
use nucleo::{Comando, Nucleo, Peticion};
use rutas::Rutas;
use std::sync::{Arc, OnceLock};

pub type M = (Arc<Mundo>, Arc<Rutas>);

/// El mundo real y sus rutas, cargados una vez por binario de pruebas (`None` si el artefacto no está).
pub fn mundo() -> Option<&'static M> {
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

/// La puerta del campamento de mercenarios `i`.
pub fn campamento(m: &Mundo, i: usize) -> Punto {
    let p = m.campamentos()[i].posicion.unwrap();
    Punto::new(p.x, p.y)
}

/// Un sitio verde con mucho bosque a 50 u o más del campamento `i` donde la Facción `faccion` puede fundar ahora: una plaza que no cae en ruinas por falta de madera.
pub fn sitio_rico(n: &Nucleo, m: &Mundo, i: usize, faccion: u32) -> Punto {
    let c = campamento(m, i);
    for r in (0..12).map(|k| 50_000 + k * 10_000) {
        for g in 0..360 {
            let a = (g as f64).to_radians();
            let p = Punto::new(c.x + (r as f64 * a.cos()) as i64, c.y + (r as f64 * a.sin()) as i64);
            if n.viabilidad(p, faccion).is_some_and(|v| v.motivo.is_none() && v.color == territorio::Viabilidad::Verde && v.parcelas_en_techo >= 6) {
                return p;
            }
        }
    }
    panic!("sin sitio rico cerca del campamento {i}");
}

/// Un punto en tierra junto a `centro + (dx, dy)`, fuera del bosque cerrado (desde el 2026-10-28 en él no se funda: `fundacion.bosque_denso`).
pub fn en_tierra(m: &Mundo, centro: Punto, dx: Mil, dy: Mil) -> Punto {
    cerca_de(centro, dx, dy, |q| m.transitable_en(q.x, q.y) && m.densidad_bosque_en(q.x, q.y).is_some_and(|d| d < 179))
}

/// Un punto pisable junto a `centro + (dx, dy)`, aunque sea bosque cerrado: para acercarse a algo (un campamento de bandidos puede estar dentro del bosque).
pub fn pisable(m: &Mundo, centro: Punto, dx: Mil, dy: Mil) -> Punto {
    cerca_de(centro, dx, dy, |q| m.transitable_en(q.x, q.y))
}

fn cerca_de(centro: Punto, dx: Mil, dy: Mil, vale: impl Fn(&Punto) -> bool) -> Punto {
    let p = Punto::new(centro.x + dx, centro.y + dy);
    (0..100i64)
        .flat_map(|r| (0..8i64).map(move |k| (r, k)))
        .map(|(r, k)| {
            let a = (k as f64 * 45.0).to_radians();
            Punto::new(p.x + (r as f64 * 2_000.0 * a.cos()) as i64, p.y + (r as f64 * 2_000.0 * a.sin()) as i64)
        })
        .find(|q| vale(q))
        .expect("hay tierra cerca")
}

/// Funda en `p` una plaza de `faccion` con una columna de administración de la cuenta `dueno`; devuelve su id.
pub fn fundar(n: &mut Nucleo, dueno: &str, faccion: u32, p: Punto) -> u64 {
    let t = n.ahora();
    let e = n.crear_ejercito(t, dueno, faccion, p, 500, 100_000).unwrap();
    assert!(n.aplicar_en(t, Peticion::nueva("admin", &format!("caravana-{dueno}"), Comando::DarCaravanaFundacion { ejercito: e }).de_admin()).aceptada());
    let r = n.aplicar_en(t, Peticion::nueva(dueno, &format!("fundar-{dueno}"), Comando::Fundar { ejercito: e }));
    assert!(r.aceptada(), "{r:?}");
    r.creadas[0]
}
