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
        let hay: Vec<String> = std::fs::read_dir(&dir).ok()?.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).filter(|f| f.starts_with("mundo-")).collect();
        if hay.is_empty() {
            return None;
        }
        // Solo el mundo acordado (2026-11-26): con otro, las pruebas sobre el mundo real dirían cosas distintas en cada máquina.
        let bueno = format!("mundo-{}.bin", hash_acordado());
        assert!(hay.contains(&bueno), "el artefacto de mundo no es el acordado ({bueno}, generador/mundo-acordado.json): hay {hay:?}. Regenéralo con la semilla acordada; si sale otro hash, esta máquina no da el mismo mundo");
        let m = Mundo::cargar(&std::fs::read(dir.join(&bueno)).ok()?, &OpcionesValidacion::default()).expect("artefacto válido");
        let r = Arc::new(Rutas::desde_mundo(&m));
        Some((Arc::new(m), r))
    })
    .as_ref()
}

/// El hash del mundo acordado (`generador/mundo-acordado.json`).
fn hash_acordado() -> String {
    let texto = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../generador/mundo-acordado.json")).expect("generador/mundo-acordado.json");
    let tras = &texto[texto.find("\"hash\"").expect("el acordado lleva «hash»") + 6..];
    let inicio = tras.find('"').expect("hash entre comillas") + 1;
    let fin = inicio + tras[inicio..].find('"').expect("hash entre comillas");
    tras[inicio..fin].to_string()
}

/// La puerta del campamento de mercenarios `i`.
pub fn campamento(m: &Mundo, i: usize) -> Punto {
    let p = m.campamentos()[i].posicion.unwrap();
    Punto::new(p.x, p.y)
}

/// Un sitio verde con mucho bosque a 50 u o más del campamento `i` donde la Facción `faccion` puede fundar ahora: una plaza que no cae en ruinas por falta de madera.
/// Con campo abierto al este ([`abierto_al_este`]): las pruebas se alejan de la plaza hacia +x. «Mucho bosque» son 3 parcelas en la zona del techo: con el bosque en
/// manchas (2026-11-01) una plaza sobrevive con 1, va bien con 2 y va sobrada con 3; 4, todo su espacio, es raro.
pub fn sitio_rico(n: &Nucleo, m: &Mundo, i: usize, faccion: u32) -> Punto {
    sitio_con_parcelas(n, m, i, faccion, 3)
}

/// Como [`sitio_rico`], con al menos `parcelas` parcelas libres en la zona del techo del nivel 1.
#[allow(dead_code)]
pub fn sitio_con_parcelas(n: &Nucleo, m: &Mundo, i: usize, faccion: u32, parcelas: usize) -> Punto {
    let c = campamento(m, i);
    for r in (0..12).map(|k| 50_000 + k * 10_000) {
        for g in 0..360 {
            let a = (g as f64).to_radians();
            let p = Punto::new(c.x + (r as f64 * a.cos()) as i64, c.y + (r as f64 * a.sin()) as i64);
            if n.viabilidad(p, faccion).is_some_and(|v| v.motivo.is_none() && v.color == territorio::Viabilidad::Verde && v.parcelas_en_techo >= parcelas) && abierto_al_este(m, p) {
                return p;
            }
        }
    }
    panic!("sin sitio rico cerca del campamento {i}");
}

/// ¿Hay tierra fuera del bosque cerrado a 30-80 u al este de `p` (a menos de 4 u de cada punto pedido) y lejos de todo campamento de mercenarios (20 u)? Con el
/// relieve a escala de pantalla (2026-10-30) muchos sitios tienen al lado una sierra o un bosque cerrado, y [`en_tierra`] devolvería un punto junto a la plaza.
pub fn abierto_al_este(m: &Mundo, p: Punto) -> bool {
    let campamentos: Vec<Punto> = (0..m.campamentos().len()).map(|i| campamento(m, i)).collect();
    [30_000, 40_000, 45_000, 60_000, 80_000].into_iter().all(|d| {
        let q = en_tierra(m, p, d, 0);
        q.dist(Punto::new(p.x + d, p.y)) <= 4_000 && campamentos.iter().all(|c| c.dist(q) > 20_000)
    })
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

/// El primer punto donde la Facción `faccion` puede fundar ahora, buscando en anillos de 2 u alrededor de `centro + (dx, dy)` (hasta 40 u): con la orilla
/// de 15 u, la plaza a 12 u de todo nodo y a 60 de toda plaza y todo campamento (2026-11-23), un punto suelto rara vez vale.
#[allow(dead_code)]
pub fn fundable_cerca(n: &Nucleo, centro: Punto, dx: Mil, dy: Mil, faccion: u32) -> Punto {
    let c = Punto::new(centro.x + dx, centro.y + dy);
    (0..=20)
        .flat_map(|k| (0..if k == 0 { 1 } else { 36 }).map(move |g| (k * 2_000, ((g * 10) as f64).to_radians())))
        .map(|(r, a)| Punto::new(c.x + (r as f64 * a.cos()) as i64, c.y + (r as f64 * a.sin()) as i64))
        .find(|&p| n.viabilidad(p, faccion).is_some_and(|v| v.motivo.is_none()))
        .expect("hay sitio fundable cerca")
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

/// La distancia de `p` al trazo del río más cercano, en milésimas (`ORILLA_RIO`, 2026-11-18), y si ese río es navegable.
#[allow(dead_code)]
pub fn distancia_al_rio(m: &Mundo, p: Punto) -> (f64, bool) {
    let mut mejor = (f64::MAX, false);
    for r in m.rios() {
        for w in r.puntos.windows(2) {
            let (ax, ay, bx, by) = (w[0].x as f64, w[0].y as f64, w[1].x as f64, w[1].y as f64);
            let (dx, dy) = (bx - ax, by - ay);
            let l = dx * dx + dy * dy;
            let t = if l == 0.0 { 0.0 } else { (((p.x as f64 - ax) * dx + (p.y as f64 - ay) * dy) / l).clamp(0.0, 1.0) };
            let d = (p.x as f64 - ax - t * dx).hypot(p.y as f64 - ay - t * dy);
            if d < mejor.0 {
                mejor = (d, r.navegable);
            }
        }
    }
    mejor
}

/// Los nodos de recurso en una celda transitable que quedan a menos de la media anchura del cauce más `radio` del trazo de un río (`ORILLA_RIO`): donde algo de
/// ese radio quedaría dentro del cauce. Índices en `m.nodos()`. Con los ríos de cauce fino (generador, 2026-11-25) ningún nodo cae ya encima del agua dibujada,
/// pero sí a un par de unidades del trazo.
#[allow(dead_code)]
pub fn nodos_junto_al_rio(m: &Mundo, orilla: &asentamiento::datos::OrillaRio, radio: Mil) -> Vec<usize> {
    m.nodos()
        .iter()
        .enumerate()
        .filter(|(_, n)| {
            let p = n.posicion.unwrap();
            let (d, navegable) = distancia_al_rio(m, Punto::new(p.x, p.y));
            m.transitable_en(p.x, p.y) && d < (if navegable { orilla.media_anchura_navegable } else { orilla.media_anchura } + radio) as f64
        })
        .map(|(i, _)| i)
        .collect()
}
