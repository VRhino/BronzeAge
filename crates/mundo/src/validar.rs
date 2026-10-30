//! Las reglas de 06 Â§3. `validar` consume el artefacto y devuelve el `Mundo` o la primera regla incumplida.

use crate::{sha256_hex, ErrorMundo, Mundo, OpcionesValidacion, MILESIMAS, TAMANO_CELDA};
use contrato::v1::*;
use prost::Message;
use std::collections::{HashMap, HashSet};

type R<T> = Result<T, ErrorMundo>;

/// Reparto del estaÃ±o y el oro por anillo, en %: foco, frontera, lejana (Doc 1.9c).
const REPARTO: [(&str, u32); 3] = [("foco", 5), ("frontera", 25), ("lejana", 70)];

pub(crate) fn validar(mut a: ArtefactoDeMundo, op: &OpcionesValidacion) -> R<Mundo> {
    // 1. Hash del contenido (con `manifiesto.hash` vacÃ­o).
    let m = a.manifiesto.as_mut().ok_or(ErrorMundo::SinManifiesto)?;
    let declarado = std::mem::take(&mut m.hash);
    let obtenido = sha256_hex(&a.encode_to_vec());
    if obtenido != declarado {
        return Err(ErrorMundo::HashDelContenido { esperado: declarado, obtenido });
    }
    a.manifiesto.as_mut().unwrap().hash = declarado;
    a.navegacion.get_or_insert_with(Default::default); // sin grafo = grafo vacÃ­o

    // 2. TamaÃ±o y rejilla.
    let m = a.manifiesto.as_ref().unwrap();
    let t = m.tamano.map_or((0, 0), |p| (p.x, p.y));
    if t.0 <= 0 || t.1 <= 0 {
        return Err(ErrorMundo::TamanoInvalido { obtenido: t });
    }
    if let Some(esperado) = op.tamano_esperado {
        if esperado != t {
            return Err(ErrorMundo::TamanoNoEsperado { esperado, obtenido: t });
        }
    }
    let celda = TAMANO_CELDA as i64;
    let (columnas, filas) = (((t.0 + celda - 1) / celda) as usize, ((t.1 + celda - 1) / celda) as usize);

    // 3. Capas: sha256 declarado y contenido de cada raster.
    let n = columnas * filas;
    let dim = (columnas as u32, filas as u32, TAMANO_CELDA);
    let u8s = (EncodingRaster::EncodingU8, n);
    let capa = |nombre, r: &Option<Raster>, enc: (EncodingRaster, usize)| -> R<Vec<u8>> {
        let r = r.as_ref().ok_or(ErrorMundo::RasterAusente { capa: nombre })?;
        raster(nombre, m, r, dim, enc)
    };
    let altura_bytes = capa("altura", &a.altura, (EncodingRaster::EncodingU16Le, 2 * n))?;
    let terreno = capa("terreno", &a.terreno, u8s)?;
    let fertilidad = capa("fertilidad", &a.fertilidad, u8s)?;
    let humedad = capa("humedad", &a.humedad, u8s)?;
    let densidad_bosque = capa("densidad_bosque", &a.densidad_bosque, u8s)?;
    let anillos = capa("anillos", &a.anillos, u8s)?;
    let transitable = capa("transitable", &a.transitable, (EncodingRaster::EncodingBits, n.div_ceil(8)))?;
    let u16s = |b: Vec<u8>| -> Vec<u16> { b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect() };
    let altura = u16s(altura_bytes);
    // Opcional: sin coste, el servidor no suaviza las rutas. Con él, vale más de 0 justo en las celdas transitables.
    let coste = match &a.coste {
        Some(r) => u16s(raster("coste", m, r, dim, (EncodingRaster::EncodingU16Le, 2 * n))?),
        None => Vec::new(),
    };
    for (i, &c) in coste.iter().enumerate() {
        let pisable = transitable[i >> 3] >> (i & 7) & 1 == 1;
        if (c > 0) != pisable {
            return Err(ErrorMundo::CosteIncoherente { celda: (i % columnas, i / columnas), coste: c, transitable: pisable });
        }
    }

    let mundo = Mundo { artefacto: a, columnas, filas, altura, terreno, fertilidad, humedad, densidad_bosque, anillos, transitable, coste };

    // 4. Vectoriales.
    ids_unicos(&mundo)?;
    grafo(&mundo)?;
    en_tierra(&mundo)?;
    alijos(&mundo)?;
    cuotas_de_metales(&mundo)?;
    parcelas(&mundo, op.rp * 2 * MILESIMAS)?;
    campamentos(&mundo, op.separacion_campamentos * MILESIMAS)?;
    Ok(mundo)
}

/// Comprueba sha256 de la capa contra el manifiesto, dimensiones, encoding y longitud; devuelve los datos descomprimidos.
fn raster(nombre: &'static str, m: &Manifiesto, r: &Raster, dim: (u32, u32, u32), (enc, largo): (EncodingRaster, usize)) -> R<Vec<u8>> {
    let mut declaradas = m.capas.iter().filter(|c| c.nombre == nombre);
    let declarada = declaradas.next().ok_or(ErrorMundo::CapaSinDeclarar { capa: nombre })?;
    if declaradas.next().is_some() {
        return Err(ErrorMundo::CapaRepetida { capa: nombre.into() });
    }
    let obtenido = sha256_hex(&r.datos);
    if obtenido != declarada.sha256 {
        return Err(ErrorMundo::CapaSha256 { capa: nombre, esperado: declarada.sha256.clone(), obtenido });
    }
    let encontrado = (r.columnas, r.filas, r.tamano_celda);
    if encontrado != dim {
        return Err(ErrorMundo::RasterDimensiones { capa: nombre, esperado: dim, obtenido: encontrado });
    }
    if r.encoding != enc as i32 {
        return Err(ErrorMundo::RasterEncoding { capa: nombre, esperado: enc as i32, obtenido: r.encoding });
    }
    let datos = if r.zstd {
        // `largo` acota la salida: un zstd que se expande mÃ¡s no pasa.
        zstd::bulk::decompress(&r.datos, largo).map_err(|e| ErrorMundo::RasterDescompresion { capa: nombre, detalle: e.to_string() })?
    } else {
        r.datos.clone()
    };
    if datos.len() != largo {
        return Err(ErrorMundo::RasterLongitud { capa: nombre, esperado: largo, obtenido: datos.len() });
    }
    Ok(datos)
}

fn unicos<'a>(coleccion: &'static str, ids: impl Iterator<Item = &'a str>) -> R<()> {
    let mut vistos = HashSet::new();
    match ids.into_iter().find(|id| !vistos.insert(*id)) {
        Some(id) => Err(ErrorMundo::IdRepetido { coleccion, id: id.into() }),
        None => Ok(()),
    }
}

fn ids_unicos(m: &Mundo) -> R<()> {
    unicos("rios", m.rios().iter().map(|x| x.id.as_str()))?;
    unicos("bosques", m.bosques().iter().map(|x| x.id.as_str()))?;
    unicos("parcelas", m.parcelas().iter().map(|x| x.id.as_str()))?;
    unicos("nodos", m.nodos().iter().map(|x| x.id.as_str()))?;
    unicos("vetas", m.vetas().iter().map(|x| x.id.as_str()))?;
    unicos("campamentos", m.campamentos().iter().map(|x| x.id.as_str()))?;
    unicos("bandidos_iniciales", m.bandidos_iniciales().iter().map(|x| x.id.as_str()))?;
    unicos("alijos", m.alijos().iter().map(|x| x.id.as_str()))
}

fn grafo(m: &Mundo) -> R<()> {
    let g = m.navegacion();
    let mut pos = HashMap::new();
    for nodo in &g.nodos {
        let p = nodo.posicion.ok_or(ErrorMundo::SinPosicion { coleccion: "navegacion.nodos", id: nodo.id.to_string() })?;
        if pos.insert(nodo.id, (p.x, p.y)).is_some() {
            return Err(ErrorMundo::IdRepetido { coleccion: "navegacion.nodos", id: nodo.id.to_string() });
        }
    }
    for e in &g.aristas {
        for nodo in [e.a, e.b] {
            let &(x, y) = pos.get(&nodo).ok_or(ErrorMundo::AristaConNodoInexistente { a: e.a, b: e.b, nodo })?;
            if !m.transitable_en(x, y) {
                return Err(ErrorMundo::AristaNoTransitable { a: e.a, b: e.b, nodo, posicion: (x, y) });
            }
        }
    }
    Ok(())
}

/// Nodos de recurso, campamentos, bandidos y alijos: dentro del mundo, fuera del agua y en celda transitable.
fn en_tierra(m: &Mundo) -> R<()> {
    let comprobar = |coleccion: &'static str, id: &str, p: &Option<Punto>| -> R<()> {
        let p = p.ok_or_else(|| ErrorMundo::SinPosicion { coleccion, id: id.into() })?;
        let posicion = (p.x, p.y);
        if m.terreno_en(p.x, p.y) == Some(ClaseTerreno::TerrenoAgua) {
            Err(ErrorMundo::EnAgua { coleccion, id: id.into(), posicion })
        } else if !m.transitable_en(p.x, p.y) {
            Err(ErrorMundo::NoTransitable { coleccion, id: id.into(), posicion })
        } else {
            Ok(())
        }
    };
    for x in m.nodos() {
        comprobar("nodos", &x.id, &x.posicion)?;
    }
    for x in m.campamentos() {
        comprobar("campamentos", &x.id, &x.posicion)?;
    }
    for x in m.bandidos_iniciales() {
        comprobar("bandidos_iniciales", &x.id, &x.posicion)?;
    }
    for x in m.alijos() {
        comprobar("alijos", &x.id, &x.posicion)?;
    }
    Ok(())
}

/// Cada alijo trae una sola cosa (Generacion_Del_Mundo §12): oro o un recurso de fundar, en cantidad positiva.
fn alijos(m: &Mundo) -> R<()> {
    match m.alijos().iter().find(|a| !matches!(a.recurso.as_str(), "oro" | "madera" | "piedra" | "trigo") || a.cantidad <= 0) {
        Some(a) => Err(ErrorMundo::AlijoInvalido { id: a.id.clone(), recurso: a.recurso.clone(), cantidad: a.cantidad }),
        None => Ok(()),
    }
}

/// Reparto por resto mayor (el empate lo gana el anillo que va antes: foco, frontera, lejana).
pub(crate) fn cuotas(total: u32) -> [u32; 3] {
    let mut cuota = REPARTO.map(|(_, pct)| total * pct / 100);
    let mut orden = [0, 1, 2];
    orden.sort_by_key(|&i| std::cmp::Reverse(total * REPARTO[i].1 % 100)); // sort estable
    for &i in orden.iter().take((total - cuota.iter().sum::<u32>()) as usize) {
        cuota[i] += 1;
    }
    cuota
}

fn cuotas_de_metales(m: &Mundo) -> R<()> {
    for tipo in ["estano", "oro"] {
        let mut hay = [0u32; 3];
        for n in m.nodos().iter().filter(|n| n.tipo == tipo) {
            let i = REPARTO.iter().position(|(a, _)| Some(*a) == n.anillo.as_deref());
            hay[i.ok_or_else(|| ErrorMundo::AnilloInvalido { tipo: tipo.into(), id: n.id.clone(), anillo: n.anillo.clone() })?] += 1;
        }
        let esperado = cuotas(hay.iter().sum());
        for i in 0..3 {
            if hay[i] != esperado[i] {
                return Err(ErrorMundo::CuotaPorAnillo { tipo, anillo: REPARTO[i].0, esperado: esperado[i], obtenido: hay[i] });
            }
        }
    }
    Ok(())
}

fn distancia(a: Punto, b: Punto) -> i64 {
    let (dx, dy) = ((a.x - b.x) as f64, (a.y - b.y) as f64);
    dx.hypot(dy) as i64
}

/// Parcelas: bosque existente y separaciÃ³n â‰¥ `minima` entre todas (rejilla de lado `minima`: solo se miran las vecinas).
fn parcelas(m: &Mundo, minima: i64) -> R<()> {
    let bosques: HashSet<&str> = m.bosques().iter().map(|b| b.id.as_str()).collect();
    let mut rejilla: HashMap<(i64, i64), Vec<&ParcelaBosque>> = HashMap::new();
    for p in m.parcelas() {
        if !bosques.contains(p.bosque_id.as_str()) {
            return Err(ErrorMundo::ParcelaSinBosque { id: p.id.clone(), bosque_id: p.bosque_id.clone() });
        }
        let c = p.centro.ok_or_else(|| ErrorMundo::SinPosicion { coleccion: "parcelas", id: p.id.clone() })?;
        let (gx, gy) = (c.x.div_euclid(minima), c.y.div_euclid(minima));
        for otra in (gx - 1..=gx + 1).flat_map(|x| (gy - 1..=gy + 1).map(move |y| (x, y))).filter_map(|k| rejilla.get(&k)).flatten() {
            let d = distancia(c, otra.centro.unwrap());
            if d < minima {
                return Err(ErrorMundo::ParcelasCercanas { a: otra.id.clone(), b: p.id.clone(), distancia: d, minima });
            }
        }
        rejilla.entry((gx, gy)).or_default().push(p);
    }
    Ok(())
}

fn campamentos(m: &Mundo, minima: i64) -> R<()> {
    let cs = m.campamentos();
    for (i, b) in cs.iter().enumerate() {
        for a in &cs[..i] {
            let d = distancia(a.posicion.unwrap(), b.posicion.unwrap()); // posiciÃ³n ya comprobada en `en_tierra`
            if d < minima {
                return Err(ErrorMundo::CampamentosCercanos { a: a.id.clone(), b: b.id.clone(), distancia: d, minima });
            }
        }
    }
    Ok(())
}
