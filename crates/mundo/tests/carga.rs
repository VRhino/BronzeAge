//! Un mundo sintético pequeño que carga bien y un caso negativo por regla.
//! Mundo de 1.200 × 800 estratégicas = 300 × 200 celdas. Dos campamentos a 600; un lago y un peñasco (no transitable).

use contrato::v1::*;
use mundo::{sha256_hex, ErrorMundo, Mundo, OpcionesValidacion, TAMANO_CELDA};
use prost::Message;

const ANCHO: i64 = 1_200_000;
const ALTO: i64 = 800_000;
const COLS: usize = (ANCHO / TAMANO_CELDA as i64) as usize;
const FILAS: usize = (ALTO / TAMANO_CELDA as i64) as usize;
const CAMPAMENTOS: [(i64, i64); 2] = [(200_000, 200_000), (800_000, 200_000)];
/// Lago (agua) y peñasco (montaña, no transitable): rectángulos [x0, x1) × [y0, y1).
const LAGO: (i64, i64, i64, i64) = (100_000, 140_000, 500_000, 540_000);
const PENASCO: (i64, i64, i64, i64) = (300_000, 340_000, 500_000, 540_000);
const TIERRA: (i64, i64) = (500_000, 100_000);

fn punto(x: i64, y: i64) -> Option<Punto> {
    Some(Punto { x, y })
}
fn en(r: (i64, i64, i64, i64), x: i64, y: i64) -> bool {
    (r.0..r.1).contains(&x) && (r.2..r.3).contains(&y)
}
fn dist(a: (i64, i64), x: i64, y: i64) -> f64 {
    ((a.0 - x) as f64).hypot((a.1 - y) as f64)
}

fn raster(encoding: EncodingRaster, crudo: &[u8]) -> Raster {
    Raster {
        columnas: COLS as u32,
        filas: FILAS as u32,
        tamano_celda: TAMANO_CELDA,
        encoding: encoding as i32,
        datos: zstd::bulk::compress(crudo, 3).unwrap(),
        zstd: true,
    }
}

/// Recalcula el sha256 de cada capa y el hash del artefacto (los tests alteran algo y vuelven a sellar).
fn sellar(a: &mut ArtefactoDeMundo) -> Vec<u8> {
    let capas = [
        ("altura", &a.altura), ("terreno", &a.terreno), ("fertilidad", &a.fertilidad), ("humedad", &a.humedad),
        ("densidad_bosque", &a.densidad_bosque), ("anillos", &a.anillos), ("transitable", &a.transitable),
    ];
    let declaradas = capas
        .iter()
        .filter_map(|(n, r)| {
            let datos = &r.as_ref()?.datos;
            Some(CapaDeMundo { nombre: n.to_string(), archivo: String::new(), sha256: sha256_hex(datos), bytes: datos.len() as u64 })
        })
        .collect();
    a.manifiesto.as_mut().unwrap().capas = declaradas;
    sellar_hash(a);
    a.encode_to_vec()
}

/// Solo el hash del contenido (las capas declaradas se dejan como están).
fn sellar_hash(a: &mut ArtefactoDeMundo) {
    a.manifiesto.as_mut().unwrap().hash = String::new();
    let h = sha256_hex(&a.encode_to_vec());
    a.manifiesto.as_mut().unwrap().hash = h;
}

fn nodo(id: &str, tipo: &str, anillo: Option<&str>, x: i64, y: i64) -> NodoRecurso {
    NodoRecurso { id: id.into(), tipo: tipo.into(), posicion: punto(x, y), anillo: anillo.map(Into::into), ..Default::default() }
}

/// Artefacto válido, SIN sellar.
fn base() -> ArtefactoDeMundo {
    let mut terreno = vec![ClaseTerreno::TerrenoLlano as u8; COLS * FILAS];
    let mut anillos = vec![0u8; COLS * FILAS];
    let mut transitable = vec![0u8; (COLS * FILAS).div_ceil(8)];
    for i in 0..COLS * FILAS {
        let (x, y) = ((i % COLS) as i64 * 4000 + 2000, (i / COLS) as i64 * 4000 + 2000); // centro de la celda
        if en(LAGO, x, y) {
            terreno[i] = ClaseTerreno::TerrenoAgua as u8;
        } else if en(PENASCO, x, y) {
            terreno[i] = ClaseTerreno::TerrenoMontana as u8;
        } else {
            transitable[i >> 3] |= 1 << (i & 7);
        }
        let d = CAMPAMENTOS.iter().map(|&c| dist(c, x, y)).fold(f64::MAX, f64::min);
        anillos[i] = if d <= 150_000.0 { Anillo::Foco } else if d <= 250_000.0 { Anillo::Frontera } else { Anillo::Lejana } as u8;
    }
    let u8s = vec![128u8; COLS * FILAS];
    let alturas: Vec<u8> = (0..COLS * FILAS).flat_map(|i| (i as u16).to_le_bytes()).collect();

    // Estaño: 20 → 1 / 5 / 14. Oro: 10 → empate de restos entre foco y frontera, gana foco: 1 / 2 / 7.
    let mut nodos = vec![nodo("ganado-0", "ganaderia", None, TIERRA.0, TIERRA.1)];
    for (tipo, reparto) in [("estano", [1, 5, 14]), ("oro", [1, 2, 7])] {
        for (anillo, n) in ["foco", "frontera", "lejana"].into_iter().zip(reparto) {
            for k in 0..n {
                nodos.push(nodo(&format!("{tipo}-{anillo}-{k}"), tipo, Some(anillo), 600_000 + 8_000 * k, 600_000));
            }
        }
    }
    let parcelas = (0..9)
        .map(|k| ParcelaBosque {
            id: format!("p{k}"),
            bosque_id: "b1".into(),
            centro: punto(600_000 + 16_000 * (k % 3), 300_000 + 16_000 * (k / 3)), // separación exacta 2·rp
            densidad_milesimas: 800,
            anillo: None,
        })
        .collect();
    let nav = GrafoNavegacion {
        nodos: vec![
            NodoGrafo { id: 1, posicion: punto(500_000, 100_000) },
            NodoGrafo { id: 2, posicion: punto(508_000, 100_000) },
            NodoGrafo { id: 3, posicion: punto(TIERRA.0, TIERRA.1) },
            NodoGrafo { id: 4, posicion: punto(LAGO.0 + 8_000, LAGO.2 + 8_000) }, // en el lago: sin aristas
        ],
        aristas: vec![
            AristaGrafo { a: 1, b: 2, coste_milesimas: 1000, vado: false },
            AristaGrafo { a: 2, b: 3, coste_milesimas: 1000, vado: false },
        ],
        espaciado_min: 8000,
    };
    ArtefactoDeMundo {
        manifiesto: Some(Manifiesto {
            version_formato: 1,
            generador: "prueba".into(),
            tamano: punto(ANCHO, ALTO),
            factor_unity: 12,
            ..Default::default()
        }),
        altura: Some(raster(EncodingRaster::EncodingU16Le, &alturas)),
        terreno: Some(raster(EncodingRaster::EncodingU8, &terreno)),
        fertilidad: Some(raster(EncodingRaster::EncodingU8, &u8s)),
        humedad: Some(raster(EncodingRaster::EncodingU8, &u8s)),
        densidad_bosque: Some(raster(EncodingRaster::EncodingU8, &u8s)),
        anillos: Some(raster(EncodingRaster::EncodingU8, &anillos)),
        transitable: Some(raster(EncodingRaster::EncodingBits, &transitable)),
        bosques: vec![ZonaBosque { id: "b1".into(), centro: punto(620_000, 320_000), radio: 40_000, densidad_milesimas: 800, area: 1 }],
        nodos,
        campamentos: CAMPAMENTOS
            .iter()
            .enumerate()
            .map(|(i, &(x, y))| CampamentoMercenarios { id: format!("c{i}"), posicion: punto(x, y), ..Default::default() })
            .collect(),
        bandidos_iniciales: vec![CampamentoBandidos { id: "bd0".into(), posicion: punto(700_000, 700_000), bosque_id: "b1".into(), ..Default::default() }],
        alijos: vec![Alijo { id: "al0".into(), posicion: punto(900_000, 700_000), oro: 100 }],
        navegacion: Some(nav),
        parcelas,
        ..Default::default()
    }
}

fn opciones() -> OpcionesValidacion {
    OpcionesValidacion { tamano_esperado: Some((ANCHO, ALTO)), rp: 8, ..Default::default() }
}

/// Aplica el cambio, sella y carga.
fn cargar_con(cambio: impl FnOnce(&mut ArtefactoDeMundo)) -> Result<Mundo, ErrorMundo> {
    let mut a = base();
    cambio(&mut a);
    Mundo::cargar(&sellar(&mut a), &opciones())
}

fn error_de(cambio: impl FnOnce(&mut ArtefactoDeMundo)) -> ErrorMundo {
    cargar_con(cambio).expect_err("debía rechazarse")
}

#[test]
fn el_mundo_valido_carga_y_responde_consultas() {
    let m = cargar_con(|_| {}).unwrap();
    assert_eq!(m.hash().len(), 64);
    assert_eq!(m.tamano(), (ANCHO, ALTO));
    assert_eq!(m.rejilla(), (COLS, FILAS));
    assert!(m.transitable_en(TIERRA.0, TIERRA.1));
    assert!(!m.transitable_en(LAGO.0 + 10, LAGO.2 + 10));
    assert!(!m.transitable_en(PENASCO.0 + 10, PENASCO.2 + 10));
    assert!(!m.transitable_en(-1, 0) && !m.transitable_en(ANCHO, 0) && !m.transitable_en(0, ALTO));
    assert_eq!(m.terreno_en(LAGO.0 + 10, LAGO.2 + 10), Some(ClaseTerreno::TerrenoAgua));
    assert_eq!(m.terreno_en(PENASCO.0 + 10, PENASCO.2 + 10), Some(ClaseTerreno::TerrenoMontana));
    assert_eq!(m.terreno_en(TIERRA.0, TIERRA.1), Some(ClaseTerreno::TerrenoLlano));
    assert_eq!(m.terreno_en(ANCHO + 1, 0), None);
    assert_eq!(m.anillo_en(CAMPAMENTOS[0].0, CAMPAMENTOS[0].1), Some(Anillo::Foco));
    assert_eq!(m.anillo_en(CAMPAMENTOS[0].0 + 200_000, CAMPAMENTOS[0].1), Some(Anillo::Frontera));
    assert_eq!(m.anillo_en(1_100_000, 700_000), Some(Anillo::Lejana));
    assert_eq!(m.altura_en(8000, 0), Some(2)); // celda 2 de la fila 0
    assert_eq!((m.nodos().len(), m.parcelas().len(), m.campamentos().len(), m.alijos().len()), (31, 9, 2, 1));
    assert_eq!(m.navegacion().aristas.len(), 2);
}

#[test]
fn bytes_que_no_son_un_artefacto() {
    assert!(matches!(Mundo::cargar(&[0xff, 0xff, 0xff], &opciones()), Err(ErrorMundo::Decodificacion(_))));
}

#[test]
fn reencodificar_un_mensaje_decodificado_da_los_mismos_bytes() {
    let bytes = sellar(&mut base());
    assert_eq!(ArtefactoDeMundo::decode(bytes.as_slice()).unwrap().encode_to_vec(), bytes);
}

#[test]
fn hash_alterado() {
    let mut a = base();
    sellar(&mut a);
    a.manifiesto.as_mut().unwrap().hash = "0".repeat(64);
    assert!(matches!(Mundo::cargar(&a.encode_to_vec(), &opciones()), Err(ErrorMundo::HashDelContenido { .. })));
}

#[test]
fn contenido_alterado_sin_resellar() {
    let mut a = base();
    sellar(&mut a);
    a.alijos[0].oro += 1;
    assert!(matches!(Mundo::cargar(&a.encode_to_vec(), &opciones()), Err(ErrorMundo::HashDelContenido { .. })));
}

#[test]
fn tamano_distinto_del_esperado() {
    let mut a = base();
    let bytes = sellar(&mut a);
    let op = OpcionesValidacion { tamano_esperado: Some((2_667_000, 2_000_000)), ..Default::default() };
    assert_eq!(
        Mundo::cargar(&bytes, &op).unwrap_err(),
        ErrorMundo::TamanoNoEsperado { esperado: (2_667_000, 2_000_000), obtenido: (ANCHO, ALTO) }
    );
}

#[test]
fn capa_con_sha256_distinto_al_declarado() {
    let mut a = base();
    sellar(&mut a);
    a.manifiesto.as_mut().unwrap().capas[1].sha256 = "ab".repeat(32);
    sellar_hash(&mut a);
    assert!(matches!(Mundo::cargar(&a.encode_to_vec(), &opciones()), Err(ErrorMundo::CapaSha256 { capa: "terreno", .. })));
}

#[test]
fn capa_ausente_del_manifiesto() {
    let mut a = base();
    sellar(&mut a);
    a.manifiesto.as_mut().unwrap().capas.retain(|c| c.nombre != "humedad");
    sellar_hash(&mut a);
    assert_eq!(Mundo::cargar(&a.encode_to_vec(), &opciones()).unwrap_err(), ErrorMundo::CapaSinDeclarar { capa: "humedad" });
}

#[test]
fn raster_ausente() {
    assert!(matches!(error_de(|a| a.fertilidad = None), ErrorMundo::RasterAusente { capa: "fertilidad" }));
}

#[test]
fn capa_corrupta_con_sha256_correcto() {
    // zstd inválido pero sellado: pasa hash y sha256, falla al descomprimir.
    let e = error_de(|a| a.densidad_bosque.as_mut().unwrap().datos = vec![1, 2, 3, 4, 5]);
    assert!(matches!(e, ErrorMundo::RasterDescompresion { capa: "densidad_bosque", .. }));
}

#[test]
fn capa_con_longitud_incoherente() {
    let e = error_de(|a| a.humedad = Some(raster(EncodingRaster::EncodingU8, &[0u8; 10])));
    assert_eq!(e, ErrorMundo::RasterLongitud { capa: "humedad", esperado: COLS * FILAS, obtenido: 10 });
}

#[test]
fn capa_con_dimensiones_incoherentes() {
    let e = error_de(|a| a.anillos.as_mut().unwrap().columnas = 7);
    assert_eq!(e, ErrorMundo::RasterDimensiones { capa: "anillos", esperado: (COLS as u32, FILAS as u32, 4000), obtenido: (7, FILAS as u32, 4000) });
}

#[test]
fn capa_con_encoding_incorrecto() {
    let e = error_de(|a| a.transitable.as_mut().unwrap().encoding = EncodingRaster::EncodingU8 as i32);
    assert!(matches!(e, ErrorMundo::RasterEncoding { capa: "transitable", .. }));
}

#[test]
fn id_repetido() {
    let e = error_de(|a| a.alijos.push(a.alijos[0].clone()));
    assert_eq!(e, ErrorMundo::IdRepetido { coleccion: "alijos", id: "al0".into() });
    let e = error_de(|a| a.nodos.push(a.nodos[0].clone()));
    assert_eq!(e, ErrorMundo::IdRepetido { coleccion: "nodos", id: "ganado-0".into() });
    let e = error_de(|a| a.navegacion.as_mut().unwrap().nodos[1].id = 1);
    assert_eq!(e, ErrorMundo::IdRepetido { coleccion: "navegacion.nodos", id: "1".into() });
}

#[test]
fn arista_a_celda_de_agua() {
    let e = error_de(|a| a.navegacion.as_mut().unwrap().aristas.push(AristaGrafo { a: 3, b: 4, coste_milesimas: 1000, vado: false }));
    assert_eq!(e, ErrorMundo::AristaNoTransitable { a: 3, b: 4, nodo: 4, posicion: (LAGO.0 + 8_000, LAGO.2 + 8_000) });
}

#[test]
fn arista_a_nodo_inexistente() {
    let e = error_de(|a| a.navegacion.as_mut().unwrap().aristas.push(AristaGrafo { a: 1, b: 99, coste_milesimas: 1000, vado: false }));
    assert_eq!(e, ErrorMundo::AristaConNodoInexistente { a: 1, b: 99, nodo: 99 });
}

#[test]
fn recurso_campamento_bandidos_y_alijo_en_agua() {
    let (x, y) = (LAGO.0 + 8_000, LAGO.2 + 8_000);
    let e = error_de(|a| a.nodos.push(nodo("cobre-0", "cobre", None, x, y)));
    assert_eq!(e, ErrorMundo::EnAgua { coleccion: "nodos", id: "cobre-0".into(), posicion: (x, y) });
    let e = error_de(|a| a.alijos[0].posicion = punto(x, y));
    assert!(matches!(e, ErrorMundo::EnAgua { coleccion: "alijos", .. }));
    let e = error_de(|a| a.bandidos_iniciales[0].posicion = punto(x, y));
    assert!(matches!(e, ErrorMundo::EnAgua { coleccion: "bandidos_iniciales", .. }));
    let e = error_de(|a| a.campamentos[1].posicion = punto(x, y));
    assert!(matches!(e, ErrorMundo::EnAgua { coleccion: "campamentos", .. }));
}

#[test]
fn nodo_en_tierra_no_transitable_o_fuera_del_mundo() {
    let (x, y) = (PENASCO.0 + 8_000, PENASCO.2 + 8_000);
    let e = error_de(|a| a.nodos.push(nodo("piedra-0", "piedra", None, x, y)));
    assert_eq!(e, ErrorMundo::NoTransitable { coleccion: "nodos", id: "piedra-0".into(), posicion: (x, y) });
    let e = error_de(|a| a.nodos.push(nodo("piedra-1", "piedra", None, ANCHO + 5, 0)));
    assert!(matches!(e, ErrorMundo::NoTransitable { .. }));
}

#[test]
fn nodo_sin_posicion() {
    let e = error_de(|a| a.nodos[0].posicion = None);
    assert_eq!(e, ErrorMundo::SinPosicion { coleccion: "nodos", id: "ganado-0".into() });
}

#[test]
fn estano_con_cuotas_erroneas() {
    // Un estaño de la lejana declarado en el foco: foco 2 / frontera 5 / lejana 13, no 1 / 5 / 14.
    let e = error_de(|a| a.nodos.iter_mut().find(|n| n.id == "estano-lejana-0").unwrap().anillo = Some("foco".into()));
    assert_eq!(e, ErrorMundo::CuotaPorAnillo { tipo: "estano", anillo: "foco", esperado: 1, obtenido: 2 });
}

#[test]
fn oro_con_cuotas_erroneas() {
    // El empate de restos (10 nodos) lo gana el foco; si el foco queda vacío y la frontera tiene 3 no vale.
    let e = error_de(|a| a.nodos.iter_mut().find(|n| n.id == "oro-foco-0").unwrap().anillo = Some("frontera".into()));
    assert_eq!(e, ErrorMundo::CuotaPorAnillo { tipo: "oro", anillo: "foco", esperado: 1, obtenido: 0 });
}

#[test]
fn quitar_un_estano_cambia_el_total_y_las_cuotas() {
    // 19 nodos → 0,95/4,75/13,3 → 0/4/13 + 2 restos a (foco .95, frontera .75) = 1/5/13: quitar de la lejana da 1/5/13.
    assert!(cargar_con(|a| a.nodos.retain(|n| n.id != "estano-lejana-0")).is_ok());
    // Quitar uno del foco deja 0/5/14: no coincide con 19 nodos (1/5/13).
    let e = error_de(|a| a.nodos.retain(|n| n.id != "estano-foco-0"));
    assert_eq!(e, ErrorMundo::CuotaPorAnillo { tipo: "estano", anillo: "foco", esperado: 1, obtenido: 0 });
}

#[test]
fn anillo_de_metal_ausente_o_desconocido() {
    let e = error_de(|a| a.nodos.iter_mut().find(|n| n.id == "oro-lejana-0").unwrap().anillo = None);
    assert_eq!(e, ErrorMundo::AnilloInvalido { tipo: "oro".into(), id: "oro-lejana-0".into(), anillo: None });
    let e = error_de(|a| a.nodos.iter_mut().find(|n| n.id == "oro-lejana-0").unwrap().anillo = Some("cerca".into()));
    assert!(matches!(e, ErrorMundo::AnilloInvalido { .. }));
}

#[test]
fn parcelas_solapadas() {
    let e = error_de(|a| a.parcelas[1].centro = punto(600_000 + 15_999, 300_000));
    assert_eq!(e, ErrorMundo::ParcelasCercanas { a: "p0".into(), b: "p1".into(), distancia: 15_999, minima: 16_000 });
    // Con un rp mayor en las opciones, las mismas parcelas dejan de valer.
    let mut a = base();
    let op = OpcionesValidacion { rp: 9, ..opciones() };
    assert!(matches!(Mundo::cargar(&sellar(&mut a), &op), Err(ErrorMundo::ParcelasCercanas { minima: 18_000, .. })));
}

#[test]
fn parcela_de_un_bosque_inexistente() {
    let e = error_de(|a| a.parcelas[3].bosque_id = "fantasma".into());
    assert_eq!(e, ErrorMundo::ParcelaSinBosque { id: "p3".into(), bosque_id: "fantasma".into() });
}

#[test]
fn campamentos_de_mercenarios_demasiado_cerca() {
    let e = error_de(|a| a.campamentos[1].posicion = punto(CAMPAMENTOS[0].0 + 599_999, CAMPAMENTOS[0].1));
    assert_eq!(e, ErrorMundo::CampamentosCercanos { a: "c0".into(), b: "c1".into(), distancia: 599_999, minima: 600_000 });
    // Con la separación mínima configurada más baja, pasa.
    let mut a = base();
    a.campamentos[1].posicion = punto(CAMPAMENTOS[0].0 + 599_999, CAMPAMENTOS[0].1);
    let op = OpcionesValidacion { separacion_campamentos: 500, ..opciones() };
    assert!(Mundo::cargar(&sellar(&mut a), &op).is_ok());
}
