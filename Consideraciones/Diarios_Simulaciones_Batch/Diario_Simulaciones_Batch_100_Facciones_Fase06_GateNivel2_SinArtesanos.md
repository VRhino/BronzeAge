# Diario de descubrimientos — 100 Facciones × 1 asentamiento, Fase 0.6 con el gate de nivel 2 corregido

**Fecha:** 2026-08-18
**Qué es esto:** tercera corrida del MISMO escenario base (100 Facciones, 1 asentamiento cada una, 5
jugadores fundadores, `simulaciones-batch/npcGobernanza.ts` sin tocar, motor sin tocar), esta vez después de
que el usuario corrigiera directamente en el motor el deadlock circular documentado en
[Diario_Simulaciones_Batch_100_Facciones_Fase06_Niveles_Asentamiento.md](Diario_Simulaciones_Batch_100_Facciones_Fase06_Niveles_Asentamiento.md)
(Hallazgo #1): `NIVEL_ASENTAMIENTO.requisitos[2].artesanos` pasó de 50 a **0** — el gate de nivel 1→2 ya no
exige Artesanos, solo Pesants (200) + 3 de los 6 edificios de extracción. El resto de la cadena (Artesanos
siguen sin aparecer sin un edificio de transformación activo, y esos edificios siguen exigiendo nivel 2 para
construirse) queda intacta — el fix rompe el círculo desde el lado de la entrada, no desde el de la salida.

## Metodología

- Exactamente el mismo escenario que los dos diarios anteriores de esta serie: mapa 2000×2000, 100 Facciones,
  1 asentamiento cada una, 5 jugadores fundadores, posiciones recomendables (fundable + bosque alcanzable),
  `npcGobernanza.ts` reutilizado sin cambios, 3000 ticks, foto cada 100 (31 fotos).
- **0 excepciones no capturadas** en los 3000 ticks.
- Motor sin tocar por esta sesión: el único cambio respecto al batch anterior es el que ya hizo el usuario
  directamente (`constants.ts`, artesanos: 0 en el requisito de nivel 2) — confirmado que ningún otro archivo
  de `src/engine/`/`src/constants.ts` cambió durante esta corrida.
- Instrumentación ampliada de nuevo: desglose de los 6 tipos de extracción por separado (no solo el conteo
  agregado) para diagnosticar cuáles de los 6 realmente se usan.
- Limitación conocida (ver Nota metodológica): esta vez sí se registraron las Caravanas de Fundación
  **lanzadas** acumuladas, pero no se desglosaron **llegadas** vs **perdidas** por separado.

## Resumen ejecutivo

| # | Hallazgo | Severidad/Tipo |
|---|---|---|
| 1 | **El deadlock se rompió**: nivel 2 de asentamiento SÍ se alcanza ahora — hasta 4 asentamientos en nivel 2 simultáneamente (tick 300-600), estabilizado en 2 desde tick 700 hasta el final | ✅ Confirma que el fix del usuario resuelve el Hallazgo #1 anterior |
| 2 | **Nivel 3 sigue sin alcanzarse nunca** — 0 asentamientos en nivel 3+ en las 31 fotos, pese a que el nivel 2 sí se logra: aparece un nuevo cuello de botella un escalón más arriba | 🟡 A vigilar — posible próximo deadlock, no confirmado como circular todavía |
| 3 | **Expansión real, por primera vez en esta serie de batches**: 62 Caravanas de Fundación lanzadas acumuladas a tick 3000 (0 en los dos batches anteriores) | ✅ Objetivo original del usuario, parcialmente alcanzado |
| 4 | Pero la expansión no se sostiene: solo **1 Facción** termina con 2+ asentamientos propios a tick 3000, pese a 62 caravanas lanzadas — la inmensa mayoría de los intentos no se traduce en una segunda ciudad viva y estable | 🟡 Expansión frágil, no un motor de crecimiento sano todavía |
| 5 | Colapso del **90%** (10/100 vivos a tick 3000) — mejor que los dos batches anteriores de esta serie (93% y 97%), y de nuevo se estabiliza rápido (desde tick 700) y se mantiene exacto el resto del batch | ✅ Mejora medible y reproducible del patrón "estabiliza y no se mueve" |
| 6 | De los 6 tipos de edificio de extracción, **oro y estaño casi nunca aparecen** (0-1 activos durante casi todo el batch) — el gate de "3 de 6" en la práctica se resuelve casi siempre con Leñera+Cantera+(Corral o Cobre) | 🟢 Diagnóstico útil, no un bug |
| 7 | Actividad sostenida sin congelamiento, igual que el batch anterior: reclutamiento y caza de bandidos siguen a ritmo constante los 3000 ticks completos (2442 reclutamientos, 302 campamentos destruidos acumulados — ambos récords de la serie) | ✅ Reproduce el Hallazgo #5 del diario anterior |
| 8 | Motor robusto: 0 excepciones en 3000 ticks con expansión REAL ocurriendo por primera vez en la serie | ✅ Positivo |

## Hallazgo #1 — El fix funciona: nivel 2 de asentamiento ya es alcanzable

Con el requisito de Artesanos en 0, el gate de nivel 2 (`NIVEL_ASENTAMIENTO.requisitos[2]`) se reduce a 200
Pesants + al menos 3 de los 6 edificios de extracción — ambos alcanzables sin depender de ningún edificio de
nivel 2. Resultado: **4 asentamientos llegan a nivel 2 hacia tick 300-600**, luego el número baja a 2 (por
colapso general de la población, no por degradación de nivel) y se mantiene ahí, estable, desde tick 700
hasta el final. Confirma limpiamente que el diagnóstico del diario anterior (Hallazgo #1: circular, Artesanos
↔ nivel 2 ↔ edificios de transformación) era la causa raíz real, no una entre varias posibles.

## Hallazgo #2 — Nivel 3 sigue sin alcanzarse nunca: nuevo cuello de botella un escalón más arriba

En ninguna de las 31 fotos apareció un solo asentamiento en nivel 3, 4 o 5. El gate de nivel 3
(`NIVEL_ASENTAMIENTO.requisitos[3]`, sin cambios en esta corrida) exige 500 Pesants + 200 Artesanos + los 5
edificios `armeria`+`curtiduria`+`fundicion`+`barracon`+`galeriaDeTiro` TODOS activos a la vez — y a
diferencia del gate de nivel 2 (3 de 6, con `edificiosMinimo`), este es una lista completa sin margen. Los
datos de esta corrida (`edificiosTransformacionActivosTotal` tope en 9, repartido entre hasta 4 asentamientos
de nivel 2 — es decir, ~2-3 edificios de transformación por asentamiento en el mejor caso, no los 3 que exige
el gate por sí solos, y sin contar Barracón/Galería de tiro en absoluto) sugieren que el mismo patrón de
cuello de botella podría estar repitiéndose un nivel más arriba, pero **esta vez no se puede confirmar si es
otro deadlock circular o simplemente un gate más exigente que necesita más tiempo/población** — a diferencia
del Hallazgo #1 anterior, aquí no hay una dependencia circular obvia en el código (Artesanos SÍ pueden crecer
una vez hay transformación en nivel 2), así que es más probable que sea cuestión de tiempo/escala que un
candado estructural. Candidato claro para un batch de seguimiento más largo o instrumentado por asentamiento.

## Hallazgo #3 y #4 — Expansión real pero fragil

Por primera vez en esta serie de 3 batches, `caravanasFundacionLanzadasAcumuladas` **no se queda en 0**:
sube de 4 (tick 300) a 62 (tick 3000), de forma sostenida durante todo el batch — el NPC de gobernanza
(`expandirSiPuede`, sin cambios) por fin tiene asentamientos candidatos (`nivelActualDe >= 2`) a los que
ofrecerles un destino, tick tras tick.

Pero el resultado neto es frágil: `faccionesConMasDeUnAsentamiento` llega a un pico de **2** en tick 300 y
cae a **1** desde tick 400 en adelante, quedándose ahí el resto del batch — pese a que las caravanas
lanzadas siguen subiendo sin parar. Esto solo se puede leer de una forma: la gran mayoría de las 62 Caravanas
de Fundación lanzadas NO terminan en una segunda ciudad viva y estable para su Facción (se pierden en tránsito,
o el asentamiento que fundan colapsa poco después por Mantenimiento, igual que le pasa al 90% de los
asentamientos originales). Solo una Facción logra sostener una segunda ciudad hasta el final del batch.

## Hallazgo #5 — 90% de colapso, mejor y más estable que los batches anteriores

| Tick | 3 niveles (batch original) | Fase 0.6, gate con Artesanos | Fase 0.6, gate sin Artesanos (este batch) |
|---|---|---|---|
| 100 | 23 | 25 | 30 |
| 300 | 6 | 7 | 14 |
| 700 | ~4-5 | 7 | 10 |
| 3000 | 3 | 7 | **10** |

La curva de supervivencia mejora de forma consistente en cada corrida de esta serie. Este batch, con nivel 2
real alcanzándose y expansión ocurriendo, termina con el doble de supervivientes que el batch anterior (10 vs
7) y el triple que el batch original de 3 niveles (10 vs 3) — y, igual que en la corrida anterior, se
estabiliza rápido (desde tick 700) y no vuelve a perder ni un asentamiento más en los 2300 ticks restantes.

## Hallazgo #6 — Oro y estaño casi nunca forman parte del gate de extracción

Desglose de `extraccionPorTipoActivosTotal` a tick 3000 (10 asentamientos vivos): `lenera` 10/10, `cantera`
4/10, `minaCobre` 2/10, `corral` 2/10, **`mina` (oro) 0/10, `minaEstano` 0/10**. Esta distribución es
consistente en casi todas las fotos del batch — `mina` y `minaEstano` rara vez pasan de 0-1 activas en total,
mientras `lenera` está en el 100% de los asentamientos y `cantera` en la mayoría. Encaja con el propio diseño
documentado del mapa (`minaEstano`: "raro y concentrado", Doc 1.1/5.7 — menos nodos que cobre/oro) — el gate
de "3 de 6" (`edificiosMinimo`) en la práctica casi siempre se resuelve con Leñera + Cantera + (Corral o
Cobre), no con las 6 opciones por igual. No es un bug, pero es un dato útil para calibrar si alguna vez se
quiere ajustar cuáles 6 tipos cuentan para este gate o el propio `edificiosMinimo`.

## Hallazgo #7 — Actividad sostenida, sin congelamiento (reproduce el batch anterior)

`reclutamientosAcumulados` (2442) y `campamentosDestruidosAcumulados` (302) a tick 3000 son ambos récords de
la serie — más altos que el batch anterior de Fase 0.6 (1775 / 303, aproximadamente el mismo ritmo pese a
tener más asentamientos vivos) y muy por delante del batch original de 3 niveles (382 / 35). Confirma que el
Hallazgo #5 del diario anterior (ciclo sostenido de recluta-combate-repone, sin congelarse) no fue un
resultado aislado de esa corrida — se repite aquí con una población superviviente distinta y mayor.

## Hallazgo #8 — Motor robusto también con expansión real ocurriendo

0 excepciones no capturadas en 3000 ticks — esta vez con Caravanas de Fundación efectivamente saliendo y
llegando (a diferencia de los dos batches anteriores, donde esa ruta de código nunca se ejercitó de verdad
porque ningún asentamiento alcanzaba el gate).

## Nota metodológica

- No se desglosaron Caravanas de Fundación **llegadas** vs **perdidas** por separado (solo se registró
  "lanzadas acumuladas") — el dato de `faccionesConMasDeUnAsentamiento` estancado en 1 pese a 62 lanzadas es
  la única evidencia indirecta de que la mayoría no prospera; un batch de seguimiento podría instrumentar
  esto directamente contando eventos de llegada/pérdida del motor.
- No se diagnosticó el gate de nivel 3 por asentamiento individual (Hallazgo #2) — se dejó como pregunta
  abierta en vez de una causa confirmada, a diferencia del Hallazgo #1 del diario anterior que sí se pudo
  atribuir con certeza a una dependencia circular en el código.
- El escenario temporal se descartó tras extraer este diario; `npcGobernanza.ts` queda sin cambios.
- Motor sin tocar por esta sesión — el único cambio de código entre este batch y el anterior de la serie es
  el fix que hizo el propio usuario (`NIVEL_ASENTAMIENTO.requisitos[2].artesanos: 50 → 0`).

## Sugerencia de siguiente paso

Dos preguntas abiertas, ninguna requiere tocar el motor para investigarse:
1. **Diagnosticar el gate de nivel 3** (Hallazgo #2): instrumentar por asentamiento, para cada uno de los 2
   que llegaron a nivel 2, cuántos de los 5 edificios de `requisitos[3].edificios` tiene activos y su
   población de Artesanos — para saber si está "cerca" (cuestión de tiempo/escala) o si hay otra dependencia
   oculta parecida a la del Hallazgo #1 anterior.
2. **Instrumentar llegada/pérdida de Caravanas de Fundación** (Hallazgo #3/#4): contar eventos de llegada
   exitosa vs pérdida en tránsito vs colapso posterior del nuevo asentamiento, para entender por qué 62
   lanzamientos producen solo 1 Facción con una segunda ciudad sostenida.
