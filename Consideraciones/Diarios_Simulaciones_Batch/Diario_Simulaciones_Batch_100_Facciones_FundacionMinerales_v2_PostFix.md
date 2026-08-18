# Diario de descubrimientos — 100 Facciones × 1 asentamiento: re-corrida tras el fix de desempate de extractores

**Fecha:** 2026-08-18
**Qué es esto:** re-corrida de las MISMAS condiciones que
[Diario_Simulaciones_Batch_100_Facciones_FundacionMinerales.md](Diario_Simulaciones_Batch_100_Facciones_FundacionMinerales.md)
(100 Facciones, 1 asentamiento cada una, 5 jugadores fundadores, fundación priorizando piedra +
cobre/estaño/livestock alcanzables, 3000 ticks, `npcGobernanza.ts` sin cambios), esta vez sobre el motor
DESPUÉS de que otra sesión aplicara el fix propuesto en
[issues/extractores_minerales_nunca_se_construyen.md](../../issues/extractores_minerales_nunca_se_construyen.md)
(`EXTRACTOR_DESEMPATE`, contador de "veces starved" para romper el sesgo de orden fijo del array de
extractores). El archivo de escenario original ya no existía (se descarta tras cada corrida, por convención)
y se recreó desde cero con la misma metodología — el **seed exacto del batch original nunca quedó registrado
en ningún diario** ("mismo mapa/seed, determinista" sin el número), así que esta corrida usa un seed nuevo
fijo: la comparación es de comportamiento agregado, no de posiciones idénticas al batch anterior.

**Resultado: el fix NO resolvió el problema a esta escala.** `mina`/`minaCobre`/`minaEstano`/`corral` siguen
en CERO absoluto durante los 3000 ticks completos, igual que antes del fix — pese a que el fix está
correctamente implementado y SÍ funcionó en el escenario puntual de 1 asentamiento que lo verificó. Ver
`issues/extractores_minerales_nunca_se_construyen.md` (reabierto y actualizado con una hipótesis nueva) para
el análisis completo de por qué.

## Metodología

- Escenario recreado (no existía el archivo original): 100 Facciones, 1 asentamiento cada una, 5 jugadores
  fundadores, fundación priorizando piedra (+2 puntos) y cobre/estaño/livestock (+1 punto) alcanzables al
  fundar (radio 30), completando con sitios "solo bosque" si no alcanzan, con separación mínima de 100
  unidades entre posiciones (mismo barrido unificado grueso→fino usado en el batch original, evita el bug de
  búsqueda por celdas independientes de una corrida más antigua de esta serie).
- Motor sin tocar desde este escenario. 3000 ticks, foto cada 100. **0 excepciones.**
- Seed nuevo (no reproducible respecto al batch original — ver nota arriba). En esta corrida: **24/100**
  posiciones con piedra alcanzable y **24/100** con algún otro mineral alcanzable al fundar (mucho menos que
  el 89/100 y 50/100 del batch original — mapa distinto, no comparable en esa cifra puntual).
- Escenario temporal descartado tras extraer este diario.

## Resumen ejecutivo

| # | Hallazgo | Severidad/Tipo |
|---|---|---|
| 1 | `mina`/`minaCobre`/`minaEstano`/`corral` en CERO absoluto durante los 3000 ticks — **idéntico al resultado pre-fix**, pese a que 24/100 asentamientos fundaron con alguno de esos recursos confirmado alcanzable | 🔴 El fix no tuvo efecto observable a esta escala |
| 2 | Cantera SÍ creció con normalidad: 48→60 instancias activas (de 94→92 asentamientos vivos), plateau hacia tick ~1500 | 🟢 Sin regresión — el extractor que ya funcionaba sigue funcionando |
| 3 | El gate de nivel 2 de asentamiento sigue sin cumplirse NUNCA (`conGateNivel2Cumplido: 0` en las 30 fotos) — consecuencia directa del Hallazgo #1, mismo resultado que toda la serie anterior | 🔴 Sin cambio respecto a antes del fix |
| 4 | Supervivencia y actividad militar en línea con la serie: 94→92 vivos (8% colapso), 315 campamentos destruidos acumulados (el más alto de toda la serie), 189 trueques de supervivencia propuestos con solo 27 cumplidos | 🟢 Neutral — el NPC de gobernanza sigue comportándose igual, no se tocó |
| 5 | Investigación de código (agente de exploración, no instrumentación directa) descarta la explicación más simple ("el contador se resetea cada tick") y encuentra una hipótesis nueva más fuerte: recorte de zona de influencia contra vecinos (`computeZonaInfluencia`, solo se activa con 2+ Facciones) puede estar excluyendo el único nodo alcanzable de cobre/estaño/livestock del polígono de búsqueda ANTES de que el candidato llegue siquiera a proponerse — ver `issues/extractores_minerales_nunca_se_construyen.md` | 🟡 Hipótesis nueva, con evidencia de código, no confirmada con instrumentación directa |
| 6 | Motor sin tocar, 0 excepciones en 3000 ticks | ✅ Positivo |

## Hallazgo #1 — El fix no tuvo efecto observable

| Tick | cantera | lenera | mina | minaCobre | minaEstano | corral | conGateNivel2Cumplido |
|---|---|---|---|---|---|---|---|
| 100 | 48 | 311 | 0 | 0 | 0 | 0 | 0 |
| 500 | 58 | 314 | 0 | 0 | 0 | 0 | 0 |
| 1000 | 59 | 317 | 0 | 0 | 0 | 0 | 0 |
| 1500 | 60 | 317 | 0 | 0 | 0 | 0 | 0 |
| 2000 | 60 | 317 | 0 | 0 | 0 | 0 | 0 |
| 2500 | 60 | 317 | 0 | 0 | 0 | 0 | 0 |
| 3000 | 60 | 317 | 0 | 0 | 0 | 0 | 0 |

Sin una sola excepción en 30 fotos a lo largo de 3000 ticks. El patrón es idéntico, campo por campo, al de los
tres batches anteriores de la serie (con y sin fundación priorizada, con y sin el fix). Esto contradice
directamente la verificación puntual documentada en el issue (1 asentamiento, ~1500 ticks, `minaCobre`
llegando a 2 instancias con el fix activo) — el fix funciona quirúrgicamente en aislamiento pero no se
manifiesta en absoluto a escala de 100 Facciones compitiendo por el mismo mapa.

## Hallazgo #5 — Pista nueva: recorte de zona de influencia contra vecinos

Se investigó el código (sin instrumentar directamente el batch, sin tocar el motor) para descartar la
explicación más simple: que `asentamiento.extractoresTicksSinCupo` se estuviera perdiendo silenciosamente en
algún paso del pipeline de `avanzarSimulacion` (p. ej. una función que reconstruye el Asentamiento sin hacer
spread completo). Se descartó — el campo se hilvana correctamente en todo el pipeline.

La hipótesis con más evidencia de código: `computeZonaInfluencia` (`src/engine/zones.ts`) recorta el polígono
de zona de cada asentamiento contra el semiplano-frontera de CUALQUIER asentamiento de otra Facción cercano.
Con 1 solo asentamiento en el mundo (el escenario que verificó el fix), este recorte nunca se activa. Con 100
Facciones separadas apenas 100 unidades (y radio de zona que llega a 60 en nivel 1), sí se activa con
normalidad — y `sitioCercaDeNodo` (el que de verdad busca dónde construir un extractor) solo busca DENTRO de
ese polígono ya recortado, a diferencia del chequeo "¿hay recurso alcanzable?" que se hace al fundar (que usa
un círculo sin recortar). Combinado con que cobre/estaño/livestock son más escasos que piedra en el mapa
(`RECURSO_RAREZA`/`espacioMinimo`), un asentamiento con un solo nodo de esos recursos tiene mucha más
probabilidad de perder justo ESE nodo por el recorte que uno con piedra (que casi siempre tiene un segundo
nodo de sobra). Si esto es correcto, el candidato de minaCobre/mina/minaEstano/corral nunca llega ni a
proponerse en la mayoría de los casos — así que el contador de "veces starved" nunca tiene nada sobre lo que
acumular, y el fix (que sí corrige el desempate cuando SÍ hay competencia real) queda sin efecto práctico.

**No confirmado con instrumentación directa** — ver la sección correspondiente en
`issues/extractores_minerales_nunca_se_construyen.md` para lo que falta para confirmarlo.

## Nota metodológica

- Motor sin tocar — confirmado por `git status` antes y después (solo el escenario temporal, ya descartado).
- El seed usado en esta corrida es distinto del batch original (nunca se registró el número exacto) — las
  cifras de acceso a minerales al fundar (24/100 y 24/100) no son comparables 1:1 contra el 89/100 y 50/100
  del batch original; lo que sí es comparable, y es el dato relevante, es el patrón de CERO absoluto en los 4
  tipos de extractor, que se repite igual en ambos seeds.
- `issues/extractores_minerales_nunca_se_construyen.md` fue actualizado con esta re-corrida y la nueva
  hipótesis — su estado pasa de "CORREGIDO" a "REABIERTO".

## Sugerencia de siguiente paso

1. Confirmar la hipótesis del recorte de zona con instrumentación directa (sin tocar el motor): para los
   asentamientos con cobre/estaño/livestock confirmado alcanzable al fundar, comparar contra su
   `zonaPoligono` ya recortado en un tick posterior — si el nodo cae fuera, confirma la hipótesis.
2. Si se confirma, cualquier corrección tocaría el motor (`engine/zones.ts` o `engine/construction.ts`) y
   necesitaría aprobación aparte, igual que el resto de ideas de corrección ya listadas en el issue.
