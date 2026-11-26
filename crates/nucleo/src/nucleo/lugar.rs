//! El recorte del lugar de una batalla de campo abierto o de caravana (entrada 78; `Docs/Design/Campo_De_Batalla_Del_Lugar.md` de Conquest §3): lo único
//! del mundo que el servidor de batalla y los clientes necesitan para generar el campo. Es lo mismo que hace `recortar` en la mesa del campo de
//! BronzeAgeClient (`pruebas/campo3d/generador.ts`), con las reglas del motor: el campo abierto es siempre terrestre y nunca queda pegado a un río sin
//! vado (autor, 2026-11-23 y 2026-11-26). Las cifras del campo (escala, lados, anillo) son las de la §13 del diseño, que son de Conquest; aquí solo
//! se usan para saber cuánto mundo recortar.

use super::*;
use base::Plan;
use combate::ciclo::{Batalla, Contexto};
use contrato::v1 as pb;
use mundo::{RelieveFino, TeselaFina, TAMANO_CELDA};
use std::collections::HashMap;

/// Metros del campo por unidad del mundo (`escala_campo`).
const ESCALA_CAMPO: i64 = 75;
/// Lado del área jugable de la batalla pequeña y de la grande, y el anillo de fondo alrededor, en metros del campo.
const LADO_PEQUENA: i64 = 600;
const LADO_GRANDE: i64 = 1000;
const ANILLO: i64 = 700;
/// Desde cuántos héroes en un bando la batalla es grande, y desde cuántos carros en la caravana de un defensor sin héroes (`heroes_grande`,
/// `carros_grande`; autor, 2026-11-26). Conquest saca el tamaño del ticket con la misma regla.
const HEROES_GRANDE: usize = 6;
const CARROS_GRANDE: usize = 5;
/// Cuánto se retrocede sobre el plan de un bando para su procedencia (`distancia_procedencia`), en milésimas.
const DISTANCIA_PROCEDENCIA: i64 = 2_000;
/// Lo que el centro queda, como poco, del borde de un río sin vado: la placa de un spawn (25 m) y un margen (15 m), en milésimas del mundo.
const APARTAR_DEL_RIO: i64 = 40 * MILESIMAS / ESCALA_CAMPO;
/// Medio ancho de un río del que el artefacto no trae el ancho (`ancho_rio_mundo`, 1,5 u).
const MEDIO_ANCHO_SIN_DATO: i64 = 750;
const MILESIMAS: i64 = 1_000;

fn metros_a_milesimas(m: i64) -> i64 {
    m * MILESIMAS / ESCALA_CAMPO
}

/// De dónde llegaba lo que se mueve sobre `plan`: el punto a [`DISTANCIA_PROCEDENCIA`] hacia atrás de su posición en `t` (o el primero del plan si anduvo
/// menos). `None` si en `t` no venía de ningún sitio.
pub(super) fn procedencia_en(plan: &Plan, t: Instante) -> Option<Punto> {
    let ahora = plan.posicion_en(t)?;
    let hechos = plan.vertices.partition_point(|v| v.en <= t);
    let (mut actual, mut resto) = (ahora, DISTANCIA_PROCEDENCIA as f64);
    for v in plan.vertices[..hechos].iter().rev() {
        let d = distancia(actual, v.p);
        if d >= resto {
            let f = resto / d;
            return Some(Punto::new(actual.x + ((v.p.x - actual.x) as f64 * f).round() as i64, actual.y + ((v.p.y - actual.y) as f64 * f).round() as i64));
        }
        resto -= d;
        actual = v.p;
    }
    (actual != ahora).then_some(actual)
}

fn distancia(a: Punto, b: Punto) -> f64 {
    ((a.x - b.x) as f64).hypot((a.y - b.y) as f64)
}

/// Las muestras del relieve fino, con las teselas que ya se descomprimieron.
struct Muestras<'a> {
    relieve: &'a RelieveFino,
    teselas: HashMap<(i64, i64), TeselaFina>,
}

impl<'a> Muestras<'a> {
    fn de(relieve: &'a RelieveFino) -> Self {
        Self { relieve, teselas: HashMap::new() }
    }

    /// La altura y el agua del vértice `(i, j)` del paquete.
    fn vertice(&mut self, i: i64, j: i64) -> (u16, bool) {
        let (t, k) = self.relieve.donde(i, j);
        let r = self.relieve;
        let tesela = self.teselas.entry(t).or_insert_with(|| r.tesela(t.0, t.1));
        (tesela.altura(k), tesela.agua(k))
    }

    /// ¿Hay agua en el vértice más cercano a `(x, y)`?
    fn agua_en(&mut self, x: i64, y: i64) -> bool {
        let p = self.relieve.paso();
        self.vertice((x + p / 2).div_euclid(p), (y + p / 2).div_euclid(p)).1
    }

    /// ¿Es agua más de la mitad del cuadrado de `lado` centrado en `c`? Por las muestras de su rejilla, como `fraccionDeAgua` de la mesa.
    fn mas_agua_que_tierra(&mut self, c: Punto, lado: i64) -> bool {
        let (agua, total) = self.agua_del_cuadrado(c, lado);
        2 * agua > total
    }

    fn agua_del_cuadrado(&mut self, c: Punto, lado: i64) -> (i64, i64) {
        let p = self.relieve.paso();
        let n = (lado + p / 2) / p;
        let mut agua = 0;
        for j in 0..=n {
            for i in 0..=n {
                agua += i64::from(self.agua_en(c.x - lado / 2 + i * p, c.y - lado / 2 + j * p));
            }
        }
        (agua, (n + 1) * (n + 1))
    }
}

/// El campo abierto es siempre terrestre (autor, 2026-11-23): con más de la mitad de agua en el área jugable, el menor desplazamiento (en 8 direcciones, a
/// pasos de una muestra, hasta dos lados) que deja más tierra que agua; en esa distancia, la dirección con menos agua.
fn centro_en_tierra(m: &mut Muestras, punto: Punto, lado: i64) -> Punto {
    const DIRECCIONES: [(i64, i64); 8] = [(1000, 0), (707, 707), (0, 1000), (-707, 707), (-1000, 0), (-707, -707), (0, -1000), (707, -707)];
    if !m.mas_agua_que_tierra(punto, lado) {
        return punto;
    }
    let paso = m.relieve.paso();
    let mut d = paso;
    while d <= 2 * lado {
        let mejor = DIRECCIONES
            .iter()
            .map(|&(cx, cy)| Punto::new(punto.x + d * cx / 1000, punto.y + d * cy / 1000))
            .map(|c| (c, m.agua_del_cuadrado(c, lado)))
            .filter(|&(_, (agua, total))| 2 * agua < total)
            .min_by_key(|&(_, (agua, _))| agua);
        if let Some((c, _)) = mejor {
            return c;
        }
        d += paso;
    }
    punto
}

/// ¿Corta el tramo `a`-`b` la caja `[x0, x1] × [y0, y1]`? (Liang-Barsky)
fn tramo_corta_caja(a: Punto, b: Punto, (x0, y0, x1, y1): (i64, i64, i64, i64)) -> bool {
    let (dx, dy) = ((b.x - a.x) as f64, (b.y - a.y) as f64);
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (p, q) in [(-dx, (a.x - x0) as f64), (dx, (x1 - a.x) as f64), (-dy, (a.y - y0) as f64), (dy, (y1 - a.y) as f64)] {
        if p == 0.0 {
            if q < 0.0 {
                return false;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
            if t0 > t1 {
                return false;
            }
        }
    }
    true
}

fn caja(c: Punto, lado: i64, margen: i64) -> (i64, i64, i64, i64) {
    (c.x - lado / 2 - margen, c.y - lado / 2 - margen, c.x + lado / 2 + margen, c.y + lado / 2 + margen)
}

fn dentro(p: &pb::Punto, (x0, y0, x1, y1): (i64, i64, i64, i64)) -> bool {
    p.x >= x0 && p.x <= x1 && p.y >= y0 && p.y <= y1
}

fn de_pb(p: &pb::Punto) -> Punto {
    Punto::new(p.x, p.y)
}

fn a_pb(p: Punto) -> pb::Punto {
    pb::Punto { x: p.x, y: p.y }
}

/// El campo nunca queda pegado a un río sin vado (autor, 2026-11-26): si un río que cruza el área jugable sin vado dentro pasa a menos de
/// [`APARTAR_DEL_RIO`] de su borde del centro, el centro se aparta tierra adentro, del lado en que está, lo justo. Con el centro encima del cauce, hacia el
/// lado del defensor (`punto`).
fn apartar_de_rios(rios: &[pb::Rio], punto: Punto, mut centro: Punto, lado: i64) -> Punto {
    // Apartarse de un tramo puede acercar a otro (un meandro, una trenza): se repite hasta que nada se mueve.
    for _ in 0..PASADAS_RIOS {
        let antes = centro;
        centro = apartar_una_vez(rios, punto, centro, lado);
        if centro == antes {
            break;
        }
    }
    centro
}

const PASADAS_RIOS: usize = 8;

fn apartar_una_vez(rios: &[pb::Rio], punto: Punto, mut centro: Punto, lado: i64) -> Punto {
    for rio in rios {
        let area = caja(centro, lado, 0);
        let cruza = rio.puntos.windows(2).any(|w| tramo_corta_caja(de_pb(&w[0]), de_pb(&w[1]), area));
        if !cruza || rio.vados.iter().any(|v| dentro(v, area)) {
            continue;
        }
        // El punto del río más cercano al centro, el medio ancho allí y la dirección del segmento.
        let mut mejor: Option<(f64, (f64, f64), i64, (f64, f64))> = None;
        for (k, w) in rio.puntos.windows(2).enumerate() {
            let (a, b) = ((w[0].x as f64, w[0].y as f64), (w[1].x as f64, w[1].y as f64));
            let (sx, sy) = (b.0 - a.0, b.1 - a.1);
            let largo2 = sx * sx + sy * sy;
            let f = if largo2 == 0.0 { 0.0 } else { (((centro.x as f64 - a.0) * sx + (centro.y as f64 - a.1) * sy) / largo2).clamp(0.0, 1.0) };
            let q = (a.0 + sx * f, a.1 + sy * f);
            let d = (centro.x as f64 - q.0).hypot(centro.y as f64 - q.1);
            let medio = rio.anchos.get(if f < 0.5 { k } else { k + 1 }).map_or(MEDIO_ANCHO_SIN_DATO, |&a| i64::from(a) / 2);
            if mejor.is_none_or(|(dm, ..)| d < dm) {
                mejor = Some((d, q, medio, (sx, sy)));
            }
        }
        let Some((d, q, medio, (sx, sy))) = mejor else { continue };
        let falta = (medio + APARTAR_DEL_RIO) as f64 - d;
        if falta <= 0.0 {
            continue;
        }
        let (mut ux, mut uy) = (centro.x as f64 - q.0, centro.y as f64 - q.1);
        if ux.hypot(uy) < 1.0 {
            // Encima del cauce: la normal del segmento hacia el defensor (o cualquiera de las dos si también está encima).
            let (nx, ny) = (-sy, sx);
            let lado_del_defensor = (punto.x as f64 - q.0) * nx + (punto.y as f64 - q.1) * ny;
            (ux, uy) = if lado_del_defensor < 0.0 { (-nx, -ny) } else { (nx, ny) };
        }
        let u = ux.hypot(uy);
        if u == 0.0 {
            continue;
        }
        let avance = falta + d.min(0.0);
        centro = Punto::new(centro.x + (ux / u * avance).round() as i64, centro.y + (uy / u * avance).round() as i64);
    }
    centro
}

impl Nucleo {
    /// ¿Es grande la batalla? (§6.1 del diseño, autor, 2026-11-26): algún bando con [`HEROES_GRANDE`] héroes o más, o un defensor sin héroes con una
    /// caravana de [`CARROS_GRANDE`] carros o más. Cuentan los del ticket al abrir.
    fn batalla_grande(b: &Batalla) -> bool {
        let carros: usize = match &b.ticket.contexto {
            Contexto::Caravana { caravanas, .. } => caravanas.iter().map(|k| k.carros).sum(),
            _ => 0,
        };
        let (a, d) = (&b.ticket.atacante, &b.ticket.defensor);
        a.participantes.len() >= HEROES_GRANDE || d.participantes.len() >= HEROES_GRANDE || (d.participantes.is_empty() && carros >= CARROS_GRANDE)
    }

    /// El centro del recorte y el recorte del lugar de una batalla de campo abierto o de caravana. `None` en los demás contextos o si el servidor no tiene
    /// el paquete de relieve: entonces Conquest juega en su campo de siempre.
    // ponytail: se calcula en cada proyección del ticket (unos pocos ms); si pesa, guardarlo por batalla.
    pub(crate) fn lugar_de_batalla(&self, b: &Batalla) -> Option<(Punto, pb::RecorteDelLugar)> {
        if !matches!(b.ticket.contexto, Contexto::CampoAbierto { .. } | Contexto::Caravana { .. }) {
            return None;
        }
        let mundo = self.geografia.as_ref()?.mundo();
        let relieve = mundo.relieve()?;
        let lado_jugable = if Self::batalla_grande(b) { LADO_GRANDE } else { LADO_PEQUENA };
        let (lado_mundo, lado) = (metros_a_milesimas(lado_jugable), metros_a_milesimas(lado_jugable + 2 * ANILLO));
        let mut muestras = Muestras::de(relieve);
        let centro = centro_en_tierra(&mut muestras, b.punto, lado_mundo);
        let centro = apartar_de_rios(mundo.rios(), b.punto, centro, lado_mundo);

        // Las muestras del relieve del recorte, en sus vértices.
        let paso = relieve.paso();
        let (x0, y0) = ((centro.x - lado / 2).div_euclid(paso), (centro.y - lado / 2).div_euclid(paso));
        let n = (lado + paso - 1) / paso + 2;
        // El agua, en 2 bits por muestra (0 tierra, 1 lago, 2 mar): el mar no sale de la altura, porque hay lagos por debajo del nivel del mar.
        let (mut alturas, mut agua) = (Vec::with_capacity((n * n * 2) as usize), vec![0u8; ((n * n + 3) / 4) as usize]);
        for j in 0..n {
            for i in 0..n {
                let (h, a) = muestras.vertice(x0 + i, y0 + j);
                alturas.extend_from_slice(&h.to_le_bytes());
                let valor: u8 = if !a { 0 } else if mundo.mar_en((x0 + i) * paso, (y0 + j) * paso) { 2 } else { 1 };
                let k = (j * n + i) as usize;
                agua[k >> 2] |= valor << (2 * (k & 3));
            }
        }
        let relieve_del_lugar = pb::RelieveDelLugar { origen: Some(pb::Punto { x: x0 * paso, y: y0 * paso }), paso: paso as u32, columnas: n as u32, filas: n as u32, alturas, agua };

        // Las celdas de 4 u que toca el recorte, con el valor de su centro. La clase de terreno no va: es la altura por los umbrales del mundo.
        let c = i64::from(TAMANO_CELDA);
        let (ci0, cj0) = ((centro.x - lado / 2).div_euclid(c), (centro.y - lado / 2).div_euclid(c));
        let (columnas, filas) = ((centro.x + lado / 2).div_euclid(c) - ci0 + 1, (centro.y + lado / 2).div_euclid(c) - cj0 + 1);
        let celdas = |f: &dyn Fn(i64, i64) -> Option<u8>| pb::RejillaDelLugar {
            origen: Some(pb::Punto { x: ci0 * c, y: cj0 * c }),
            paso: c as u32,
            columnas: columnas as u32,
            filas: filas as u32,
            valores: (0..filas).flat_map(|j| (0..columnas).map(move |i| (i, j))).map(|(i, j)| f((ci0 + i) * c + c / 2, (cj0 + j) * c + c / 2).unwrap_or(0)).collect(),
        };
        let (bosque, humedad, fertilidad) = (celdas(&|x, y| mundo.densidad_bosque_en(x, y)), celdas(&|x, y| mundo.humedad_en(x, y)), celdas(&|x, y| mundo.fertilidad_en(x, y)));

        // Los trozos de río que cruzan el recorte (con un punto de más a cada lado) y los vados de dentro.
        let (dentro_del_recorte, ampliada) = (caja(centro, lado, 0), caja(centro, lado, MILESIMAS));
        let rios = mundo
            .rios()
            .iter()
            .filter_map(|rio| {
                let cortan: Vec<usize> = (0..rio.puntos.len().saturating_sub(1)).filter(|&i| tramo_corta_caja(de_pb(&rio.puntos[i]), de_pb(&rio.puntos[i + 1]), ampliada)).collect();
                let (primero, ultimo) = (*cortan.first()?, cortan.last()? + 1);
                let (desde, hasta) = (primero.saturating_sub(1), (ultimo + 2).min(rio.puntos.len()));
                Some(pb::RioDelLugar {
                    puntos: rio.puntos[desde..hasta].to_vec(),
                    vados: rio.vados.iter().filter(|v| dentro(v, dentro_del_recorte)).cloned().collect(),
                    navegable: rio.navegable,
                    anchos: rio.anchos.get(desde..hasta).map(<[u32]>::to_vec).unwrap_or_default(),
                })
            })
            .collect();

        let procedencia = |p: Option<Punto>| pb::ProcedenciaDelLugar { punto: Some(a_pb(p.unwrap_or(b.punto))), quieto: p.is_none() };
        Some((
            centro,
            pb::RecorteDelLugar {
                lado: lado as u32,
                relieve: Some(relieve_del_lugar),
                bosque: Some(bosque),
                humedad: Some(humedad),
                fertilidad: Some(fertilidad),
                rios,
                // ponytail: sin caminos todavía (como la mesa); añadir los tramos de `RedCaminos` que cruzan el recorte.
                caminos: vec![],
                procedencia_atacante: Some(procedencia(b.procedencias[0])),
                procedencia_defensor: Some(procedencia(b.procedencias[1])),
            },
        ))
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use base::Vertice;
    use crate::estado::Reglas;
    use crate::nucleo::{Ajustes, Arranque};
    use std::sync::Arc;

    #[test]
    fn la_procedencia_retrocede_sobre_el_plan_y_un_bando_quieto_no_tiene() {
        let p = |x, y| Punto::new(x, y);
        let plan = Plan::nuevo(0, vec![Vertice { p: p(0, 0), en: 0 }, Vertice { p: p(1_000, 0), en: 10 }, Vertice { p: p(1_000, 5_000), en: 60 }]);
        // En t = 40 va por (1000, 3000): 2 u atrás son 2000 de vuelta por el segundo tramo, hasta (1000, 1000).
        assert_eq!(procedencia_en(&plan, 40), Some(p(1_000, 1_000)));
        // En t = 20 (en (1000, 1000)) ha andado 2 u justas: llega al primer vértice.
        assert_eq!(procedencia_en(&plan, 20), Some(p(0, 0)));
        // En t = 5 ha andado media unidad: el primer vértice.
        assert_eq!(procedencia_en(&plan, 5), Some(p(0, 0)));
        assert_eq!(procedencia_en(&plan, 0), None, "aún no se ha movido");
        assert_eq!(procedencia_en(&Plan::default(), 0), None);
    }

    /// El mundo real con su relieve (`generador/salida/mundo/`), si está; si no, las pruebas sobre él se omiten.
    fn nucleo_del_mundo_real() -> Option<Nucleo> {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../generador/salida/mundo");
        let fichero = |prefijo: &str| std::fs::read_dir(&dir).ok()?.flatten().find(|e| e.file_name().to_string_lossy().starts_with(prefijo)).map(|e| e.path());
        let m = mundo::Mundo::cargar(&std::fs::read(fichero("mundo-")?).ok()?, &mundo::OpcionesValidacion::default()).expect("artefacto válido");
        m.poner_relieve(RelieveFino::desde(&std::fs::read(fichero("relieve-")?).ok()?).expect("relieve válido"));
        let rutas = Arc::new(rutas::Rutas::desde_mundo(&m));
        Some(Nucleo::nuevo(Arranque { reglas: Reglas::default(), ajustes: Ajustes::default(), semilla: 1, inicio: 0, rutas, mundo_hash: m.hash().into(), mundo: Some(Arc::new(m)) }))
    }

    /// Una batalla de campo abierto en `punto` con `heroes` héroes atacantes (6 o más, grande).
    fn batalla(n: &Nucleo, punto: Punto, heroes: u64) -> Batalla {
        let cr = &n.estado.reglas.economia.combate;
        let participante = |heroe| combate::ciclo::Participante { heroe, faccion: None, controlador: militar::heroe::Controlador::Humano, escuadras: vec![] };
        let atacante = combate::ciclo::BandoDeTicket::nuevo(None, (1..=heroes).map(participante).collect(), vec![]);
        let ticket = combate::ciclo::Ticket::nuevo(cr, Contexto::CampoAbierto { punto, solitarios: heroes < 6 }, atacante, combate::ciclo::BandoDeTicket::nuevo(None, vec![], vec![]));
        Batalla::abrir(cr, 1, 1, punto, ticket, Default::default(), 0)
    }

    #[test]
    fn en_el_mundo_real_el_recorte_mide_lo_que_pide_el_tamano_lleva_el_vado_sale_del_mar_y_se_aparta_de_un_rio_sin_vado() {
        let Some(n) = nucleo_del_mundo_real() else {
            eprintln!("sin artefacto de mundo real: se omite");
            return;
        };
        let mundo = n.geografia.as_ref().unwrap().mundo();
        let paso = mundo.relieve().unwrap().paso();

        // Junto a un vado (que no esté junto al agua quieta): el centro no se mueve y el vado va en el recorte.
        let vado = mundo.rios().iter().flat_map(|r| r.vados.iter()).map(de_pb).find(|v| !Muestras::de(mundo.relieve().unwrap()).mas_agua_que_tierra(*v, metros_a_milesimas(LADO_PEQUENA))).expect("un vado en tierra");
        let (centro, r) = n.lugar_de_batalla(&batalla(&n, vado, 1)).expect("con relieve hay recorte");
        assert_eq!(centro, vado);
        assert_eq!(r.lado as i64, metros_a_milesimas(LADO_PEQUENA + 2 * ANILLO));
        let rel = r.relieve.as_ref().unwrap();
        let lado_en_muestras = (r.lado as i64 + paso - 1) / paso + 2;
        assert_eq!((rel.columnas as i64, rel.filas as i64, rel.alturas.len() as i64, rel.agua.len() as i64), (lado_en_muestras, lado_en_muestras, 2 * lado_en_muestras * lado_en_muestras, (lado_en_muestras * lado_en_muestras + 3) / 4));
        assert!(r.rios.iter().any(|rio| rio.vados.iter().any(|v| de_pb(v) == vado)), "el vado va en su río");
        assert!(r.procedencia_atacante.as_ref().unwrap().quieto, "sin plan, quietos");
        let bosque = r.bosque.as_ref().unwrap();
        assert_eq!(bosque.valores.len(), (bosque.columnas * bosque.filas) as usize);
        let bytes = prost::Message::encoded_len(&r);
        assert!(bytes < 9_000, "la pequeña ocupa unos 7 KB, no {bytes}");

        // La grande lee más mundo.
        let (_, grande) = n.lugar_de_batalla(&batalla(&n, vado, 6)).unwrap();
        assert_eq!(grande.lado as i64, metros_a_milesimas(LADO_GRANDE + 2 * ANILLO));

        // En el agua, junto a la costa: el centro se va a tierra hasta que el área jugable tenga más tierra que agua.
        let lado = metros_a_milesimas(LADO_PEQUENA);
        let mut m = Muestras::de(mundo.relieve().unwrap());
        let (ancho, alto) = mundo.tamano();
        let en_el_agua = (1..40)
            .flat_map(|j| (1..40).map(move |i| Punto::new(ancho * i / 40, alto * j / 40)))
            .find(|&p| m.agua_en(p.x, p.y) && m.mas_agua_que_tierra(p, lado) && centro_en_tierra(&mut m, p, lado) != p)
            .expect("un punto de agua junto a tierra");
        let (centro, r) = n.lugar_de_batalla(&batalla(&n, en_el_agua, 1)).unwrap();
        assert_ne!(centro, en_el_agua);
        assert!(!m.mas_agua_que_tierra(centro, lado), "más tierra que agua tras apartarse");
        assert!(r.relieve.unwrap().agua.iter().any(|&a| a > 0), "lleva el agua de la que se apartó");

        // Pegado a un río sin vado cerca: el centro se aparta del cauce lo que mide medio río más la placa con su margen.
        let sin_vado = mundo
            .rios()
            .iter()
            .filter(|rio| rio.puntos.len() > 20 && rio.anchos.len() == rio.puntos.len())
            .find_map(|rio| {
                let k = rio.puntos.len() / 2;
                let (a, c) = (de_pb(&rio.puntos[k]), de_pb(&rio.puntos[k + 1]));
                let lejos_de_vados = rio.vados.iter().chain(mundo.rios().iter().flat_map(|r| r.vados.iter())).all(|v| distancia(de_pb(v), a) > 3.0 * lado as f64);
                let (sx, sy) = ((c.x - a.x) as f64, (c.y - a.y) as f64);
                let l = sx.hypot(sy);
                let junto = Punto::new(a.x + (-sy / l * 200.0) as i64, a.y + (sx / l * 200.0) as i64);
                (lejos_de_vados && !m.mas_agua_que_tierra(junto, lado)).then_some((junto, i64::from(rio.anchos[k]) / 2))
            })
            .expect("un río largo sin vados cerca");
        let (centro, _) = n.lugar_de_batalla(&batalla(&n, sin_vado.0, 1)).unwrap();
        let al_rio = mundo.rios().iter().flat_map(|rio| rio.puntos.windows(2)).map(|w| {
            let (a, c) = (de_pb(&w[0]), de_pb(&w[1]));
            let (sx, sy) = ((c.x - a.x) as f64, (c.y - a.y) as f64);
            let f = (((centro.x - a.x) as f64 * sx + (centro.y - a.y) as f64 * sy) / (sx * sx + sy * sy).max(1.0)).clamp(0.0, 1.0);
            (centro.x as f64 - a.x as f64 - sx * f).hypot(centro.y as f64 - a.y as f64 - sy * f)
        });
        let cerca = al_rio.fold(f64::MAX, f64::min);
        assert!(cerca + 2.0 >= (sin_vado.1 + APARTAR_DEL_RIO) as f64, "a {cerca} del río, con medio ancho {} y {APARTAR_DEL_RIO} de placa", sin_vado.1);
        eprintln!("pegado al río: {:?} → {:?}", sin_vado.0, centro);
    }

    #[test]
    fn el_centro_se_aparta_de_un_rio_sin_vado_y_no_de_uno_con_vado() {
        let rio = |vados: Vec<pb::Punto>| pb::Rio { puntos: vec![pb::Punto { x: -10_000, y: 0 }, pb::Punto { x: 10_000, y: 0 }], anchos: vec![1_000, 1_000], vados, ..Default::default() };
        let defensor = Punto::new(0, 200);
        let apartado = apartar_de_rios(&[rio(vec![])], defensor, defensor, 8_000);
        assert_eq!(apartado, Punto::new(0, 500 + APARTAR_DEL_RIO), "medio ancho y la placa con su margen, del lado del defensor");
        assert_eq!(apartar_de_rios(&[rio(vec![pb::Punto { x: 1_000, y: 0 }])], defensor, defensor, 8_000), defensor, "con un vado en el área jugable no se aparta");
        let lejos = Punto::new(0, 3_000);
        assert_eq!(apartar_de_rios(&[rio(vec![])], lejos, lejos, 8_000), lejos);
        let encima = Punto::new(0, 0);
        assert_eq!(apartar_de_rios(&[rio(vec![])], encima, encima, 8_000).x, 0, "encima del cauce: por la normal");
    }
}

