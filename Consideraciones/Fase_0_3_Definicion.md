# Definición de Fase 0.3 — Movimiento con terreno, caminos comerciales y chokepoints

Sub-fase dentro de Fase 0 (sigue siendo datos puros, mapa 2D top-down, sin Unity ni 3D — ver
`Fase_0_Definicion.md`). Cierra dos mecánicas de diseño que estaban CERRADAS pero sin implementar (Doc
`1_Sistema_de_Mundo_y_Territorio.md`, secciones 1.5 y 1.6) ahora que Fase 0.1 dejó el mundo con relieve
real: coste de movimiento sensible al terreno, caminos comerciales automáticos, y chokepoints estratégicos
con un primer efecto mecánico (control por zona de influencia + peaje en oro).

## Por qué ahora

`Fase_0_1_Definicion.md` dejó "coste de movimiento / pathfinding" explícitamente fuera de alcance,
reservado para "cuando llegue movimiento militar" — pero ya adelantaba cómo debía construirse: como función
continua de la pendiente, generalizando de cómo las caravanas ya interpolaban (`progreso` 0→1) sobre una
línea, y con chokepoints como geometría (puntos/zonas), nunca como grafo de celdas. Doc 1.5 marcaba
chokepoints como "NO IMPLEMENTADO EN FASE 0 (terreno plano)" — esa condición ya no aplica: el terreno tiene
relieve real desde Fase 0.1.

## Decisión de arquitectura

Mismo criterio que 0.1/0.2: campos continuos + geometría vectorial, nunca rejilla horneada ni tiles
guardados en `MapaGenerado`.

- **Coste de movimiento** (`worldgen/costeMovimiento.ts`): función pura `costeEnPunto(campo, p)` sobre
  `CampoElevacion`, igual patrón que `evaluarTerreno` — un multiplicador continuo por punto, no un lookup
  por celda.
- **Pathfinding** (`world/rutas.ts`, `calcularRuta`): A* sobre una malla de muestreo EFÍMERA (nunca
  guardada), calculada solo dentro de la caja origen-destino con margen. Vive en `world/` y no en
  `worldgen/` porque necesita la fachada `Mapa` — `worldgen/` no puede depender de `world/` sin crear un
  ciclo (`world/mapa.ts` ya depende de `worldgen/`). Una sola función, reutilizada por caminos comerciales Y
  por el lanzamiento de cualquier caravana (comercial o de Fundación) — evita tener dos pathfindings
  distintos que puedan divergir.
- **Chokepoints** (`worldgen/chokepoints.ts`): determinista por seed, igual que ríos — entra en el pipeline
  de generación (`WORLDGEN_VERSION` sube a 7). Detectados como PUNTOS DE SILLA del campo de elevación
  (Hessiano discreto por diferencias finitas, mismo estilo que `gradienteElevacion`): mínimo local a lo
  largo de la cresta, máximo local en la dirección perpendicular — la definición matemática de un puerto de
  montaña, invariante a la orientación de la cordillera. Solo puertos de montaña en esta pasada; vados de
  río quedan fuera (ver Pendientes).
- **Movimiento de caravana sobre polilínea** (`engine/movimiento.ts`): generaliza `progreso` (0→1) de
  "fracción de la recta" a "fracción de la longitud real de `Caravana.ruta`" — el avance por tick pasa de
  una fracción fija a `velocidadBase / costeEnPunto(posición actual)`, así que cruzar colina/montaña de
  verdad cuesta más ticks sin recalibrar las velocidades base ya existentes por tipo de caravana. Caravanas
  sin `ruta` (partidas guardadas antes de Fase 0.3) siguen el comportamiento de línea recta de siempre —
  compatibilidad hacia atrás sin migración.
- **Caminos comerciales** (`engine/caminos.ts`): Doc 1.6, se generan al proponer un trueque entre dos
  asentamientos (si no existe ya uno para ese par) reusando el mismo `calcularRuta`. Viven como estado de
  PARTIDA (`GameState.caminos`/`EstadoSimulacion.caminos`), no de mundo generado — dependen de qué
  relaciones comerciales existen, no de la seed. Dan un multiplicador de velocidad
  (`COSTE_MOVIMIENTO.factorCamino`) a las caravanas que los siguen. "Mejorable vía Políticas" (Doc 1.6) no
  necesitó ninguna política nueva: `factorVelocidadCaravana` ("Rutas Rápidas", Tesorero) ya existía y se
  sigue aplicando encima del bonus de camino.
- **Control de chokepoints y peaje** (`engine/chokepoints.ts`): alcance confirmado con el usuario para este
  pase — geometría + control (asentamiento cuya zona de influencia cubre el chokepoint) + peaje en oro a
  Facciones no controladoras, cobrado al asentamiento DESTINO en el momento de la entrega y abonado al
  controlador. SIN bloqueo/escolta militar todavía (ver Pendientes).

## Acoplamiento 0 entre interfaz y motor (recordado explícitamente por el usuario)

Mismo contrato que ya regía desde la Ampliación de comercio (`Correcciones_Durante_Desarrollo.md` #38): la
interfaz (`ui/*`, `main.ts`) solo habla con `gameStore` (acciones + `getState` + `subscribe`) y con tipos de
dominio para tipar lo que lee — nunca importa `engine/*` ni reimplementa una regla de juego por su cuenta.
`world/mapa.ts` (la fachada `Mapa`) es la ÚNICA excepción declarada: se consulta directamente desde ambos
lados porque el propio archivo la define como "la única vía por la que el motor y la interfaz hablan con el
mundo" — de ahí que `ui/canvas.ts` llame a `mapa.listarRios()`/`listarChokepoints()`/`terrenoEn()` etc.
directamente. Cualquier otra cosa que decida "quién controla X" o "qué pasa si Y" es una regla de juego y
vive en `engine/`, expuesta a la interfaz como un dato ya resuelto por un método de `GameStore`.

Se rompió una vez en esta sub-fase y se corrigió (ver `Correcciones_Durante_Desarrollo.md` #39): el anillo
de control de chokepoints se calculaba dentro de `ui/canvas.ts` (importando `pointInPolygon` de `world/` y
recorriendo zonas a mano) en vez de reutilizar `controladorDeChokepoint` (`engine/chokepoints.ts`). Ahora
`GameStore.chokepointsControl()` calcula el `Map<chokepointId, asentamientoControladorId>` una vez y
`DrawState` lo lleva ya resuelto — `ui/canvas.ts` solo lo pinta.

## Contratos que se mantienen sin excepción

- Determinista por seed (mulberry32) — mismo seed, mismo mundo (incluidos los chokepoints).
- Orden de consumo del PRNG es parte del contrato — `generarChokepoints` se añadió al FINAL del pipeline
  (después de nodos/livestock) para no desplazar el consumo de ningún paso ya calibrado; toda seed produce
  nodos/bosques/ríos idénticos a v6 y solo diverge a partir de los chokepoints.
- `MapaGenerado` sigue siendo datos puros, serializable, sin funciones cerradas.
- `WORLDGEN_VERSION` sube a 7 — se acepta que todas las seeds/mundos guardados cambian.
- Los caminos comerciales, al ser estado de partida y no de mundo generado, NO suben `WORLDGEN_VERSION` —
  viven en `SimulacionExportada.caminos` (opcional, `?? []` en partidas guardadas antes de Fase 0.3).

## Pendiente (documentado también en `Preguntas_Abiertas.md`)

- Bloqueo/escolta militar de chokepoints (Doc 1.5): necesita un concepto de "guerra activa" entre Facciones
  que no existe hoy — el combate en Fase 0 es cálculo puntual (asedio/campo abierto/intercepción), no
  presencia física continua en el mapa.
- Vados de río como chokepoint: esta pasada solo detecta puertos de montaña.
- Qué pasa con un camino comercial si se rompe la relación que lo originó (Doc 1.6, ya pendiente desde antes
  de Fase 0.3): de momento el camino queda como infraestructura física permanente.
- Calibración por simulación de `COSTE_MOVIMIENTO` (coste por terreno, bonus de camino) y
  `CHOKEPOINTS_PEAJE.oro`: valores placeholder, mismo criterio que el resto de cifras nuevas del proyecto.
