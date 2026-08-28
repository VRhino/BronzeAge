# Diario de descubrimientos — 100 Facciones × 1 asentamiento: 92% de supervivencia (vs 3-12% en toda la serie anterior)

**Fecha:** 2026-08-18
**Qué es esto:** re-corrida EXACTA del escenario de
[Diario_Simulaciones_Batch_100_Facciones_TruequeSupervivencia.md](Diario_Simulaciones_Batch_100_Facciones_TruequeSupervivencia.md)
(mismo mapa/seed, 100 Facciones, 1 asentamiento cada una, 5 jugadores fundadores, 3000 ticks), tras dos
ajustes puntuales al `simulaciones-batch/npcGobernanza.ts`: (1) el trueque de supervivencia ya no propone un
pacto nuevo si el asentamiento necesitado YA tiene ayuda activa en camino para ese recurso (cierra el
Hallazgo #4 del diario anterior — acuerdos redundantes con varios socios), y (2) mientras el asentamiento
sigue en nivel 1, solo UN residente recluta por tick (no los 5). **El resultado es el mejor de toda la serie,
por un margen enorme.**

## Metodología

- Mismo mundo/seed/escala que el batch de referencia — único cambio: los dos ajustes de arriba en el NPC.
  Motor sin tocar (confirmado por `git status` antes/después). 3000 ticks, foto cada 100. **0 excepciones.**
- Escenario temporal descartado tras extraer este diario.

## Resumen ejecutivo

| Batch (mismo escenario base, 100 Facciones × 1) | Vivos a tick 3000 | % Colapso |
|---|---|---|
| Fase 0.6 original (gate de nivel 2 con Artesanos) | 3 | 97% |
| Fase 0.6, gate corregido (sin Artesanos) | 10 | 90% |
| + Trueque de supervivencia (v1, con el gap de duplicados) | 12 | 88% |
| **+ Gate de "ayuda en camino" + reclutamiento 1× en nivel 1 (este batch)** | **92** | **8%** |

| # | Hallazgo | Severidad/Tipo |
|---|---|---|
| 1 | **92% de supervivencia a tick 3000** — el mejor resultado de toda la serie por lejos, una mejora de +767% en supervivientes sobre el batch anterior (12→92) | ✅ Éxito rotundo |
| 2 | La curva es prácticamente PLANA desde tick 100: 93 vivos en tick 100, 92 en tick 3000 — casi todo el colapso ocurre (si acaso) en la ventana crítica inicial, no después | ✅ Estabilidad real, no solo un colapso pospuesto (a diferencia de todos los batches anteriores) |
| 3 | El trueque de supervivencia bajó de 7723 propuestas (v1) a solo 270 — el gate de "ayuda en camino" funcionó exactamente como se esperaba, sin perder capacidad: casi nadie necesita ayuda porque casi nadie está en riesgo | ✅ Confirma el diagnóstico del diario anterior |
| 4 | Pese al éxito en supervivencia, **el progreso de nivel de asentamiento sigue completamente estancado**: 0 asentamientos alcanzaron nivel 2 en 3000 ticks — ahora el cuello de botella es claramente la variedad de nodos de extracción alcanzables, no la supervivencia | 🟡 Nuevo techo claro, distinto del anterior |
| 5 | Reclutamiento mucho más comedido (203 vs 545 acumulados) pero MÁS campamentos de bandidos destruidos en total (311, el número más alto de toda la serie) — con casi todos los asentamientos vivos y activos durante 3000 ticks completos, hay más "cuerpo" total cazando bandidos aunque cada uno recluta menos | ✅ Trade-off que se invierte a favor cuando la base de asentamientos vivos es mucho mayor |
| 6 | Motor sin tocar, 0 excepciones en 3000 ticks | ✅ Positivo |

## Hallazgo #1 y #2 — 92% de supervivencia, estable desde el tick 100

| Tick | Vivos |
|---|---|
| 100 | 93 |
| 500 | 93 |
| 1000 | 93 |
| 1500 | 93 |
| 2000 | 93 |
| 2300 | 93 |
| 2400 | 92 |
| 3000 | **92** |

La curva es, con diferencia, la más plana de toda la serie: de 93 vivos en tick 100 a 92 en tick 3000, solo
UNA muerte más en los 2900 ticks restantes. No hay señal de un colapso pospuesto (como sí lo hubo en todos
los batches anteriores, incluido el de supervivencia v1, que seguía bajando lentamente después de tick 100)
— esto sugiere que la combinación de reserva de madera (150) + reclutamiento comedido (1 residente en nivel
1) + trueque de supervivencia (sin el gap de duplicados) resuelve la ventana crítica de fondo, no solo la
retrasa.

## Hallazgo #3 — El trueque de supervivencia funciona mejor con MENOS volumen

`truequesSupervivenciaAcumulados` baja de 7723 (v1) a **270** — una reducción del 96.5% en propuestas, exactamente
lo esperado del gate de "ayuda en camino". Y sin embargo el resultado de supervivencia es muchísimo MEJOR, no
peor: confirma que el volumen alto de la v1 era pura redundancia (acuerdos duplicados que no aportaban nada
real), no una señal de que el mecanismo estuviera trabajando duro para salvar asentamientos. `acuerdosCumplidos`
baja de 137 a 26 en términos absolutos, pero la RAZÓN es sana: casi nadie necesita ayuda ya (la reserva +
reclutamiento comedido resuelven la mayoría de los casos por sí solos), así que hay menos necesidad real que
cubrir, no un mecanismo peor.

## Hallazgo #4 — Con la supervivencia resuelta, el nuevo techo es la variedad de extracción

`nivelesAsentamiento` se mantiene en 100% nivel 1 durante los 3000 ticks completos — ni un solo asentamiento
alcanzó nivel 2, pese a que ahora prácticamente todos siguen vivos y activos. El desglose por tipo
(`extraccionPorTipoActivosTotal`, tick 3000): `lenera` 92/92 (universal), `cantera` 12/92, y
**`mina`/`minaCobre`/`minaEstano`/`corral` en CERO durante todo el batch**. El gate de nivel 2 exige 3 de los
6 tipos — con solo 2 apareciendo alguna vez (Leñera y Cantera), y Cantera solo en el 13% de los
asentamientos, la enorme mayoría se queda estructuralmente corta, no por falta de tiempo o de supervivencia,
sino porque los otros 4 tipos de extracción dependen de nodos de recurso (cobre/estaño/oro/livestock) que
simplemente no existen dentro del radio inicial de la mayoría de los asentamientos (ver el propio comentario
de diseño en `evaluarViabilidadFundacion`: solo exige bosque alcanzable, no minerales). Con la supervivencia
ya resuelta, este es ahora el cuello de botella más claro y accionable de toda la serie.

## Hallazgo #5 — Menos reclutamiento por asentamiento, pero más caza de bandidos en total

| Métrica (tick 3000) | v1 (trueque, sin fix) | Este batch |
|---|---|---|
| Vivos | 12 | 92 |
| Reclutamientos acumulados | 545 | 203 |
| Campamentos destruidos acumulados | 123 | **311** |
| Nivel de Facción máximo | 8 | 8 |
| XP de Facción máxima | 989 | 1477 |

Con el límite de 1 residente reclutando en nivel 1, cada asentamiento individual recluta menos — pero al
haber 92 asentamientos vivos en vez de 12, el total de campamentos destruidos termina siendo el más alto de
toda la serie (incluso por encima del batch original sin ningún gate, que llegó a 302). La prudencia
individual, multiplicada por una base de supervivientes mucho mayor, termina produciendo MÁS actividad
militar agregada, no menos.

## Nota metodológica

- Motor sin tocar — confirmado por `git status` antes y después: solo `simulaciones-batch/npcGobernanza.ts`
  y su `README.md` cambiaron.
- No se instrumentó el detalle de nodos de recurso alcanzables por asentamiento (Hallazgo #4) — la hipótesis
  sobre escasez de nodos de cobre/estaño/oro/livestock se apoya en el diseño ya documentado del mapa
  (`RECURSO_RAREZA`, comentarios de `evaluarViabilidadFundacion`), no en una medición directa de este batch.

## Sugerencia de siguiente paso

1. **Diagnosticar directamente la disponibilidad de nodos de extracción** por asentamiento (cuántos de los 6
   tipos tienen al menos un nodo alcanzable en su radio, no solo cuántos construyeron) — confirmaría si el
   Hallazgo #4 es un techo de mapa/generación de mundo o algo que el NPC podría mitigar (p. ej., fundando más
   cerca de minerales, si el usuario decide priorizar eso además de bosque).
2. Con la supervivencia ya resuelta con margen (92%), un batch más largo (o con más ticks) podría revelar si
   alguna Facción termina cruzando el nivel 2 tarde, o si el techo de extracción es verdaderamente estructural
   e independiente del tiempo.
