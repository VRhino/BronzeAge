# Diario de descubrimientos — 100 Facciones × 1 asentamiento: fundación priorizando piedra + otro mineral

**Fecha:** 2026-08-18
**Qué es esto:** re-corrida EXACTA del escenario de
[Diario_Simulaciones_Batch_100_Facciones_Supervivencia92pct.md](Diario_Simulaciones_Batch_100_Facciones_Supervivencia92pct.md)
(mismo mapa/seed, 3000 ticks, `npcGobernanza.ts` sin cambios), cambiando SOLO cómo el escenario elige las 100
posiciones de fundación inicial: en vez de exigir solo bosque alcanzable, puntúa cada sitio candidato (+2 si
tiene piedra alcanzable, +1 si tiene además cobre/estaño/livestock) y elige primero los de mayor puntaje,
completando con sitios "solo bosque" si no alcanzan. El objetivo era probar si el techo de nivel 2
(Hallazgo #4 del diario anterior: casi ningún asentamiento tenía más de 2 de los 6 tipos de extracción)
mejora al fundar cerca de más minerales desde el principio. **Resultado: mejora mucho para piedra, pero
revela un techo distinto e inesperado — cobre/estaño/oro/livestock siguen en CERO pese a tener acceso.**

## Metodología

- Mismo mundo/seed/escala que el batch de referencia. Único cambio: la función de selección de posiciones
  del escenario (no el motor, no `npcGobernanza.ts`). 3000 ticks, foto cada 100. **0 excepciones.**
- Escenario temporal descartado tras extraer este diario.

## Resumen ejecutivo

| # | Hallazgo | Severidad/Tipo |
|---|---|---|
| 1 | La priorización funcionó para piedra: 89/100 sitios elegidos tenían piedra alcanzable (vs el ~13% que terminaba con Cantera activa en el batch anterior) | ✅ La priorización de fundación funciona como se esperaba |
| 2 | Cantera activa sube de 12 a **74 de 91 asentamientos vivos** (81%) — un salto enorme | ✅ Éxito claro |
| 3 | **Pero mina (oro), minaCobre, minaEstano y corral (livestock) se quedan en CERO durante los 3000 ticks completos** — pese a que 50/100 sitios elegidos SÍ tenían acceso a cobre/estaño/livestock | 🔴 Techo nuevo, inesperado, sin explicación confirmada todavía |
| 4 | El gate de nivel 2 sigue sin cumplirse NUNCA (0 asentamientos) — con solo Leñera y Cantera activándose de los 6 tipos, nunca se llega a "3 de 6" | 🔴 Consecuencia directa del Hallazgo #3 |
| 5 | Supervivencia prácticamente idéntica al batch anterior: 91% vs 92% — la fundación priorizada no cambió el resultado de supervivencia (para bien ni para mal), solo la composición de edificios | 🟢 Neutral, esperado |
| 6 | Motor sin tocar, 0 excepciones | ✅ Positivo |

## Hallazgo #1 y #2 — La piedra responde bien a la priorización

Al puntuar los sitios candidatos por piedra alcanzable, **89 de los 100 elegidos la tenían** (vs fundar solo
por bosque, que no la garantizaba en absoluto). El resultado se nota directo en el juego: Cantera activa pasa
de 12/92 (13%, batch anterior) a **74/91 (81%)** — la priorización de fundación es, sin duda, una palanca real
para este recurso específico.

## Hallazgo #3 — Cobre, estaño, oro y livestock: cero pese a tener acceso

Este es el hallazgo central del batch. De los 100 sitios elegidos, **50 tenían algún nodo de
cobre/estaño/livestock alcanzable** dentro del radio inicial (según `evaluarViabilidadFundacion` en el
momento de fundar). Pese a eso, `extraccionPorTipoActivosTotal` se mantuvo en
`{"mina": 0, "minaCobre": 0, "minaEstano": 0, "corral": 0}` durante los 3000 ticks completos, sin una sola
excepción — ni un solo asentamiento, de los ~50 con acceso, llegó a construir ninguno de estos 4 extractores.

No se pudo confirmar la causa raíz en este batch (no se instrumentó a ese nivel de detalle), pero dos datos
del propio código del mapa (`src/worldgen/config.ts`) descartan la explicación más obvia ("simplemente no
hay suficientes nodos"):
- `RECURSO_RAREZA`: piedra es `'comun'` (240 nodos en el mapa), cobre es `'intermedio'` (80 nodos), estaño y
  oro son `'raro'` (24 nodos cada uno) — más escasos que piedra, pero no inexistentes, y el propio dato de
  fundación (50/100 con acceso) confirma que SÍ hay suficientes cerca de donde se fundó.
- `corral` (livestock) ni siquiera comparte el mecanismo de nodo-en-mapa de los otros 4: no está en el
  conjunto `EDIFICIOS_EN_MAPA` de `engine/construction.ts` (`{mina, minaCobre, minaEstano, cantera}` — corral
  no aparece ahí), a diferencia de Cantera que sí. Esto sugiere que Corral podría depender de un mecanismo de
  colocación distinto (más parecido a Granja/Leñera, ligado a la zona en vez de a un nodo específico del
  mapa) — posible pista, no confirmada.

En resumen: la disponibilidad de recurso no es el problema (50% tiene acceso); algo en cómo la
auto-construcción prioriza o intenta construir estos 4 tipos específicos (frente a Cantera, que sí funciona
con normalidad) parece estar bloqueándolos. No se investigó más a fondo en este batch — queda como pregunta
abierta central para el siguiente paso.

## Hallazgo #4 — El gate de nivel 2 sigue sin cumplirse nunca

Con Leñera y Cantera como únicos tipos activos (2 de 6), y el gate exigiendo 3 de 6 distintos
(`edificiosMinimo`), ningún asentamiento pudo cruzar nivel 2 en 3000 ticks — mismo resultado que el batch
anterior, pese a la mejora real en variedad de RECURSOS alcanzables. La priorización de fundación por sí
sola no resuelve el gate si los edificios correspondientes nunca llegan a construirse.

## Hallazgo #5 — Supervivencia sin cambios significativos

91% de supervivencia (91/100 a tick 3000) frente al 92% del batch anterior — dentro del ruido esperado entre
corridas con posiciones distintas. La priorización de fundación no tuvo efecto notable, positivo ni
negativo, sobre la supervivencia general — actuó únicamente sobre qué se construye, no sobre si el
asentamiento sobrevive.

## Nota metodológica

- No se instrumentó por qué minaCobre/minaEstano/mina/corral nunca se activan pese a tener recurso
  alcanzable — las dos pistas del Hallazgo #3 (rareza real pero no nula, y la posible diferencia de mecanismo
  de Corral) son observaciones del código, no una causa confirmada con datos de este batch.
- La función de selección de posiciones usada aquí fue parte del escenario temporal (ya descartado), no de
  `simulaciones-batch/npcGobernanza.ts` — si el usuario quiere conservarla para batches futuros, habría que
  decidir si migrarla a algún lugar persistente (hoy la elección de posiciones vive fuera de
  `npcGobernanza.ts` a propósito, ver su propio README: "responsabilidad del escenario").

## Sugerencia de siguiente paso

1. **Diagnosticar directamente por qué minaCobre/minaEstano/mina/corral no se construyen** pese a tener
   nodo alcanzable — instrumentar, para los ~50 asentamientos con acceso, si el candidato llega a proponerse
   en `evaluarNecesidades` (y se descarta por reserva/cupo/prioridad) o si ni siquiera llega a proponerse
   (el nodo no se detecta como reclamable). Sin tocar el motor, se puede diagnosticar leyendo estado —
   modificarlo si resulta ser un gap real sí requeriría tocar el motor, sujeto a que el usuario lo apruebe.
2. Si Corral resulta depender de un mecanismo distinto (Hallazgo #3, pista 2), confirmarlo leyendo
   `sitioParaTipo`/`sitioEnBarrio` en `engine/construction.ts` para ver cómo se ubica específicamente.
