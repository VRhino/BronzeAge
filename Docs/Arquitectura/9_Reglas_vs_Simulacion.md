# Reglas vs. simulación: qué puede salir del servidor y qué no

Referencia de clasificación de **todo** el motor. Se escribe en la Fase C (2026-08-26) porque el cliente pasó
a vivir en otro repositorio y hablar solo por red: a partir de ahí, cada función del motor tiene que estar en
un lado del cable o en el otro, y hasta ahora no existía ningún criterio escrito para decidirlo.

El eje **no** es "¿el cliente tiene motor, sí o no?". Ese planteamiento —heredado del desarrollo de
aplicaciones web— es demasiado grueso para un videojuego: un *tooltip* que necesita un viaje de red es
inusable, así que algo del dominio tiene que vivir en el cliente. El eje correcto es **reglas vs. simulación**,
y lo explica [6_Sincronizacion_Visibilidad_y_Escala.md §6](6_Sincronizacion_Visibilidad_y_Escala.md#6-modelo-de-sincronización-reglas-al-cliente-simulación-en-el-servidor)
con las fuentes de la industria (el patrón es el *Static Data Export* de EVE Online).

Este documento es la aplicación de ese criterio, módulo por módulo. **Es una referencia, no un plan**: los
hitos que lo ejecutan son C7 (servir el balance), C10 (consultas derivadas) y C11 (el mapa como asset).

---

## El criterio, y el eje que casi se nos escapa

Tres categorías por **naturaleza**:

| | Qué es | Marca reconocible en el código |
|---|---|---|
| **T1 · Regla-dato** | Tabla. Datos puros, sin comportamiento | `constants.ts` |
| **T2 · Regla-función** | Derivación pura: entidades + tablas → un número o una forma. No muta, no consume RNG | `calcular*`, `compute*`, `*Info`, predicados |
| **T3 · Simulación** | Muta estado, lo avanza o consume RNG | `avanzar*`, los 30 comandos, `worldgen/` |

Pero la naturaleza **no basta para decidir de qué lado del cable va una función**. Hay un segundo eje,
independiente y menos evidente:

> **T2 no implica "puede calcularlo el cliente".** Una función puede ser perfectamente pura y aun así ser
> **solo de servidor**, porque su ENTRADA es privilegiada.

`calcularPrecioReferencia(recurso, asentamientos)` no muta nada y no toca el RNG — pero suma el stock de
**todos** los asentamientos del mundo, incluidos los rivales. Para que el cliente la calculara habría que
mandarle el almacén de cada rival, que es exactamente la fuga que las proyecciones de C4 existen para evitar.

Por eso cada entrada de T2 lleva abajo una columna **"entrada"**:

- **propia** — solo necesita entidades que el jugador ya ve en su proyección. Puede calcularse en el cliente.
- **privilegiada** — necesita estado ajeno. **Solo servidor**, y lo que viaja es el resultado, no la entrada.

### Corolario incómodo, y no lo resuelve C10

Un valor derivado de una entrada privilegiada **filtra información sobre esa entrada**, se entregue como se
entregue. El aviso de viabilidad al fundar (`evaluarViabilidadFundacion`) necesita las posiciones de todos los
asentamientos para comprobar separación: mandar el resultado en vez de la entrada reduce la fuga, no la
elimina — de dónde *no* puedes fundar se deduce dónde hay alguien.

Es una decisión de diseño de niebla de guerra (**C4 Slice 2**), no un problema de transporte. Queda anotado
aquí para que no se cuele como resuelto.

---

## T1 · Regla-dato — `src/constants.ts`

39 tablas, 1251 líneas. **Todas son candidatas a viajar al cliente** en cuanto exista C7 (balance versionado
y servido). Es lo que hoy obliga a `cliente/src/app/gameStore.ts` a importar 8 módulos de `constants` para
poder pintar un formulario.

Por consumidor de interfaz:

| Grupo | Tablas | Lo usa la interfaz para |
|---|---|---|
| **Catálogos** | `EDIFICIO_CATALOGO`, `POLITICA_CATALOGO`, `TROPAS_RECLUTABLES`, `CARAVANA_CATALOGO` | Construir los formularios: qué se puede construir, activar, reclutar |
| **Cupos y niveles** | `NIVEL_FACCION`, `NIVEL_ASENTAMIENTO`, `CAP_FUNDACION_POR_NIVEL`, `CUPO_NIVEL_ASENTAMIENTO`, `POLITICAS`, `CIUDADANIA` | Barras de progreso, "te faltan N para subir" |
| **Costes y economía** | `MANTENIMIENTO`, `ALMACEN`, `NECESIDADES`, `PRECIO_BASE`, `PRECIO_REFERENCIA`, `COMISION`, `TRUEQUE`, `RESERVA_CONSTRUCCION` | Mostrar el coste antes de confirmar |
| **Geometría urbana** | `REJILLA_ASENTAMIENTO`, `EDIFICIO_TAMANO`, `TRAZADO`, `SITIO`, `PUESTO_MERCADO_FORMA`, `MERCADO_PUESTOS_POR_NIVEL` | Dibujar la vista urbana |
| **Mundo y militar** | `ZONA_INFLUENCIA`, `FUNDACION`, `POBLACION`, `MILITAR`, `CHOKEPOINTS_PEAJE`, `LENERA_POR_BOSQUE` | Leyendas, radios, previsualizaciones |

**Tres tablas a revisar antes de publicarlas**, no por ser secretas sino porque nadie lo ha decidido:

- `CAMPAMENTOS_BANDIDOS` y `REGENERACION_NODOS` — parámetros de aparición y regeneración. En la mayoría de
  juegos de estrategia esto acaba en una wiki de todas formas, pero publicarlo es una decisión de diseño.
- `SCORE_BANDAS`, `EXTRACTOR_DESEMPATE`, `LINEAS_PRODUCCION`, `EXTRACCION_MAXIMOS` — heurísticas internas de
  colocación automática. El cliente no las necesita para nada.
- `SIMULACION_AUTO_COMERCIO` — herramienta de simulación de desarrollo, apagada por defecto. No es balance de
  juego; no debe salir.

> Este es literalmente el problema que CCP describe al rehacer el *Static Data Export* de EVE: *"había que
> tener extremo cuidado sobre qué datos iban al servidor y cuáles al cliente, para no filtrar información que
> no debía ser pública"*. La estructura de un solo `constants.ts` que sirve a los dos lados es justo la que
> lo hace fácil de equivocar.

---

## T2 · Regla-función — derivaciones puras

### T2a · Entrada propia → el cliente puede calcularlas

Solo necesitan entidades que el jugador ya tiene en su proyección. Sirviendo T1 (C7), el cliente las resuelve
sin viaje de red.

| Función | Módulo | Deriva de |
|---|---|---|
| `calcularCapFundacion`, `calcularCupoNivel`, `calcularNivelFaccion` | `engine/faccion` | Nivel de la Facción propia + tablas |
| `capacidadCasas`, `esCiudadano` | `engine/faccion` | Facción propia |
| `nivelActualDe`, `progresoNivelAsentamiento`, `capacidadVivienda*`, `poblacionTotal`, `edificiosPorTipoYEstado`, `tieneMercadoActivo`, `cupoCaravanas`, `ticksCooldownCaravanaRestantes`, `ratioManoObra*`, `manoObraInfo` | `engine/asentamientoQuery` | **Un** asentamiento propio |
| `calcularNivelAsentamiento`, `calcularCostoMantenimiento`, `encontrarCapital` | `engine/mantenimiento` | Asentamientos propios |
| `poderEscuadron` | `engine/combate` | **Un** escuadrón propio + tick |
| `consumoComidaPoblacion` | `engine/population` | Un asentamiento |
| `consumoRacionTropas`, `poblacionTotalConTropas` | `engine/tropas` | Un asentamiento |
| `slotsDisponibles` y los 10 `factor*` | `engine/politicas` | Cargo + nivel + políticas activas propias |
| `esResidente`, `tieneCargoLocal`, `esReyDe`, `esReyOEmbajadorDe`, `cargoOcupado` | `engine/pertenencia` | Asentamiento/Facción propios |
| `tieneRecursos`, `cantidadDisponible` | `engine/almacen` | Almacén propio |
| `estadoMejoraEdificio`, `factorLineaProduccion`, `maximoViviendasPorNivel`, `alcanzoTopeDe*` | `engine/construction` | Asentamiento propio |
| `computeLigas` | `engine/liga` | Relaciones + Facciones — **ya públicas** en la proyección |
| `buscarCamino` | `engine/caminos` | Busca en `caminos`, **ya público** en la proyección (no recorre el mapa: es un `find` sobre el array) |
| `calcularRuta` | `world/rutas` | Pathfinding sobre el terreno, que lo ven todos |
| `narrarCambiosDeTitulo` | `engine/titulos` | Títulos + Facciones, ambos públicos. Vive en el servidor porque este posee el log de eventos, **no** por privilegio |
| `edificiosInternos`, `celdasDeEdificio`, `redDeCalles`, `segmentosDeRed`, `tamanoDeEdificio`, `celdaMinimaDeEdificio` y el resto de `engine/trazado` | `engine/trazado` | Los edificios de **un** asentamiento propio |
| Todo `world/geometria`, `world/poligonos` | — | Geometría pura, sin estado de juego |

**Precio de esta columna**: si el cliente las calcula, la fórmula existe dos veces (servidor por autoridad,
cliente por presentación) y pueden divergir. Se acepta a conciencia —la alternativa es un viaje de red por
*tooltip*— y se mitiga manteniendo la regla lo más "tabla pura" posible, para que el cliente haga *lookup* y
no reimplemente lógica.

### T2b · Entrada privilegiada → solo servidor, aunque sean puras

Necesitan estado que el jugador no debe ver. Lo que viaja es **el resultado ya calculado**, nunca la entrada.

| Función | Módulo | Por qué es privilegiada |
|---|---|---|
| `calcularPrecioReferencia` | `engine/market` | Suma el stock de **todos** los asentamientos del mundo |
| `computeZonaInfluencia`, `computeTodasLasZonas`, `computeZonasFusionadasPorFaccion` | `engine/zones` | Zona de cada asentamiento **contra todos los demás** |
| `controladorDeChokepoint`, `chokepointsDePeajeEnRuta` | `engine/chokepoints` | Necesita las zonas de todos |
| `evaluarViabilidadFundacion` | `engine/settlement` | Comprueba separación **contra todos** los asentamientos |
| `posicionLibreParaFundar`, `mejorFertilidadEnZona` | `engine/zones` | Ídem |
| `calcularTitulos` | `engine/titulos` | Ranking global: recorre **todos** los asentamientos, que sí se filtran |

**Las cuatro de geometría son además las que `render()` pide en cada `mousemove`** (`getZonasFusionadas`,
`chokepointsControl`, `getTrazadoAsentamiento`, `viabilidadFundacion`). No es coincidencia: son consultas
espaciales de ámbito mundial. Eso las deja atrapadas entre dos exigencias —**no pueden ser un endpoint**
(latencia) y **no pueden calcularse en el cliente** (visibilidad)— y solo hay una salida: viajar
**precalculadas dentro de la proyección**, que solo cambia por tick. Es el grupo (c) del hito C10.

---

## T3 · Simulación — solo servidor, sin excepciones

Nunca sale del backend. Ni como código, ni como datos, ni "solo para predecir": ver
[§6.2 del doc 6](6_Sincronizacion_Visibilidad_y_Escala.md#62-por-qué-lockstep-queda-descartado-aquí) para por
qué se descartó el modelo *lockstep* que sí lo haría.

### T3a · El tick

Todo lo que empieza por `avanzar*`, más `engine/simulation.ts` como orquestador:

`avanzarSimulacion` · `avanzarConstruccion` · `avanzarMantenimiento` · `avanzarNivelAsentamiento` ·
`avanzarNivelesFaccion` · `avanzarNutricionPoblacion` · `crecerPoblacion` · `avanzarComercio` ·
`avanzarMercado` · `avanzarTributos` · `avanzarPoliticas` · `avanzarReputacion` · `avanzarCaravanasFundacion` ·
`avanzarSpawnBandidos` · `avanzarAtaquesBandidos` · `avanzarMantenimientoTropas` · `avanzarPosicionEnRuta` ·
`avanzarAutoComercioSimulado` · `Mapa.avanzarRegeneracion`

Cuatro de ellos consumen RNG (`engine/simulation`, `engine/combate`, `engine/population`, `engine/bandidos`) —
lo que hace su reproducción en el cliente imposible sin compartir también el estado del RNG, que hoy ni
siquiera es serializable (A3, diferido a B3).

### T3b · Los comandos

Los 30 de `session/comandos/registro.ts`. El cliente manda la **intención** (`{tipo, params}`) y recibe el
resultado; nunca los ejecuta. Sus implementaciones de motor:

`fundarAsentamiento` · `crearFaccion` · `otorgarCiudadania` · `comprarCasa` · `asignarRey` ·
`asignarEmbajador` · `asignarCargoLocal` · `liberarCargoLocal` · `activarPolitica` ·
`anadirEdificioManualmente` · `quitarDeCola` · `moverEnCola` · `mejorarEdificioManualmente` ·
`proponerVasallaje` · `proponerAlianza` · `romperRelacion` · `rebelionVasallo` · `anexionar` · `fusionar` ·
`proponerTrueque` · `colocarOrdenMercado` · `construirCaravanaComercial` · `lanzarCaravanaFundacion` ·
`desarmarCaravanaFundacion` · `reclutarTropa` · `resolverCombate` · `iniciarAsedio` · `combateCampoAbierto` ·
`interceptarCaravana` · `atacarCampamentoBandidos` · `descontarRecursos` / `agregarRecurso` ·
`Mapa.extraer` · `ajustarReputacion` · `asegurarCaminoComercial`

### T3c · `worldgen/` — el caso especial que resuelve C11

Los 15 módulos de `src/worldgen/` son simulación (consumen RNG, y **el orden de consumo es parte del
contrato**: añadir una llamada desplaza todo el mundo generado). Pero tienen una propiedad que ninguna otra
simulación de este proyecto tiene:

> Su salida es **función pura de la seed**. Determinista, reproducible, e **inmutable durante toda la partida**.

Eso convierte al mapa en un **asset**, no en estado — medido: 125,4 KB idénticos byte a byte en los ticks 0,
50 y 200 ([§6.4 del doc 6](6_Sincronizacion_Visibilidad_y_Escala.md#64-medición-qué-viaja-hoy-de-verdad)).
El cliente no necesita `worldgen/` **ni** que se lo manden en cada respuesta: necesita el resultado servido
una vez y cacheado por `seed`+`worldgenVersion`. Ese es el hito **C11**.

Ojo con el matiz que hoy lo impide: `MapaGenerado.elevacion` y `.fertilidad` **no son rásteres**, son
*parámetros de ruido*, y el bioma no se guarda — se evalúa por píxel con `evaluarBioma`. Servir el
`MapaGenerado` tal cual no basta: hay que rasterizar. La rasterización es igual de determinista por seed, así
que se cachea igual.

---

## Zona gris: `world/mapa.ts`

`Mapa` es una fachada de consulta sobre `MapaGenerado` + `EstadoMapa`, y **está a los dos lados de la línea**:

- **T2a** (consulta pura, entrada no privilegiada — el terreno lo ven todos): `elevacionEn`, `biomaEn`,
  `terrenoEn`, `fertilidadEn`, `costeEnPunto`, `listarRios`, `listarBosques`, `listarNodos`,
  `listarChokepoints`, `contornosBosques`, `dentroDelMapa`, `nodosEnRadio`, `nodosEnPoligono`.
- **T3** (muta el estado de la partida): `extraer` (agota un yacimiento) y `avanzarRegeneracion`.

Consecuencia para C11: lo que el cliente necesita de `Mapa` es la **capa de consulta de terreno**, y eso se
sustituye por el ráster servido. `EstadoMapa` (lo consumido de cada nodo) sí es estado de partida y sigue
viajando en la proyección — hoy pesa 0,3 KB al tick 200, es irrelevante.

---

## Fuera de juego: administración y desarrollo

No entran en la clasificación porque no son ni regla ni simulación de partida:

- `world/exportUnity.ts` (`exportarParaUnityTerrain`) — herramienta de *worldgen*, no del juego. El
  [doc 8](8_Triaje_Consultas.md) ya la marcaba como endpoint de administración; hoy corre en el navegador.
- `GameStore.exportarSimulacion` — serializa la partida a un archivo. Administración (**C12**).
- `engine/simulacionAutoComercio.ts` — simulación de desarrollo, apagada por defecto.

---

## Cómo usar esta referencia

Al añadir una función al motor, dos preguntas en este orden:

1. **¿Muta, avanza o consume RNG?** → T3. Servidor. Fin.
2. **Si no: ¿qué necesita como entrada?** Solo entidades que el jugador ya ve → T2a, el cliente puede
   calcularla. Estado ajeno → T2b, el servidor manda el resultado.

Y una advertencia que ya costó una premisa falsa en C0: **que una función sea pura no significa que pueda
salir del servidor.** La pureza dice que es segura de *reejecutar*; no dice nada sobre quién puede ver sus
entradas.

La simétrica también muerde, y pasó al escribir este documento: **el nombre no basta para clasificar, hay que
leer la firma.** `buscarCamino` sonaba a recorrer el mapa (T2b) y resultó ser un `find` sobre un array que la
proyección ya publica entero (T2a). `narrarCambiosDeTitulo` parecía privilegiada por vecindad con
`calcularTitulos` —que sí lo es— y opera sobre datos públicos. Ambas estaban mal clasificadas en el primer
borrador. **La pregunta 2 se contesta mirando los parámetros y comprobando cuáles de ellos filtra
`proyectarParaJugador`**, no por intuición.
