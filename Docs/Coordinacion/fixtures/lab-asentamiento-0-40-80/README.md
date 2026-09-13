# Fixture — asentamiento del laboratorio (`lab/`), con murallas

Segunda versión del fixture de reconstrucción, generada con el **laboratorio de trazado** (`lab/`) en vez de
una partida HTTP real — a petición del usuario, para poder moldear la ciudad a mano (incluida una muralla
completa, que la partida real usada en el primer fixture no tenía) y exportarla cuando se vea bien en
pantalla.

**Diferencia de procedencia con el fixture anterior (`../asentamiento-0-1108-1328/`) — léela antes de usar
este archivo:** aquel era el cuerpo EXACTO de una respuesta HTTP real de `GET /jugador/partidas/:gameId`.
Este NO lo es — el laboratorio (`scripts/lab.ts`, cabecera del archivo: "el motor real corriendo en el
navegador, sin API, sin partida y sin red") no tiene servidor. Lo que sí comparte con una respuesta real:
`asentamientos[0]` es el objeto `Asentamiento` real del motor, y `trazadoPorAsentamiento` sale de
`trazadoParaAsentamiento()`, la MISMA función que usa `session/proyecciones/jugador.ts` para servir el campo
homónimo por HTTP (`lab/src/main.ts`, comentario de `computar()`: "el trazado llega YA RESUELTO del motor
— el mismo `trazadoParaAsentamiento` que sirve el backend por HTTP"). Los datos de edificios/huellas/
calles/caminos/murallas son de fiar con la misma fidelidad que el fixture anterior. Los campos de
CONTEXTO (`gameId`, `jugadorId`, `faccionId`, `version`, `instante`) son sintéticos del laboratorio, no de
una sesión real — marcados así explícitamente en el propio JSON (`_origen`).

## Qué hay en esta carpeta

- `proyeccion_lab.json` — export real del laboratorio (botón "Exportar proyección", o
  `window.__lab.exportarProyeccion()` desde la consola). Mismo criterio de formato que el fixture anterior:
  indentado a 2 espacios para lectura humana, ningún valor tocado a mano.
- `vista_cenital.png` — `document.getElementById('lab-canvas').toDataURL('image/png')`, igual que el
  fixture anterior: el bitmap real del canvas, no una captura de pantalla recortada.
- Este `README.md`.

## Cómo se generó (reproducible, sin depender de ninguna partida guardada)

1. `npm run lab` (o el lanzador `lab-trazado` de `.claude/launch.json`, puerto 5180) — abre
   `http://localhost:5180`.
2. "Fundar" con la seed por defecto (1). El asentamiento nace con materiales infinitos (`darMaterialesInfinitos`)
   y una Facción a nivel máximo, para que ningún cupo de partida frene el crecimiento urbano.
3. "+50" repetido (unos 850 ticks en total) hasta alcanzar nivel 2 con ~76 edificios — el crecimiento
   automático se estanca ahí porque el gate de nivel 3 pide tipos que el auto-constructor no prioriza solo.
4. Pestaña "Construcción manual": encolar a mano `granFundicion`, `barracon`, `galeriaDeTiro`, `mercado`,
   `carpinteria` (uno por uno, con materiales infinitos no hace falta esperar entre cada uno). Avanzar unos
   ticks más — el asentamiento sube a **nivel 3** y el auto-constructor añade además `patioDeGremios`,
   `plazaDeArmas`, `puestoMercado`, `tallerCarpinteria` (anclas/satélites gratis que dispara el propio
   crecimiento).
5. "Trazar recinto" (nivel 2, muro de piedra) → "Comprometer" → avanzar ticks hasta que el estado de muralla
   marque "100% (cerrado)". Esto es la mecánica real del juego (`comprometerRecinto`/`avanzarMuralla`), no
   un atajo del laboratorio — el único atajo que el lab ofrece aparte ("Quitar murallas") no se usó aquí.
6. Botón **"Exportar proyección"** (añadido en esta sesión, ver "Cambio de código" abajo) → descarga
   `lab-proyeccion-<asentamientoId>-tick<N>.json`. Para esta captura se leyó en su lugar
   `window.__lab.exportarProyeccion()` desde la consola (mismo objeto que construye el botón, sin pasar por
   la descarga del navegador).

## Cambio de código hecho para esto

Se añadió a `lab/` (sin tocar ningún otro dominio ni el backend):

- `lab/index.html`: botón `#lab-exportar` ("Exportar proyección"), junto a los controles de tick.
- `lab/src/main.ts`: `construirProyeccionLab()` (envuelve `asentamiento`+`trazado` reales en la misma forma
  que `ProyeccionJugador.asentamientos`/`trazadoPorAsentamiento`) y `exportarProyeccion()` (descarga vía
  `Blob`+`<a download>`, mismo patrón que el botón "Exportar" ya existente en `cliente/src/main.ts`).
  También se expuso como `window.__lab.exportarProyeccion()` para poder pedirlo desde la consola/automatización
  sin simular un click — mismo criterio que el resto de `window.__lab` (gancho de depuración documentado en
  `publicarEnConsola()`).

## Qué building types y estructura trae este ejemplo concreto

- **126 edificios, 21 tipos distintos**, nivel 3 (más tipos y un nivel más que el fixture anterior, que
  tenía 75/12/nivel 2): `centroUrbano`, `vivienda`, `granja`, `lenera`, `granero`, `almacen`, `corral`,
  `cantera`, `patioDeGremios`, `curtiduria`, `armeria`, `plaza`, `fundicion`, `granFundicion`,
  `plazaDeArmas`, `barracon`, `galeriaDeTiro`, `mercado`, `carpinteria`, `puestoMercado`,
  `tallerCarpinteria`.
- `trazadoPorAsentamiento['asentamiento-0-40-80']`: 124 huellas, 138 rectángulos de `calles`, 58 de
  `caminos`.
- **Murallas: SÍ, completas.** `Asentamiento.recintos[0]`: nivel 2, 191 celdas, `avance: 190` (cerrado —
  igual al índice de la última celda, `celdas.length - 1`). `trazadoPorAsentamiento[...].murallas[0]`: 102
  piezas de `muro`, 9 `puertas`, 17 `torres` (fusionadas en rectángulos, por eso 102 ≠ 191: varias celdas de
  muro contiguas se fusionan en un solo `RectanguloLocal`, igual que hacen `calles`/`caminos` — ver
  `fusionarCeldas`, `engine/trazado.ts`). Mismo formato `{x,y,ancho,alto}` con ancla de ESQUINA que las
  huellas de edificios (no de centro) — ver la sección de unidades abajo.

## Unidades, coordenadas y rotación — idéntico al fixture anterior, reconfirmado con datos nuevos

Todo lo documentado en `../asentamiento-0-1108-1328/README.md` (celda = 3 unidades locales, origen = centro
del Centro Urbano, `y` crece hacia abajo en pantalla, `Edificio.posicion` ancla por CENTRO mientras que
`huellas`/`calles`/`caminos`/`murallas.*` anclan por ESQUINA, rotación YA incorporada en `ancho`/`alto` de
las huellas) sigue valiendo tal cual — son las mismas funciones del motor, no una reimplementación para el
laboratorio. Reconfirmado con un ejemplo real de ESTE fixture: `edificio-asentamiento-0-40-80-53` es una
`armeria` con `rotado: true`; su huella sale `{ancho:18, alto:12}` — el tamaño base de una armería es
`{ancho:4, alto:6}` celdas × 3 unidades/celda = `{12,18}` sin rotar, y `{18,12}` ya intercambiado. No hay que
rotarla otra vez en Unity.

`Asentamiento.posicion` de este ejemplo es `{x:40, y:80}` — sigue siendo coordenada de MAPA GENERAL (donde
está la ciudad en el mundo), un espacio TOTALMENTE DISTINTO del espacio local de `huellas`/`calles`/
`caminos`/`murallas` (donde está cada pieza DENTRO de la ciudad). No mezclar los dos, igual que en el
fixture anterior.

## Campos del modelo compartido acordado que NO aparecen — confirmado igual que antes

`heroeId`, `Edificio.visualSeed`, `Asentamiento.visualCatalogVersion`, `Asentamiento.layoutVersion`, id
propio de `CeldaMuro` (`enclosureId`+índice) — confirmado ausente en este JSON también (no son campos que
dependan de venir de una partida real vs. del laboratorio: sencillamente no existen todavía en
`domain/types.ts`/`engine/trazado.ts`, así que ninguna fuente los tiene). Ver
`Docs/Coordinacion/01_Modelo_de_datos_compartido.md` §17 para el estado de cada uno.

## Contexto sintético — qué NO tomar como real de este JSON

`gameId: "lab"`, `jugadorId: "jugador-lab"`, `faccionId: "faccion-lab"`, `version`/`instante` derivados del
contador de tick del laboratorio (no de un reloj de partida real) — todos marcados por el campo `_origen`
del propio JSON. Si se necesita un `gameId`/`jugadorId` que de verdad resuelvan contra un backend en marcha,
hay que usar el fixture HTTP anterior (`../asentamiento-0-1108-1328/`), no este.
