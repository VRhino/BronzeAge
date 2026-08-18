# Diario de descubrimientos — 100 Facciones × 1 asentamiento, re-corrido tras Fase 0.6 (niveles 3→5)

**Fecha:** 2026-08-18
**Qué es esto:** re-corrida del MISMO escenario de
[Diario_Simulaciones_Batch_100_Facciones_Expansion.md](Diario_Simulaciones_Batch_100_Facciones_Expansion.md)
(100 Facciones, 1 asentamiento cada una, 5 jugadores fundadores, mismo `simulaciones-batch/npcGobernanza.ts`
sin tocar, motor sin tocar), corrida inmediatamente después de implementarse
[Fase_0_6_Definicion_Expansion_Niveles_Asentamiento.md](Fase_0_6_Definicion_Expansion_Niveles_Asentamiento.md)
(niveles de asentamiento 3→5, gates de construcción reordenados) — para medir el efecto real de ese rediseño
sobre un escenario que, en su versión anterior (3 niveles), nunca logró pasar de nivel 1.

## Metodología

- Mismo mundo/escala que el batch anterior: mapa 2000×2000 (seed distinta, determinista), **100 Facciones**,
  cada una funda **1 asentamiento** con **5 jugadores fundadores**, posiciones elegidas con
  `evaluarViabilidadFundacion(...).recomendable` (fundable + bosque alcanzable) — prioridad de bosque cercano.
- `simulaciones-batch/npcGobernanza.ts` reutilizado **sin ningún cambio** (persistente de la sesión anterior).
- **3000 ticks**, foto cada 100 (31 fotos). **0 excepciones no capturadas** en todo el batch.
- Motor sin tocar: confirmado que los únicos archivos de `src/engine/`/`src/constants.ts` con cambios
  respecto a `HEAD` son los mismos que ya traía la implementación de Fase 0.6 al empezar este batch — nada
  nuevo se tocó para correr esta simulación.
- Instrumentación ampliada respecto al batch anterior, siguiendo su propia sección de "siguiente paso":
  distribución de nivel de asentamiento a 5 niveles (antes 3), Artesanos totales, edificios de transformación
  activos, Murallas/Palacios construidos.
- El escenario temporal (`src/engine/__tests__/_tmp_batch_100_facciones_fase06.test.ts`) se borró tras
  extraer este diario, mismo criterio que los batches anteriores — nada nuevo persistente además de lo que
  ya vivía en `simulaciones-batch/`.

## Resumen ejecutivo

| # | Hallazgo | Severidad/Tipo |
|---|---|---|
| 1 | **Deadlock estructural confirmado**: cero Artesanos en los 3000 ticks, en los 100 asentamientos — el gate de nivel 2 exige 50 Artesanos, pero los 4 edificios que generan Artesanos ahora exigen nivel 2 (o 3) para construirse. Circular, sin salida por juego normal | 🔴 Crítico — bug de diseño, no falta de calibración |
| 2 | Cero asentamiento alcanzó nivel 2 (ni superior) en 3000 ticks — consecuencia directa del Hallazgo #1 | 🔴 Crítico |
| 3 | Cero expansión (0 Caravanas de Fundación lanzadas) — mismo gate de nivel 2, mismo resultado que el batch anterior | 🔴 (esperado, ver batch anterior) |
| 4 | Colapso del 93% (7/100 vivos a tick 3000) — leve mejora sobre el 97% del batch anterior, pero con una dinámica distinta: se **estabiliza en 7 desde tick 300 y no vuelve a bajar** en los 2700 ticks restantes | 🟡 Mixto — sigue siendo un colapso severo, pero estable, no un sangrado lento |
| 5 | El sistema **NO se congela** esta vez: reclutamiento y caza de bandidos siguen activos de forma sostenida y cíclica los 3000 ticks completos, a tasa constante — contraste directo con el "congelamiento total desde tick 1800" del batch anterior | ✅ Diferencia real observada |
| 6 | Una Facción llega al nivel MÁXIMO (10) hacia tick 1200 y se queda ahí, con 6986 XP acumulada a tick 3000 — muy por delante del nivel 5 original, pero (igual que antes) totalmente desacoplada del nivel de su propio asentamiento | 🟢/🟡 Confirma el techo de XP, repite el desacople ya documentado |
| 7 | Motor robusto: 0 excepciones en 3000 ticks con 5 niveles de asentamiento activos, reclutamiento, combate y expansión potencial corriendo cada tick | ✅ Positivo |

## Hallazgo #1 — Deadlock estructural: los edificios que generan Artesanos ahora exigen el nivel que los Artesanos desbloquean

Esta es la pieza central de este diario. En **los 3000 ticks, en los 100 asentamientos, `artesanosTotal` se
mantuvo en 0** — ni un solo Artesano apareció en todo el batch. La causa está en cómo interactúan dos piezas
de código que, por separado, cada una es una decisión razonable de Fase 0.6, pero juntas cierran un círculo:

1. **Los Artesanos solo aparecen si hay al menos un edificio de transformación ACTIVO**
   (`engine/population.ts:47-65`, `EDIFICIOS_TRANSFORMACION` en `engine/asentamientoQuery.ts:71` =
   `['fundicion', 'curtiduria', 'armeria', 'carpinteria']`) — sin eso, la tasa de crecimiento de Artesanos
   nunca se activa, por diseño (comentario del propio código: "no aparece ninguno hasta el primer edificio de
   transformación activo").
2. **Los 4 edificios de transformación ahora exigen nivel de asentamiento ≥2 para siquiera EMPEZAR a
   construirse** (`constants.ts`: `fundicion`/`curtiduria`/`armeria` ganaron
   `requisitoNivelAsentamientoConstruccion: 2` en Fase 0.6 — antes construibles desde nivel 1; `carpinteria`
   subió de nivel 2 a nivel 3) — y el propio Fase_0_6 (§6, registro de implementación) confirma que este gate
   se aplicó también a la auto-construcción, no solo a la manual, así que no hay una vía indirecta.
3. **El gate para subir a nivel 2 exige 50 Artesanos**
   (`NIVEL_ASENTAMIENTO.requisitos[2].artesanos`, `constants.ts:1016-1021`, sin cambios en Fase 0.6).

El círculo: para tener Artesanos hace falta un edificio de transformación activo → para construir cualquier
edificio de transformación hace falta nivel 2 → para llegar a nivel 2 hacen falta 50 Artesanos. Ningún
asentamiento que arranca en nivel 1 (que es TODOS, por diseño — Doc 1.3) tiene ninguna puerta de salida de
este ciclo por juego normal. No es un problema de calibración de cifras (como sí lo son, según el propio
documento de Fase 0.6 §4, los umbrales de población 3→4/4→5 o los techos de nivel 4-5) — es un candado
circular incondicional, del mismo tipo que el que el consejo LLM ya advirtió como riesgo bloqueante en
Fase_0_6 §5 punto 3 (aunque sobre la piedra/cantera, no sobre este eje) al recomendar validar antes de
implementar; el usuario decidió invertir el orden, y este es exactamente el tipo de hallazgo que esa
validación habría podido atrapar antes de implementar.

**Antes de Fase 0.6** (batch anterior, mismo escenario): Fundición/Curtidería/Armería eran construibles desde
nivel 1, así que sí aparecían Artesanos orgánicamente — el batch anterior registró 55 Artesanos totales y 3
edificios de transformación activos entre los 3 supervivientes finales. Fase 0.6, al mover ESE gate
específico a nivel 2 sin tocar el requisito de Artesanos del propio nivel 2, cerró esa única puerta.

## Hallazgo #2 — Cero asentamiento alcanzó nivel 2 (consecuencia directa del #1)

`nivelesAsentamiento` se mantuvo en `{1: N, 2: 0, 3: 0, 4: 0, 5: 0}` en las 31 fotos completas, para
cualquier N de asentamientos vivos en ese momento (25 en tick 100, bajando hasta 7 desde tick 300). Ni
Murallas ni Palacios se construyeron nunca (`murallasActivas`/`palaciosActivos` = 0 siempre) — ambos exigen
nivel 3 y 4 respectivamente, inalcanzables sin pasar por nivel 2 primero.

## Hallazgo #3 — Cero expansión, mismo gate que el batch anterior

`caravanasFundacionLanzadasAcumuladas` se mantuvo en 0 durante los 3000 ticks — `lanzarCaravanaFundacion`
exige `nivelActualDe(origen) >= 2` (`engine/expansion.ts`), y ningún asentamiento lo alcanzó nunca (Hallazgo
#2). Resultado idéntico al batch anterior, ahora con una causa raíz mucho más nítida (Hallazgo #1) en vez de
"no se puede confirmar con los datos disponibles" como quedó la sección equivalente del diario anterior.

## Hallazgo #4 — 93% de colapso, pero con una dinámica distinta: se estabiliza y se queda quieto

| Tick | Vivos (batch anterior, 3 niveles) | Vivos (este batch, Fase 0.6) |
|---|---|---|
| 100 | 23 | 25 |
| 200 | 8 | 12 |
| 300 | 6 | 7 |
| 500 | 5 | 7 |
| 1700 | 4 | 7 |
| 1800 | 3 | 7 |
| 3000 | 3 | 7 |

El colapso inicial (100→7 hacia tick 300) es prácticamente igual de brutal que antes, pero a partir de ahí la
curva es marcadamente distinta: en el batch anterior, la población seguía bajando lentamente hasta tick 1800
(7→5→4→3); en este batch, **se detiene en exactamente 7 desde tick 300 y no vuelve a moverse en los 2700
ticks restantes**, sin una sola muerte más. El resultado final (93% vs 97% de colapso) es una mejora leve en
términos absolutos, pero el dato más interesante es la estabilidad: los 7 supervivientes de este batch
encontraron un punto de equilibrio sostenible mucho antes y con más margen que los 3 del batch anterior.

## Hallazgo #5 — El sistema NO se congela: reclutamiento y bandidos siguen activos todo el batch

A diferencia del "Hallazgo #4" del batch anterior (congelamiento total de TODAS las métricas desde tick 1800
hasta el final), aquí `reclutamientosAcumulados` y `campamentosDestruidosAcumulados` **siguen subiendo a tasa
constante durante los 3000 ticks completos**, sin desacelerar ni una sola foto:

- `reclutamientosAcumulados`: +50 cada 100 ticks, de forma exacta, desde tick 400 hasta tick 3000 (1775
  acumulados al final — más de 4.5x los 382 del batch anterior).
- `campamentosDestruidosAcumulados`: +10 cada 100 ticks, igual de constante (303 acumulados al final, contra
  35 del batch anterior).
- `tropasVivas` se mantiene FIJO en 875 desde tick 400 en adelante — exactamente 7 asentamientos × 5
  jugadores × 25 unidades (el tope por escuadrón), es decir, los 7 supervivientes están permanentemente AL
  TOPE de tropas.

La combinación de estos tres datos dibuja un ciclo perpetuo y sostenido: los campamentos de bandidos
reaparecen (`CAMPAMENTOS_BANDIDOS.ticksRespawn`), los escuadrones los atacan y sufren bajas, y el
reclutamiento constante de los 5 jugadores por asentamiento repone esas bajas manteniendo el tope de 25 por
escuadrón — un motor de XP de combate autosostenido, sin necesidad de crecimiento de población ni de nivel de
asentamiento. No se puede afirmar con certeza que este contraste con el batch anterior sea un efecto de Fase
0.6 en sí (podría deberse a variación en qué Facciones sobrevivieron, con distinta densidad de campamentos
cercanos) — queda anotado como una diferencia real observada, no una causa confirmada.

## Hallazgo #6 — Una Facción alcanza el nivel máximo (10), desacoplada de su asentamiento

La Facción líder llega a **nivel de Facción 10** (el máximo, `NIVEL_FACCION.nivelMaximo`) hacia tick 1200 y
se mantiene ahí el resto del batch, acumulando **6986 XP** hacia tick 3000 — muy por delante del nivel 5 que
pedía el objetivo original del usuario. La XP proviene casi enteramente de la caza sostenida de campamentos
de bandidos (Hallazgo #5: XP de combate × hasta 5 jugadores por evento). Pero, igual que en el batch
anterior, **su propio asentamiento nunca superó nivel 1** — el mismo desacople entre nivel de Facción (por
XP, sin techo real con combate sostenido) y nivel de asentamiento (bloqueado por el deadlock del Hallazgo #1)
documentado ya dos veces antes.

## Hallazgo #7 — Motor robusto también con 5 niveles de asentamiento

Cero excepciones no capturadas en 3000 ticks, con hasta 100 asentamientos simultáneos, reclutamiento, combate
contra bandidos y comprobación de expansión corriendo cada tick bajo la nueva escalera de gates de Fase 0.6.

## Nota metodológica

- No se registró el detalle de "cuántos de los 6 tipos de extracción" tiene cada asentamiento individual en
  las fotos finales (se calculó una heurística agregada, `cercaDelGateNivel2`, contando extracción ≥2 tipos
  activos, que llegó a un máximo de 12 de 100 en tick 100) — dado que el Hallazgo #1 (Artesanos = 0 siempre)
  ya explica por completo por qué nunca se alcanzó nivel 2 independientemente del lado de extracción, no hizo
  falta profundizar más ahí para este batch.
- Reversión/no-modificación del motor confirmada por `git status` antes y después de correr el batch: los
  únicos archivos con cambios en `src/engine/`/`src/constants.ts` son los mismos que ya traía la
  implementación de Fase 0.6 al empezar, nada se tocó para esta simulación.
- El escenario temporal se descartó tras extraer este diario; `npcGobernanza.ts` queda sin cambios,
  disponible para el siguiente batch.

## Sugerencia de siguiente paso

El Hallazgo #1 no es una cuestión de calibración de cifras (las que sí quedaron pendientes en Fase_0_6 §4) —
es un candado circular que, tal como está el código hoy, hace que **ningún asentamiento pueda superar nivel 1
por juego normal, nunca**, salvo que se rompa el ciclo por algún otro medio (conquista/anexión de un
asentamiento que ya viniera con Artesanos de antes, herencia de partidas guardadas pre-Fase 0.6, u otra vía
fuera del alcance de este batch). Antes de correr más simulaciones de calibración sobre los placeholders de
población/techos de nivel 4-5 (que hoy son inalcanzables de cualquier forma), probablemente valga la pena que
el usuario decida cómo romper este círculo — dos vías obvias, sin implementar ninguna todavía: (a) bajar el
gate de construcción de Fundición/Curtidería/Armería de vuelta a nivel 1 (revierte parcialmente la intención
de Fase 0.6 §2 de "concentrar la responsabilidad de transformación en nivel 2"), o (b) bajar el umbral de
Artesanos del gate de nivel 2 a algo alcanzable sin edificios de transformación, o introducir una fuente de
Artesanos que no dependa de ellos. Ninguna de las dos se implementó aquí — el batch se corrió tal como pidió
el usuario, sin tocar el motor.
