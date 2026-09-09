# Diario de descubrimientos — por qué los NPC del batch no construían NINGÚN edificio de transformación

**Fecha:** 2026-09-08
**Qué es esto:** investigación puntual, no una vuelta completa de la metodología de los diarios anteriores.
Arrancó de una observación del usuario: en varias corridas de `scripts/run-batch-sim.ts` los asentamientos
llegaban al gate de nivel 2 pero se quedaban **para siempre** con 0 Artesanos y 0 Curtiduría/Armería/Fundición
activas, clavados en el techo de población de nivel 1 (225 pesants). El diario
[Diario_Simulaciones_Batch_500_Transformacion.md](Diario_Simulaciones_Batch_500_Transformacion.md) ya había
medido una adopción baja (13-19%), pero esto era otra cosa: adopción **cero** en la mitad de las semillas.

A diferencia de todos los diarios anteriores de la serie, **este sí terminó en un cambio de código** — pero en
el arnés de batch (`scripts/run-batch-sim.ts`), no en `src/engine/*` ni en `src/session/*`, que siguen sin
tocarse.

## Resumen ejecutivo

| # | Hallazgo | Severidad |
|---|---|---|
| 1 | El arnés de batch NO hilaba el contador de ids del NPC entre ticks — cada tick arrancaba en 0 | 🔴 Bug del arnés |
| 2 | Consecuencia: `anadirEdificioManualmente` generaba el MISMO id (`edificio-<asent>-manual-N`) tick tras tick para el Barracón/Galería que el NPC re-encola | 🔴 Causa raíz |
| 3 | `avanzarConstruccion` indexa su `Map` de resultados por id: dos edificios con el mismo id se pisan, y el Paso 2 acababa devolviendo el proyecto promocionado a `en_cola`. Deadlock total de la cola de obra | 🔴 Efecto |
| 4 | La Curtiduría auto (banda de score más baja, 500) nunca conseguía hueco de obra detrás de 3 Barracones fantasma con id colisionado → 0 transformación → 0 Artesanos → población clavada en 225 | 🔴 Efecto |
| 5 | Las semillas que "funcionaban" antes (7, 23) solo se libraban por timing de contención de cuadrillas, no porque el bug no las tocara | 🟢 Explica la bimodalidad |
| 6 | El fix (hilar `contadorInicial`/`contadorFinal` igual que `session/comandos/avanzarFaccionesNpc.ts`) es de 3 líneas y NO toca el motor | ✅ |
| 7 | Post-fix: transformación pasa de 0 a ~60-100 edificios activos por corrida a tick 250, miles de Artesanos, población rompiendo los 225, y aparece el primer asentamiento en **nivel 3** de toda la serie de diarios | ✅ |
| 8 | 0 excepciones del motor, antes y después | ✅ |

## Metodología

- `scripts/run-batch-sim.ts` reutilizado tal cual (motor + `avanzarNpcGobernanza` sin `faccionesIds` = NPC
  gobierna el mundo entero), 30 Facciones, 1 asentamiento inicial cada una, 5 jugadores fundadores.
- Diagnóstico con un escenario temporal (`scripts/_tmp_transf_diag.ts`, borrado al terminar) que volcaba, para
  un asentamiento concreto y tick a tick, el estado de construcción ANTES del motor, DESPUÉS del motor y
  DESPUÉS del NPC, con el id de cada edificio en la cola.
- Comparación antes/después sobre 6 semillas (3, 7, 11, 23, 42, 55), 1000 ticks cada una. El "antes" se midió
  con `git stash` sobre el mismo commit.

## Hallazgo #1-#3 — La cadena del deadlock

`avanzarNpcGobernanza` (`src/session/npcGobernanza.ts:1328`) arranca cada llamada con:

```ts
let contador = config.contadorInicial ?? 0;
```

La partida real (`src/session/comandos/avanzarFaccionesNpc.ts:28`) le pasa `contadorInicial: ctx.ids.actual()`
y luego adelanta su generador con `ctx.ids.fijar(resultado.contadorFinal)` — así los ids que el NPC crea nunca
se repiten ni chocan con una acción manual posterior.

`scripts/run-batch-sim.ts` **no hacía ni una cosa ni la otra**: llamaba con `config` a secas, sin
`contadorInicial`, y descartaba `contadorFinal`. Cada tick, el contador del NPC volvía a 0.

`asegurarNucleoMilitar` (`npcGobernanza.ts:397`) llama a `anadirEdificioManualmente(...'barracon'..., contador)`
en cuanto `!tieneOEnCurso(asentamiento, 'barracon')`, y ese helper acuña el id como
`edificio-${asentamiento.id}-manual-${contador}`. Con el contador reseteado y el asentamiento maduro corriendo
los 3 mismos `contador++` cada tick (granjas → comercial → militar), **el id sale idéntico cada tick**:
`edificio-<asent>-manual-14`, tick tras tick.

Secuencia observada (asentamiento `...-o-4-175-1600`, seed 11):

```
t185 NPC   COLA[barracon(manual-14)]              ← el NPC encola el Barracón
t186 MOTOR CONSTR[barracon(manual-14)]            ← Paso 2 lo promociona
t186 NPC   COLA[..., galeriaDeTiro(manual-14)]    ← Barracón ya "en curso" → el NPC pasa a Galería,
                                                     que recibe el MISMO id manual-14
t187 MOTOR CONSTR[vivienda]  (barracon DESAPARECE)
```

En `avanzarConstruccion` los resultados del tick se acumulan en un `Map` keyed por `edificio.id`
(`construction.ts` ~1290-1368). Con `barracon(manual-14)` en `en_construccion` y `galeria(manual-14)` en
`en_cola`:

- Paso 1 hace `resultados.set('manual-14', barracon)`.
- Paso 2 promociona la galería: `resultados.set('manual-14', {galeria, en_construccion})` — **sobrescribe al
  Barracón**.
- El reensamblado final `asentamiento.edificios.map(e => resultados.get(e.id)!)` mapea las DOS posiciones
  (Barracón y Galería) al mismo objeto. El Barracón se pierde.

A partir de ahí bola de nieve: `tieneOEnCurso` vuelve a dar `false` (no queda ningún Barracón), el NPC re-encola
otro con id `manual-14`, y en pocos ticks hay **3 objetos con el mismo id** ocupando 3 de los 4 slots de
`NECESIDADES.maximoEnCola`. El baile de sobre-escritura del Paso 2 los deja SIEMPRE de vuelta en `en_cola`:

```
t250..t1600  CONSTR[]  COLA[barracon#900, curtiduria#500, barracon#900, barracon#900]   (congelado)
```

`CONSTR` vacío, `cupoObraDisponible = 2`, y aun así nada promociona. Cola muerta.

## Hallazgo #4 — Por qué se propaga a Artesanos y a la población

La Curtiduría la auto-detecta `evaluarNecesidades` con `SCORE_BANDAS.transformacion = 500` — la banda más
baja. Detrás de 3 Barracones fantasma a 900 que se re-promocionan y se caen cada tick, nunca ve un hueco de
obra. Sin ningún edificio de transformación activo no aparecen Artesanos (regla ya documentada en
[Diario_Simulaciones_Batch_100_Facciones_Fase06_GateNivel2_SinArtesanos.md](Diario_Simulaciones_Batch_100_Facciones_Fase06_GateNivel2_SinArtesanos.md)),
y sin la vía de crecimiento de Artesanos el asentamiento se queda en el techo de Vivienda de nivel 1
(14 Viviendas → ~225 pesants), aunque `calcularNivelAsentamiento` ya diga 2.

Es decir: **el gate circular Artesanos ↔ nivel 2 ↔ transformación que el usuario ya rompió en 2026-08-18 por
el lado de la entrada seguía cerrado de facto por el lado de la salida**, pero por un bug del arnés, no por el
diseño.

## Hallazgo #5 — La bimodalidad entre semillas

Antes del fix, con 30 Facciones × 1000 ticks:

| seed | transf activa @250 / @500 / @1000 | Artesanos @1000 |
|---|---|---|
| 3  | 0 / 0 / 0 | 0 |
| 7  | 0 / 0 / ~8 | ~3000 (tardío) |
| 11 | 0 / 0 / 0 | 0 |
| 23 | 17 / 24 / 24 | ~3400 |
| 42 | 0 / 0 / 0 | 0 |
| 55 | — / 6 / 6 | 200 |

Las semillas 7/23/55 no se libraban del bug: simplemente, en algún tick temprano la contención de cuadrillas
dejó completar el Barracón ANTES de que el NPC encolara una Galería con el id colisionado, y a partir de ahí
`tieneOEnCurso` se mantuvo `true`. Puro azar de scheduling.

## Hallazgo #7 — Post-fix

Cambio en `scripts/run-batch-sim.ts` (mismo patrón que el backend):

```ts
let contadorNpc = 0;
// ...
const trasNpc = avanzarNpcGobernanza(trasMotor, mapa, contexto, { ...config, contadorInicial: contadorNpc });
contadorNpc = trasNpc.contadorFinal;
```

Mismas 6 semillas, 30 Facciones × 1000 ticks:

| seed | transf activa @250 / @500 / @1000 | Artesanos @500 | pesantsMaximo @1000 |
|---|---|---|---|
| 3  | — / 98 / 113 | 3790 | — |
| 7  | 72 / 95 / 86 | 3277 | 600 (niv 3 visto a t500) |
| 11 | 72 / 136 / 133 | 4825 | 700 |
| 23 | 72 / 99 / 89 | 3975 | 600 |
| 42 | 62 / 116 / 120 | 4611 | 600 |
| 55 | — / 102 / 93 | 3736 | — |

- Transformación pasa de **0** a **60-72 edificios activos ya a tick 250**, estabilizando en ~90-140.
- `pesantsMaximo` rompe los 225: 349 → 675 → 700 (seed 11).
- Primer asentamiento en **nivel 3** de toda la serie de diarios batch (seed 7, tick 500).
- 0 excepciones del motor en las 6 corridas.

## Nota de alcance y siguientes preguntas

- El motor NO se tocó. `avanzarConstruccion` confiando en que los ids de edificio son únicos dentro de un
  asentamiento es un contrato razonable que la partida real ya respeta (`ctx.ids`). El agujero estaba solo en
  el arnés de batch, que es el único otro consumidor de `avanzarNpcGobernanza`.
- Queda como posible endurecimiento defensivo (NO hecho, YAGNI hasta que haga falta): que
  `anadirEdificioManualmente` o `avanzarConstruccion` detecten un id duplicado y fallen ruidosamente en vez de
  corromper la cola en silencio. Hoy un id colisionado no lanza nada.
- Todos los diarios batch **anteriores a esta fecha** que midieron adopción de transformación / Artesanos /
  nivel 3 lo hicieron con este bug activo. Sus números de "adopción baja" y "nivel 3 inalcanzable" son un
  límite inferior contaminado — conviene re-medir esas preguntas con una corrida limpia antes de sacar
  conclusiones de balance.
- El escenario temporal de diagnóstico (`scripts/_tmp_transf_diag.ts`) se borró tras extraer este diario.
