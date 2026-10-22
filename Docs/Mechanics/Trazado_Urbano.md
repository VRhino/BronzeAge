# Trazado urbano — la Vista de Asentamiento

> **Reescrito el 2026-10-13**: el **servidor calcula el trazado** de cada asentamiento una vez, con este algoritmo, y lo entrega; Unity solo lo dibuja (decisión del usuario). El espacio local de la ciudad y el mapa del mundo son **dos espacios separados, sin conversión entre ellos** (Doc 1.0a; la escala de transición `ESCALA` se borró). Los algoritmos de este documento no cambian. El «cómo funcionaba» está en `typescript-final:Docs/Mechanics/Trazado_Urbano.md`.

> **Implementado en Rust (2026-10-18).** La geometría es el crate `crates/trazado` (reglas puras en enteros; `LAYOUT_VERSION` = 3, no 2: `pseudoAleatorio` con `sin` se sustituyó por un hash entero, así que **no reproduce el trazado de TypeScript**). Su integración en el núcleo:
>
> - `crates/nucleo/src/urbanismo.rs`: la tabla tipo → huella y categoría (las huellas salen de los bloques `EDIFICIO_TAMANO_POR_DEFECTO`, `EDIFICIO_TAMANO`, `PUESTO_MERCADO_FORMA` y `MERCADO_PUESTOS_POR_NIVEL` de este documento, incrustado con `include_str!`, y del `tamano` de cada nivel de la Granja en `EDIFICIO_CATALOGO`; **esos bloques entran en la huella de las reglas de las fotos**; la categoría de cada tipo es §6.1), la `Urbe` de cada plaza (la `Ciudad` más el caché de consultas) y `SitiosUrbe`. Cada plaza del núcleo lleva su `Urbe` (`nucleo::Asentamiento.urbe`); `Nucleo::ciudad(id)` y `Nucleo::vista_ciudad(id)` la entregan para pintar (piezas con su rectángulo en unidades locales, calles y caminos).
> - `crates/nucleo/src/codec_ciudad.rs`: serialización para el hash y las fotos (formato de la ciudad introducido en `BAN4` y vigente en `BAN5`: semilla, `Parametros`, piezas en su orden de construcción y recintos; suelo y red se derivan al leer). Una foto con otra `LAYOUT_VERSION` se rechaza.
> - `crates/asentamiento`: el trait `Sitios` coloca (`proponer`, `tomar(id, …)`, `puestos_de_mercado`, `hueco_al_mejorar`, `reubicar`) y recibe un `Urbano` (nivel, nivel operativo, edificios completados, perfil de la ordenanza, id siguiente). Las anclas de saturación y los puestos del Mercado entran en la economía como edificios activos y gratuitos (`Evento::EdificioGratuito`, cuentan a la mitad en el mantenimiento, no agrandan la zona ni la ciudad).
> - **Semilla** de cada ciudad: `hash(semilla del núcleo, "ciudad", [id de la plaza])`. **Perfil**: el de la ordenanza activa (`Politicas::perfil_trazado`) o, si no hay, el de la tradición de la plaza; solo afecta a lo que se coloca después. **Radio de la ciudad**: `radio_ciudad` con los edificios que pasaron por la cola y el nivel alcanzado; la zona del mapa es otro espacio y crece por su regla.
> - **Cuándo se coloca**: al *comprometer* el proyecto (al encolar, manual o automático), no al completarlo; quitarlo de la cola libera su sitio. Si el barrio no tiene ancla con hueco y el nivel operativo es ≥ 2, el trazado crea antes el ancla (un edificio gratuito más). Un proyecto que se queda sin sitio entre la pregunta y el compromiso (otro de la misma pasada se lo llevó) no se cobra ni se compromete. Los puestos del Mercado nacen al completarse y al mejorar a cada nivel; la Granja se muda al terminar su mejora y, si no hay hueco, la mejora espera conservando su cuadrilla (la mejora manual lo comprueba antes).
> - **Caché**: cada `Urbe` guarda «¿cabe este tipo?» y «¿cabe esta mejora?» con lo único de lo que depende (revisión de la ciudad, radio, id siguiente y gate donde se crean anclas, perfil); no entra en el hash ni en las fotos y no cambia ningún resultado.
> - **Pueblos extractores** (2026-10-19, fase 3C): su planta es `urbanismo::planta_de_pueblo(reglas, semilla, id, extractor)`, una función pura del id que no se guarda (el extractor como ancla en el origen, 4 casas, el almacén y el corral, con un radio de 8 u. locales); el pueblo no tiene pieza en la ciudad de su madre ni pasa por la cola (`Economia_Construccion.md` §14).
- **Recintos de muralla** (2026-10-22, fase de Murallas, paso 1): el **trazo** vive en el mismo crate, módulo `trazado::muralla` (`trazar_recinto`, `interior_de`, `contiene_a`, `edificios_extramuros`, `marcar_torres`; ver [`Murallas.md`](Murallas.md) §3 y la nota de implementación al final). El anillo se calcula sobre la red de la ciudad **sin recintos** y sobre la misma ventana de ±96 celdas (se rechaza si no cabe). **Falta** que el núcleo trace y comprometa recintos: `Urbe::set_recintos` es el gancho, sin llamadores de producción, y `codec_ciudad` aún persiste los recintos (pasará a derivarlos de la plaza). El batch de población sigue sin ciudad (todo cabe). Las `DUDA` abiertas están anotadas en el código (`crates/nucleo/src/urbanismo.rs`, `crates/asentamiento/src/{asentamiento,obras}.rs` y `crates/trazado`).

Cómo se decide **dónde va cada edificio** de un asentamiento y por dónde pasan sus calles. Es una geometría pura sobre una rejilla, determinista y sin
estado propio: lo único que se guarda de ella son las posiciones de los edificios (y los recintos, ver [`Murallas.md`](Murallas.md)).

- **Verificado contra:** `e18c0b4` (2026-10-09).
- **Módulo:** `src/engine/trazado.ts` (≈2.400 líneas). Lo usan `engine/construction.ts` (auto-construcción, construcción manual, `asegurarAnclaPara`), `engine/settlement.ts`
  (fundación) y `engine/muralla.ts` (comparte el vocabulario de celdas).
- **Diseño e historia:** el documento largo de diseño (`Vista_Asentamiento_Trazado_Urbano.md`, modelo de *aristas* ya retirado) se borró el 2026-10-12; la descripción vigente es este documento.
  Esta es la descripción vigente.

## 1. Los tres pilares

1. **La rejilla manda.** Todo (edificios, calles, murallas) son **celdas cuadradas** de una rejilla local. Una calle **ocupa suelo**: es una celda más. (Hasta la Etapa 6
   corrían sobre aristas y eran gratis, lo que producía calles de ancho cero.)
2. **La red de calles es derivada, no persistida.** Se reconstruye siempre desde `edificios` (en su orden de construcción) y `recintos`. Mismo input → misma red.
3. **Nada se pre-genera.** Al fundar solo existe el anillo de calle que rodea al Centro Urbano. Todo lo demás aparece cuando un edificio concreto lo necesita.

Las manzanas no existen como dato: son el espacio que queda encerrado cuando las calles cierran un ciclo.

## 2. Modelo de datos

### 2.1 Espacios y escala

- **Espacio local** de cada asentamiento: plano, centrado en el Centro Urbano, `(0,0)` = su centro. Unidades «locales».
- **Celda** = `REJILLA_ASENTAMIENTO.tamanoCelda` = **3 unidades locales** de lado. Una calle mide una celda (`TRAZADO.anchoCalle = 1`).
- `radioMapa = 220` es solo presentación (zoom del lienzo); no interviene en ninguna decisión.
- `LAYOUT_VERSION = 2`: revisión de la geometría. **Un snapshot de otra revisión se rechaza al cargar** (`cargarPartida`), porque las huellas y la red se *derivan* de las tablas y se
  reinterpretarían en silencio. Se sube al cambiar cualquier huella, el tamaño de celda, `FONDO_MANZANA` o una constante de `TRAZADO` que mueva lo ya colocado.
- El espacio local de la ciudad y el mapa del mundo son **espacios separados**: la ciudad no se mide contra el exterior (decidido el 2026-10-17; `ESCALA` y `radioUrbanoDe` se borraron). Su tamaño es **propio e interno**: `radioCiudad`, en unidades locales, nace en `TRAZADO.radioCiudad.inicial` (30), crece `TRAZADO.radioCiudad.crecimientoPorEdificio` (5) por edificio completado y topa en `radioCiudad.maximoPorNivel` (60 / 90 / 120 / 150 / 180 para los niveles 1-5). No se deriva del radio de la zona de influencia ni de ninguna cifra del mapa; los valores son los de siempre, así que el trazado no cambia.

### 2.2 Celda, rectángulo y posición

- `Celda = { col, row }` (enteros; `+row` es el sur).
- `Edificio.posicion` es el **centro** del rectángulo que ocupa, en unidades locales. Con `T = 3`:
  - `posicion = ((minCol + ancho/2)·T, (minRow + alto/2)·T)`
  - `minCol = round(posicion.x / T − ancho/2)`, `minRow = round(posicion.y / T − alto/2)` (inversa exacta para lados pares e impares).
- `rotado = true` intercambia `ancho` y `alto` de la huella.
- El Centro Urbano mide **4×4** (lado par, para que su centro caiga en un vértice y pueda ser `(0,0)`) y ocupa las columnas −2..1 y las filas −2..1.
- Claves de conjunto: texto `"col,row"` en la mayor parte; en las rutas calientes de `muralla.ts` y del BFS, enteros `(col+32768)·65536 + (row+32768)`.

### 2.3 Campos de `Edificio` que intervienen

| Campo | Significado en el trazado |
|---|---|
| `tipo` | decide huella, categoría y por qué rama de colocación pasa |
| `posicion` | centro del rectángulo (arriba) |
| `nivelInterno` | solo cambia la huella en `granja` (progresión) y `puestoMercado` (identifica la **forma**, no el nivel) |
| `rotado` | huella girada |
| `ambito` | `'asentamiento'` (por defecto; participa de la rejilla) o `'mapa'` (minas, cantera: no participan) |
| `anclaLlena` | (anclas) su núcleo ya no admite más satélites — latch permanente |
| `semillaSaturada` | (anclas) sus 5 ranuras de crecimiento se agotaron — latch permanente |

El **orden del array `edificios` es significativo**: es el orden de construcción, solo se hace `push`/`map`, nunca se reordena, y la red de calles se reconstruye recorriéndolo.

### 2.4 Huellas

`tamanoEdificio(tipo, nivelInterno)`:
- `granja` → `EDIFICIO_CATALOGO.granja.niveles[nivelInterno ?? 1].tamano` (2×2, 2×3, 4×3, 6×6 en los niveles 1-4).
- `puestoMercado` → `PUESTO_MERCADO_FORMA[nivelInterno ?? 1]`.
- cualquier otro → `EDIFICIO_TAMANO[tipo]`, y si no está en la tabla `EDIFICIO_TAMANO_POR_DEFECTO` (1×1: vivienda, leñera, minas, gran fundición, maravilla).

<!-- parametros:EDIFICIO_TAMANO_POR_DEFECTO -->
```json
{
  "ancho": 1,
  "alto": 1
}
```
<!-- /parametros -->

<!-- parametros:EDIFICIO_TAMANO -->
```json
{
  "centroUrbano": {
    "ancho": 4,
    "alto": 4
  },
  "carpinteria": {
    "ancho": 4,
    "alto": 2
  },
  "fundicion": {
    "ancho": 2,
    "alto": 2
  },
  "curtiduria": {
    "ancho": 2,
    "alto": 2
  },
  "armeria": {
    "ancho": 2,
    "alto": 3
  },
  "barracon": {
    "ancho": 2,
    "alto": 2
  },
  "galeriaDeTiro": {
    "ancho": 2,
    "alto": 4
  },
  "caballerizas": {
    "ancho": 3,
    "alto": 2
  },
  "salaConsejo": {
    "ancho": 3,
    "alto": 3
  },
  "taberna": {
    "ancho": 2,
    "alto": 2
  },
  "mercado": {
    "ancho": 3,
    "alto": 2
  },
  "palacio": {
    "ancho": 4,
    "alto": 4
  },
  "corral": {
    "ancho": 4,
    "alto": 3
  },
  "almacen": {
    "ancho": 2,
    "alto": 1
  },
  "granero": {
    "ancho": 4,
    "alto": 2
  },
  "plaza": {
    "ancho": 2,
    "alto": 2
  },
  "plazaDeArmas": {
    "ancho": 2,
    "alto": 2
  },
  "patioDeGremios": {
    "ancho": 2,
    "alto": 2
  },
  "pozo": {
    "ancho": 1,
    "alto": 1
  },
  "parque": {
    "ancho": 3,
    "alto": 2
  }
}
```
<!-- /parametros -->

<!-- parametros:PUESTO_MERCADO_FORMA -->
```json
{
  "1": {
    "ancho": 1,
    "alto": 2
  },
  "2": {
    "ancho": 1,
    "alto": 3
  },
  "3": {
    "ancho": 1,
    "alto": 1
  }
}
```
<!-- /parametros -->

<!-- parametros:MERCADO_PUESTOS_POR_NIVEL -->
```json
{
  "1": [
    1,
    1,
    2,
    2,
    3
  ],
  "2": [
    1,
    1,
    2,
    2,
    3,
    3,
    3
  ],
  "3": [
    1,
    2,
    2,
    3
  ]
}
```
<!-- /parametros -->

`MERCADO_PUESTOS_POR_NIVEL[n]` son las **formas** de los puestos que se añaden al *alcanzar* el nivel interno `n` de Mercado (acumulativo): la zona queda en 6, 13 y 17 piezas contando el propio Mercado.

## 3. Determinismo

Nada del trazado puede usar un generador aleatorio global. Toda variación por asentamiento sale de su `id`:

```
hashTexto(texto)      = FNV-1a de 32 bits sobre los charCode UTF-16:  h = 2166136261; por cada c: h ^= c; h = (h × 16777619) mod 2³²   (resultado sin signo)
pseudoAleatorio(n)    = x − floor(x)  con  x = sin(n × 12.9898) × 43758.5453          → [0,1)
```

> **Aviso de portabilidad:** `pseudoAleatorio` depende de que `sin` de doble precisión dé el mismo resultado al bit en el otro lenguaje. Las bibliotecas matemáticas pueden diferir en el último bit, y
> este valor decide desempates y orientaciones. Al migrar, o se reproduce `sin` de V8 exactamente, o se sustituye por otro generador **y se acepta que las ciudades ya guardadas cambian**
> (lo cubriría un `LAYOUT_VERSION` nuevo).

Las sales de texto que se hashean (con `${id}` = id del asentamiento):

| Sal | Resultado |
|---|---|
| `${id}-largo-fila` | ancho de manzana: `largoFilaMin + floor(pa × (largoFilaMax − largoFilaMin + 1))` → 4..8 |
| `${id}-desfase-manzana` | desfase del retículo: `floor(pa × maxFila)` |
| `${id}-rotacion-eje` | giro del eje de ranuras: `pa × PASO_RANURA` (entre 0 y 72°) |
| `${id}-perfil-trazado` | perfil por tradición: `PERFILES_TRAZADO[floor(pa × 4)]` |
| `${semilla.id}-${idNuevaAncla}-orientacion-ancla` | orientación de un ancla no cuadrada (`pa < 0.5` → normal primero) |
| `${semilla.id}-ranura-${i}` | barajado de las 5 ranuras (Fisher–Yates descendente, `j = floor(pa × (i+1))`) |
| `${col},${row},${rotado}` | desempate final de un candidato (`semillaCandidato`) |
| `${idAncla}` (`tipoAnclaParaCategoria`) | tipo de ancla de saturación: `opciones[floor(pa × n)]` |

## 4. El suelo ocupado

`ocupadasConRed(edificios, red, recintos, excluirId?)` es el único punto por el que pasa toda colocación. Una celda está **ocupada** si es:

1. una celda de un edificio interno (excepto `excluirId`),
2. una celda de **calle** o de **camino** de la red,
3. una celda de **muro o torre** de cualquier recinto, **levantada o no** (basta con que el recinto esté comprometido). Las **puertas no ocupan**: son transitables.

Hay una sola definición a propósito: tener dos (una para anclas, otra para la colocación real) ya causó un bloqueo en el que la ciudad dejó de construir viviendas para siempre.

## 5. La red de calles

`redDeCalles(id, edificios, recintos)` devuelve `{ calles, caminos }` (conjuntos de celdas). **Calle** = urbana, forma manzanas. **Camino** = rural, solo conecta granjas y corrales. Se memoiza por contenido
(§9), pero conceptualmente es la función pura de abajo.

### 5.1 Algoritmo `calcularRedDeCalles`

```
entrada: id, edificios (orden de construcción), recintos
si no hay Centro Urbano → red vacía

ocupadas := celdas de TODOS los edificios internos            (desde el principio, no paso a paso)
          ∪ celdas de muro y torre de los recintos            (las puertas NO)

añadirConectadas( anillo(CU), ocupadas, red, calles )         // semilla: el perímetro del Centro Urbano

maxFila  := largoMaxFila(id)                                  // 4..8
desfase  := floor(pa(hash(id+"-desfase-manzana")) × maxFila)
pasoCol  := maxFila + anchoCalle                               // manzana + su calle
pasoFila := FONDO_MANZANA + anchoCalle                         // 2 + 1 = 3

para cada edificio e (en orden, saltando el Centro Urbano):
    rect := rectángulo de e
    destino := (e es granja|corral) ? caminos : calles
    si NO tieneFrenteDeCalle(rect):                            // ninguna celda ortogonal adyacente es red (calle o camino)
        corredor := corredorHastaLaRed(rect, ocupadas, red, cap)   // cap = capCorredorAfueras si es de afueras, si no capCorredorUrbano
        si corredor ≠ null: añadir todas sus celdas a destino
    si e ∈ ANCLAS_REALES: añadirConectadas( anillo(rect), …, calles )
    si e es de afueras: continuar                              // un camino rural no forma manzanas
    franja := celdas del retículo pegadas a e (ver abajo)
    añadirConectadas( franja, ocupadas, red, calles )
```

**Retículo blando** (la franja): para `g` de 0 a `anchoCalle−1`:
- lado izquierdo: si `esBordeDeManzana(minCol−1−g, pasoCol, desfase)` → las celdas `(minCol−1−g, minRow..minRow+alto−1)`;
- lado derecho: si `esBordeDeManzana(minCol+ancho+g, …)` → `(minCol+ancho+g, cada fila del rect)`;
- arriba: si `esBordeDeManzana(minRow−1−g, pasoFila, desfase)` → `(cada col del rect, minRow−1−g)`;
- abajo: si `esBordeDeManzana(minRow+alto+g, …)` → `(cada col del rect, minRow+alto+g)`.

con `esBordeDeManzana(i, paso, desfase) = ((i − desfase) mod paso) == 0` (módulo siempre no negativo). El criterio es **posicional** (no depende del orden de construcción): se probó por largo de hilera y por
módulos y salían transversales amontonadas o ausentes.

**Primitivas:**

- `anillo(rect)`: las celdas del anillo de una celda de grosor que rodea el rectángulo, **con las cuatro esquinas**.
- `adyacentes(rect)`: lo mismo **sin las esquinas** (por una diagonal no se sale).
- `tieneFrenteDeCalle(rect)`: alguna celda de `adyacentes(rect)` está en `calles ∪ caminos`.
- `añadirConectadas(candidatas, ocupadas, red, destino)`: de las candidatas, descarta las ocupadas y las que ya son red; las que quedan se añaden **solo si se conectan** a la red existente,
  directamente (una vecina ortogonal está en la red) o a través de otras del mismo grupo (flood fill 4-conexo). Con la red completamente vacía todas las libres son semilla (caso raíz: el anillo del
  Centro Urbano). Un tramo huérfano **no se añade**: no hay islas.
- `corredorHastaLaRed(rect, ocupadas, red, cap)`: BFS 4-conexo sobre celdas libres. Arranca en las celdas de `adyacentes(rect)` (en ese orden fijo, paso 1), vecinos en el orden **arriba, derecha, abajo,
  izquierda**; nunca atraviesa el propio rectángulo; una celda es meta cuando alguna vecina ortogonal está en la red. Devuelve el camino de longitud mínima (de la meta hacia atrás) o `null` si no hay dentro de
  `cap` pasos. **El orden de arranque y de vecinos fija el corredor y, con él, la red entera.**

### 5.2 Notas

- Las celdas de calle nunca pisan un edificio: cada añadido comprueba `ocupadas`. Un anillo o una franja pueden salir incompletos, y es correcto (un edificio grande desvía la calle).
- `ocupadas` lleva a **todos** los edificios desde el principio y no solo a los ya procesados: un corredor es un camino físico por la ciudad *tal como es ahora*. Antes llevaba solo los anteriores y un corredor
  recalculado podía atravesar un edificio posterior.
- Las **puertas de las murallas no se siembran como calle**: son huecos que se convierten en calle solos, cuando el replay estira un corredor a través de ellos.
- Una celda que es calle no se dibuja también como camino (`rectangulosDeRed`: «la calle manda»).

## 6. Colocación: ¿dónde va un edificio nuevo?

`sitiosParaTipo(asentamiento, edificios, tipo, nivelInterno?, perfil)` devuelve la **lista ordenada** de sitios posibles `{ punto, rotado }`; el mejor es el primero (`sitioParaTipo`). Vacía = no cabe.
El resultado depende solo de `(id, edificios, recintos con su avance, radioCiudad, tipo, nivelInterno, perfil)`.

El **perfil** efectivo es `TRAZADO.perfilForzado ?? perfil de la política activa ?? perfilPorTradicion(id)` (§8).

### 6.1 Despacho por tipo (en este orden)

1. **Afueras** (`granja`, `corral`) → §6.3.
2. **Genéricos** (`palacio`, `almacen`, `granero`, `lenera`, `taberna`, `salaConsejo`) → §6.4.
3. Sin categoría (`CATEGORIA_POR_TIPO[tipo]` ausente) → **lista vacía**.
4. **Ancla primaria de su categoría** (`mercado`) → nace por el árbol de anclas (§7.2): un único sitio, el de `crearAnclaNueva`.
5. **Satélites** (vivienda, fundición, curtiduría, armería, gran fundición, maravilla, barracón, galería de tiro, caballerizas, carpintería, puestos de mercado) → §6.5.

`CATEGORIA_POR_TIPO`: `residencial` (vivienda) · `industria` (fundición, curtiduría, armería, gran fundición, maravilla) · `militar` (barracón, galería de tiro, caballerizas, carpintería) · `mercado` (mercado,
puestoMercado) · `almacenaje` (almacén, granero, leñera, corral).

### 6.2 El barrido de candidatos (`candidatosLibres`)

Parámetros: `referencia` (rectángulo contra el que se miden radio y distancia; `ORIGEN_RECT` = el origen), `radioUrbano` (unidades locales), `tamano`, `ocupadas`, `red`, `distanciaMinima`, `rotado`, `capCorredor`.

```
centro  := centro de referencia;  maxCeldas := ceil(radioUrbano / T) + 1
para col, row en [centro±maxCeldas]:
    centroCandidato := ((col + ancho/2)·T, (row + alto/2)·T)
    si distancia(centroCandidato, centro) > radioUrbano → descartar
    si distanciaMinima > 0 y distanciaBordeAlCentro(rect, centro) < distanciaMinima → descartar   // se mide el BORDE de la huella, no su centro
    si el rectángulo contiene alguna celda ocupada → descartar                                       // tabla de sumas acumuladas 2D
    coste := costeDesdeLaRed(rect)                                                                    // ver abajo
    si coste = null → descartar                                                                       // GATE DURO: un sitio sin salida a la calle no es un sitio
    nivel := coste = 0 ? (continuaFila(rect) ? 0 : 1)
                       : (hay una celda ocupada pegada al rectángulo ? 2 : 3)
    emitir { min, punto, nivel, rotado }
```

- **`costeDesdeLaRed(rect)`**: 0 si alguna celda de `adyacentes(rect)` ya es red; si no, el menor número de celdas libres que hay que abrir para llegar a la red (campo de distancias de un único BFS
  multiorigen desde las celdas libres que tocan la red, distancia 1, propagado hasta `capCorredor`); `null` si es inalcanzable dentro del cap.
- **`continuaFila(rect)`**: alguna celda de red en `adyacentes(rect)` tiene, entre sus otras vecinas ortogonales (fuera del candidato), una celda ocupada (otro edificio da a la misma calle).
- **Niveles:** 0 continúa una fila · 1 da a calle · 2 pared con pared con algo · 3 suelto (habrá que estirar la red). Menor es mejor.
- Con `permitirRotacion` y huella no cuadrada, se barre también la huella girada y se unen los dos conjuntos. Nunca rota: `granja`. `puestoMercado` sí puede.

### 6.3 Afueras (granja, corral)

- `radioMaximoAfueras = max(radioUrbano, radioAfuerasMin + anchoBandaAfueras) + mediaDiagonal(huella)` (unidades locales). Nunca menos que `60 + 36`: el campo está **fuera** de la zona de influencia.
- Veto: ninguna celda de la huella a menos de `radioAfuerasMin` (60 u.l. = 20 celdas) del centro. Se mide contra el borde de la huella.
- `capCorredor = capCorredorAfueras` (200). Sus conexiones son **caminos**.
- Orden: **el más lejano al origen primero** (`porDistanciaAlOrigen(…, masLejos = true)`); empate por `semillaCandidato`. Así las granjas acompañan el borde de la ciudad.

### 6.4 Genéricos (palacio, almacén, granero, leñera, taberna, Sala del Consejo)

Sin ancla ni barrio: todos los candidatos del radio urbano con `distanciaMinima = 0`, ordenados **del más cercano al origen** (desempate `semillaCandidato`), y después la preferencia intramuros (§6.6).

### 6.5 Satélites: atracción dura a un ancla

1. Se elige la ancla: la **más cercana al origen** (por distancia de su `posicion`) entre las instancias de los tipos de ancla de la categoría (`tiposAnclaDe`: la primaria, si hay, y las de saturación) que no estén `anclaLlena`.
   Si no hay ninguna → lista vacía (quien llama debió garantizarla antes, §7.1).
2. `celdasAfines` = celdas de los edificios **afines** al tipo (mismo tipo, o misma categoría), excluida el ancla.
3. `sitiosPorAtraccionDura(ancla, tamano, ocupadas, red, permitirRotacion, celdasAfines, perfil)`:
   - `rectConAnillo` = rectángulo del ancla ensanchado `anchoCalle` celdas por cada lado (su anillo de calle).
   - Se barren candidatos alrededor del ancla (radio de escaneo = `FONDO_MANZANA + max(lado ancla) + max(lado satélite)` celdas, `distanciaMinima = 0`).
   - `hueco` = `gapCeldas(candidato, rectConAnillo)` (Chebyshev entre bordes; 0 si se tocan o solapan).
   - **Núcleo:** solo valen los candidatos con `hueco ≤ FONDO_MANZANA` (= 2): el anillo de calle más dos hileras de satélites espalda con espalda.
   - Se ordenan **lexicográficamente** por el orden del perfil (§8) sobre estas claves (si ganan al ser menores, `hueco`, `nivel`, `centro`; si al ser mayores, `borde`, `bordeAfin`) y de último `semillaCandidato`:
     - `hueco`: celdas hasta el ancla. · `nivel`: el de §6.2. · `borde`: `bordeCompartido` con `rectConAnillo` (largo de lado compartido; 0 si solo se tocan en diagonal; solo discrimina con hueco 0).
     - `bordeAfin`: celdas de `celdasAfines` ortogonalmente adyacentes al perímetro del candidato. · `centro`: distancia del punto al origen.
   - Vacío = el núcleo está geométricamente lleno.
4. Preferencia intramuros (§6.6).

El **mercado** y sus **puestos** son un caso de §6.5 con ancla = `mercado`: los puestos comparten categoría con él y se agrupan a su alrededor sin ninguna regla extra.

### 6.6 Preferencia intramuros

Si hay un **recinto exterior completo** (`integridad ≥ 1`, recorriendo desde el último), los candidatos **urbanos** (todo salvo granja y corral) cuya huella queda entera dentro de su interior van antes que los demás. Es una **partición estable**
(el orden que traían no se toca dentro de cada grupo) y no un criterio más del orden lexicográfico, para no romper los perfiles. El interior es la inundación 4-conexa desde una celda del Centro Urbano que no cruza ninguna celda del
anillo (puertas incluidas). Sin recinto completo, no hace nada.

### 6.7 Reubicación de un edificio que crece (`reubicarPorTamano`)

Hoy solo la granja al mejorar. Se busca un sitio para la huella nueva con el suelo ocupado **sin el propio edificio**, con las reglas de afueras si procede. Entre los candidatos:
1. los que **no acercan** su borde interior al centro respecto al actual, y de ellos el **más cercano a la posición actual**;
2. si ninguno cumple, el más cercano a la posición actual (la mejora manda sobre la dirección);
3. `null` solo si no cabe en ningún sitio (entonces la mejora no se aplica).

## 7. Las anclas

### 7.1 Qué son y cómo se garantizan

Una **ancla** es un edificio que da frente de calle a sus satélites: siembra el anillo completo de calle a su alrededor (§5.1).

- `ANCLA_PRIMARIA_POR_CATEGORIA`: `residencial → centroUrbano`, `mercado → mercado`.
- `ANCLA_SATURACION_POR_CATEGORIA`: `residencial → [plaza, pozo, parque]`, `militar → [plazaDeArmas]`, `industria → [patioDeGremios]`.
- `ANCLAS_REALES` = la unión de las dos tablas. Los marcadores no se construyen ni cuestan: nacen «gratis» sin pasar por la cola.

`asegurarAnclaPara(asentamiento, edificios, tipo, nextId)` (en `construction.ts`) corre **antes** de pedir sitio para un edificio con categoría:

```
categoria := CATEGORIA_POR_TIPO[tipo]; si no hay → nada
(instancia, llenas) := anclaActivaParaCategoria(...)       // recorre las instancias no llenas de más cerca a más lejos del origen y
                                                           // prueba sitiosPorAtraccionDura; la primera con hueco gana; las que se
                                                           // prueban y no sirven se devuelven como "recién llenas"
marcar anclaLlena = true en las llenas
si instancia ≠ null → fin
si nivelActual < NIVEL_GATE_ANCLAS (= 2) → fin
tipoAncla := tipoAnclaParaCategoria(categoria, idNuevo)    // sorteo determinista entre las de saturación; null para mercado
resultado := crearAnclaNueva(...)                           // §7.2
si null → fin
marcar semillaSaturada = true en las recién saturadas
GATE de utilidad: si sitiosPorAtraccionDura(nuevaAncla, …) está vacío → NO se añade el ancla (se conservan las semillas saturadas) y se reintenta en la siguiente reevaluación
si no → añadir la nueva ancla
```

### 7.2 El árbol único de anclas (`crearAnclaNueva`)

Todas las anclas, de cualquier tipo, forman **un solo árbol** que crece desde el Centro Urbano.

```
semilla := semillaActiva(edificios, excluidas)   // el ancla real no semillaSaturada y no excluida más cercana al origen
mientras haya semilla:
    otras := rectángulos de las demás anclas reales
    direcciones := 5 ranuras equidistantes (72°), la primera hacia el norte (−y) más la rotación del asentamiento, barajadas por semilla
    libres := las que no tienen ya un ancla encima (ranuraOcupada); usadas := el resto
    para cada dirección en libres ++ usadas:
        para cada orientación del ancla (normal/girada, orden sembrado):
            rect := huecoEnDireccion(centro de la semilla, dirección, tamano, ocupadas, otras)
            si rect ≠ null Y (tiene frente de calle O hay corredor hasta la red dentro de capCorredorUrbano):
                devolver { nuevaAncla en el centro de rect, rotado }
    excluidas += semilla; semilla := semillaActiva(...)
devolver null
```

- `huecoEnDireccion`: prueba radios de `radioInicialRanura() = separacionMinimaAnclas` (6) a `radioMaximoRanura() = 3 × separacionMinimaAnclas` (18) celdas, de `FONDO_MANZANA` en `FONDO_MANZANA`; el rectángulo centrado en
  `origen + dirección × radio × T` debe estar libre y a `gapCeldas ≥ separacionSeguridadAnclas` (3, **piso duro**) de **todas** las demás anclas.
- `ranuraOcupada(centroSemilla, dirección, otras)`: hay un ancla a una distancia entre `radioInicialRanura − 2` y `radioMaximoRanura + 2` celdas y con un ángulo a menos de `0,55 × media ranura` de la dirección.
- Un ancla recién creada con el **mercado** como tipo usa este mismo camino (es su propia ancla primaria).

## 8. Perfiles de trazado

El perfil es una **permutación del orden de desempate** de los satélites, no una plantilla: las reglas son las mismas, cambia cuál criterio manda.

| Perfil | Orden de las claves |
|---|---|
| `nucleos` (histórico, por defecto) | hueco, nivel, borde, bordeAfin, centro |
| `caminera` | nivel, hueco, borde, bordeAfin, centro |
| `compacta` | centro, hueco, nivel, borde, bordeAfin |
| `gremial` | bordeAfin, hueco, nivel, borde, centro |

Precedencia: `TRAZADO.perfilForzado` (solo laboratorio y batch) > perfil de la política activa (`perfilTrazadoDePolitica`; hoy «Postura Defensiva» → `compacta`) > `perfilPorTradicion(id)`.
Un quinto perfil («palatina», dominado por `borde`) se descartó: `borde` solo tiene señal con hueco 0 y salía idéntico a `nucleos`.

## 9. Cachés (no cambian ningún resultado)

Dos cachés LRU de 512 entradas (`Map`, se reinserta al acertar y se desaloja la primera):

- **Red de calles**, clave = `id` + por cada edificio `tipo:x,y:nivelInterno:rotado:ambito` + por cada recinto sus celdas `col,row,clase`. **No** incluye `estado` ni `completaEn`.
- **Sitios**, clave = la anterior + por recinto `id:nivel:avance:mejorandoA` + `radioCiudad` + `tipo` + `nivelInterno` + `perfil`. Incluye `avance` porque la preferencia intramuros solo se aplica con el recinto completo.

La clave es el contenido exacto, no un hash: una colisión daría una red equivocada en silencio. Ante la duda, la clave incluye de más (cuesta aciertos) y nunca de menos (daría una respuesta errónea).

## 10. Salida de dibujo

`trazadoParaAsentamiento` devuelve para el cliente, en coordenadas locales (el cliente no ve celdas): `calles` y `caminos` (rectángulos), `huellas` por id, y `murallas`.
`fusionarCeldas` agrupa cada fila en tiradas horizontales (una avenida de 20 celdas viaja como un rectángulo): ordena filas y columnas de menor a mayor y cada tirada es `{ x: inicio·T, y: row·T, ancho: n·T, alto: T }`.

## 11. Fundación

`edificiosIniciales`: el Centro Urbano en `(0,0)`; `FUNDACION.viviendasIniciales` (3) viviendas colocadas con `sitioEnBarrio` (cada una ve las anteriores); la granja inicial con `sitioEnBarrio` (si no cabe, **cae al
origen** `(0,0)`, encima del Centro Urbano: el fallback es defensivo); contexto de colocación `{ id, radioCiudad: TRAZADO.radioCiudad.inicial }`.

## 12. Parámetros

<!-- parametros:REJILLA_ASENTAMIENTO -->
```json
{
  "tamanoCelda": 3,
  "radioMapa": 220
}
```
<!-- /parametros -->

<!-- parametros:LAYOUT_VERSION -->
```json
2
```
<!-- /parametros -->

<!-- parametros:TRAZADO -->
```json
{
  "perfilForzado": null,
  "largoFilaMin": 4,
  "largoFilaMax": 8,
  "radioAfuerasMin": 60,
  "anchoBandaAfueras": 36,
  "separacionMinimaAnclas": 6,
  "separacionSeguridadAnclas": 3,
  "anchoCalle": 1,
  "capCorredorUrbano": 12,
  "capCorredorAfueras": 200
}
```
<!-- /parametros -->

<!-- parametros:PERFILES_TRAZADO -->
```json
[
  "nucleos",
  "caminera",
  "compacta",
  "gremial"
]
```
<!-- /parametros -->

`TRAZADO.radioAfuerasMin` y `anchoBandaAfueras` están en **unidades locales** (no en celdas); el resto de distancias de este bloque, en celdas. `FONDO_MANZANA = 2` y `NIVEL_GATE_ANCLAS = 2` son constantes locales de
`trazado.ts` y `construction.ts`, no están en `constants.ts`.

## 13. Invariantes

| Invariante | Dónde se vigila |
|---|---|
| Un edificio nunca ocupa una celda de otro, de calle, de camino ni de muro/torre | `ocupadasConRed` + `candidatosLibres` |
| Todo sitio válido tiene salida a la red dentro del cap (gate duro) | `escalaRejilla.test.ts`, `trazado*.test.ts` |
| La red es un único componente conexo (sin islas de calle) | `añadirConectadas`; tests de conectividad |
| Mismo input → misma red y mismo orden de sitios | caché y tests de determinismo |
| La separación mínima entre anclas reales es `separacionSeguridadAnclas` (borde a borde) | `huecoEnDireccion` |
| Un recinto cerrado no parte la red: las puertas la mantienen conexa | `muralla.test.ts` |
| El área encerrada por un recinto nunca decrece al ampliar | [`Murallas.md`](Murallas.md) §8 |

## 14. Decisiones que parecen raras

- **Las anclas tienen un anillo de calle**, así que un satélite nunca *toca* al ancla: el hueco se mide contra el ancla *con su anillo*. Medir contra el ancla desnuda hacía imposible `hueco 0`.
- **`anclaLlena` y `semillaSaturada` son latches permanentes** (nada libera celdas). Por eso un ancla solo se marca llena cuando su banda no tiene ni un hueco geométrico, aunque no quede ninguno con frente de calle.
- **`nextId()` se consume por candidato propuesto** en la auto-construcción, aunque luego no se comprometa; por eso la búsqueda de sitio no puede filtrarse por «¿puedo pagarlo?» sin cambiar el comportamiento.
- **Una granja envuelta por la ciudad** queda intramuros: ya no son las afueras (ver [`Murallas.md`](Murallas.md) §3).
