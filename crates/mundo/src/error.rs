//! Errores de carga: una variante por regla incumplida, con los datos para diagnosticar.
//! Los puntos van en milésimas de unidad estratégica, `(x, y)`.

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErrorMundo {
    #[error("el artefacto no se puede decodificar: {0}")]
    Decodificacion(String),
    #[error("el artefacto no trae manifiesto")]
    SinManifiesto,
    #[error("hash del contenido: el manifiesto declara {esperado}, el contenido da {obtenido}")]
    HashDelContenido { esperado: String, obtenido: String },
    #[error("tamaño del mundo inválido: {obtenido:?}")]
    TamanoInvalido { obtenido: (i64, i64) },
    #[error("tamaño del mundo: se esperaba {esperado:?} y el manifiesto declara {obtenido:?}")]
    TamanoNoEsperado { esperado: (i64, i64), obtenido: (i64, i64) },
    #[error("el manifiesto no declara la capa «{capa}»")]
    CapaSinDeclarar { capa: &'static str },
    #[error("el manifiesto declara dos veces la capa «{capa}»")]
    CapaRepetida { capa: String },
    #[error("sha256 de la capa «{capa}»: el manifiesto declara {esperado}, los datos dan {obtenido}")]
    CapaSha256 { capa: &'static str, esperado: String, obtenido: String },
    #[error("falta el raster «{capa}»")]
    RasterAusente { capa: &'static str },
    #[error("raster «{capa}»: (columnas, filas, tamano_celda) esperado {esperado:?} y obtenido {obtenido:?}")]
    RasterDimensiones { capa: &'static str, esperado: (u32, u32, u32), obtenido: (u32, u32, u32) },
    #[error("raster «{capa}»: encoding {obtenido} en vez de {esperado}")]
    RasterEncoding { capa: &'static str, esperado: i32, obtenido: i32 },
    #[error("raster «{capa}»: no descomprime ({detalle})")]
    RasterDescompresion { capa: &'static str, detalle: String },
    #[error("raster «{capa}»: {obtenido} bytes en vez de {esperado}")]
    RasterLongitud { capa: &'static str, esperado: usize, obtenido: usize },
    #[error("id repetido en {coleccion}: «{id}»")]
    IdRepetido { coleccion: &'static str, id: String },
    #[error("{coleccion} «{id}» sin posición")]
    SinPosicion { coleccion: &'static str, id: String },
    #[error("arista ({a}, {b}) usa el nodo {nodo}, que no existe en el grafo")]
    AristaConNodoInexistente { a: u32, b: u32, nodo: u32 },
    #[error("arista ({a}, {b}): el nodo {nodo} en {posicion:?} no está en celda transitable")]
    AristaNoTransitable { a: u32, b: u32, nodo: u32, posicion: (i64, i64) },
    #[error("{coleccion} «{id}» en {posicion:?}: está en agua")]
    EnAgua { coleccion: &'static str, id: String, posicion: (i64, i64) },
    #[error("{coleccion} «{id}» en {posicion:?}: no está en celda transitable")]
    NoTransitable { coleccion: &'static str, id: String, posicion: (i64, i64) },
    #[error("nodo de {tipo} «{id}»: anillo {anillo:?} ausente o desconocido (foco | frontera | lejana)")]
    AnilloInvalido { tipo: String, id: String, anillo: Option<String> },
    #[error("cuota de {tipo} en el anillo {anillo}: se esperaban {esperado} nodos y hay {obtenido}")]
    CuotaPorAnillo { tipo: &'static str, anillo: &'static str, esperado: u32, obtenido: u32 },
    #[error("parcela «{id}»: el bosque «{bosque_id}» no existe")]
    ParcelaSinBosque { id: String, bosque_id: String },
    #[error("parcelas «{a}» y «{b}» a {distancia} milésimas, mínimo {minima}")]
    ParcelasCercanas { a: String, b: String, distancia: i64, minima: i64 },
    #[error("campamentos de mercenarios «{a}» y «{b}» a {distancia} milésimas, mínimo {minima}")]
    CampamentosCercanos { a: String, b: String, distancia: i64, minima: i64 },
}
