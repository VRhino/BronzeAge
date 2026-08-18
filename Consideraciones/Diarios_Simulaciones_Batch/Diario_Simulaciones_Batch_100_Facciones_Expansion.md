# Diario de descubrimientos — 100 Facciones, expansión, nivel de Facción y XP de combate (3000 ticks)

**Fecha:** 2026-08-18
**Qué es esto:** continuación directa de
[Diario_Simulaciones_Batch_100_Asentamientos_Comercio.md](Diario_Simulaciones_Batch_100_Asentamientos_Comercio.md),
pero con un escenario distinto a petición del usuario: en vez de 20 Facciones de 5 asentamientos cada una,
esta vez son **100 Facciones con 1 asentamiento cada una** (5 jugadores fundadores por asentamiento) — el
foco no es el trueque, es si las Facciones CRECEN: si fundan nuevos asentamientos (expansión) y si llegan a
subir hasta nivel de Facción 5, ejercitando dos fuentes de experiencia que el diario anterior no tocó
(reclutamiento y caza de campamentos de bandidos).

## Cambios de código de esta sesión (a diferencia del batch anterior, que quedó revertido)

1. **Reversión completa** del cambio de motor del batch anterior — `SIMULACION_AUTO_COMERCIO.
   reservaMaderaCaravana` (constante fija de 100) se quitó de `src/constants.ts` y
   `src/engine/simulacionAutoComercio.ts` volvió a su reserva original (atada al costo real de la caravana
   comercial, 50). El motor queda exactamente como estaba antes de las dos sesiones de batch, sin ninguna
   línea tocada. `npx vitest run src/engine/__tests__` confirmó 138/138 tras revertir.
2. **`simulaciones-batch/npcGobernanza.ts`** (nuevo, persistente — no se borra): el comportamiento
   reutilizable del NPC-solo-para-simulación que juega Gobernador/Tesorero/Rey, para no reescribirlo en cada
   batch futuro. Cubre 4 decisiones con las funciones PÚBLICAS del motor (nada de esto vive en `src/engine/`):
   infraestructura (delega en `avanzarAutoComercioSimulado`), reclutamiento (`reclutarTropa`), ataque a
   campamentos de bandidos (`atacarCampamentoBandidos`) y expansión (`lanzarCaravanaFundacion`).
3. Dentro de ese mismo archivo, dos ajustes que salieron de correr el escenario y encontrar comportamiento
   roto — documentados en detalle en el Hallazgo #5 más abajo, porque son hallazgos en sí mismos:
   - `avanzarAutoComercioSimulado` solo actúa sobre Facciones con **2+ asentamientos propios** — con 100
     Facciones de 1 asentamiento cada una, NINGUNA calificaba, así que sin este arreglo ningún asentamiento
     habría tenido Gobernador ni Tesorero en todo el batch. `npcGobernanza.ts` ahora asigna ambos cargos a
     cualquier asentamiento, sin importar el tamaño de su Facción.
   - La reserva del Tesorero (`reservaManual`) BLOQUEA toda auto-construcción nueva por debajo de su umbral
     (`puedeIniciarConstruccion`, `engine/construction.ts`) — reservar de entrada el costo COMPLETO de la
     Caravana de Fundación (varios cientos de recursos) desde el tick de fundación habría congelado la
     Vivienda/Granja/Leñera que un asentamiento recién fundado necesita para sobrevivir. La reserva ahora solo
     se activa una vez el asentamiento ya está en `nivelActual >= 2` (el gate real de expansión) — en este
     batch en particular, como se ve abajo, nunca llegó a activarse.

## Metodología

- 1 mundo (mapa 2000×2000, seed fija), **100 Facciones**, cada una funda **1 asentamiento** con **5
  jugadores fundadores**.
- Posiciones elegidas con `evaluarViabilidadFundacion(...).recomendable` (fundable Y bosque alcanzable) —
  prioridad de bosque cercano, a diferencia del batch anterior que solo exigía `fundable`.
- **100/100 fundaciones exitosas**, todas recomendables.
- **3000 ticks** (vs. 1000 del batch anterior), `avanzarSimulacion` → `avanzarNpcGobernanza` cada tick. Foto
  cada 100 ticks (31 fotos).
- **0 excepciones no capturadas** en los 3000 ticks.

## Resumen ejecutivo

| # | Hallazgo | Severidad/Tipo |
|---|---|---|
| 1 | Una Facción SÍ llegó a nivel 7 (más allá del nivel 5 pedido) por XP de combate — pero completamente desacoplada del nivel de su propio asentamiento, que nunca pasó de nivel 1 | 🟢 Responde la pregunta del usuario, con matiz importante |
| 2 | CERO expansión: ninguna Facción fundó un segundo asentamiento en 3000 ticks — nunca se cumplió el gate real (`nivelActual ≥ 2`) | 🔴 El objetivo #1 del usuario no se alcanzó |
| 3 | Colapso catastrófico: 97% a tick 3000 (3/100 vivos) — con mucha diferencia el peor resultado de los 3 batches de este tipo hechos hasta ahora | 🔴 Crítico |
| 4 | El sistema queda completamente CONGELADO desde tick ~1800 hasta el final: 1200 ticks sin un solo cambio en ninguna métrica | 🟡 Medio |
| 5 | Reclutamiento y caza de bandidos SÍ funcionan como mecánicas y como fuente de XP — pero ambos se agotan rápido y se detienen para siempre | ✅/🟡 Confirma que las mecánicas funcionan, con techo bajo |
| 6 | Gap real de diseño encontrado y corregido en el NPC (no en el motor): sin el arreglo, Facciones de 1 solo asentamiento nunca reciben Gobernador/Tesorero | 🟠 Alto (ya corregido en `npcGobernanza.ts`) |
| 7 | Cero excepciones en 3000 ticks × hasta 100 asentamientos | ✅ Positivo |

## Hallazgo #1 — Una Facción llegó a nivel 7 (supera el nivel 5 pedido), pero desacoplada de su asentamiento

`NIVEL_FACCION.xpParaNivel` exige 240 XP acumulada para nivel 5 y 600 para nivel 7. Una Facción alcanzó
**805 XP** ya para tick 300 (nivel 7) — por delante del objetivo del usuario. La XP vino casi toda de caza de
campamentos de bandidos (`combate: 5 × jugadoresParticipantes`, hasta 5 jugadores atacando juntos = 25 XP por
campamento destruido) más una fracción de `edificioCompletado`. **Pero su propio asentamiento nunca superó
nivel 1** — mismo desacople ya documentado en `Batch_500_Comercio` (Hallazgo #4: "el nivel de Facción está
totalmente desacoplado del nivel de sus asentamientos"), confirmado aquí de forma aún más extrema: una
Facción de nivel 7 con una aldea que nunca llegó ni a nivel 2.

## Hallazgo #2 — Cero expansión: el gate de nivel 2 sigue siendo la pared real

`lanzarCaravanaFundacion` exige `nivelActualDe(origen) >= 2` — y **ningún asentamiento, en ninguna de las 31
fotos tomadas, llegó nunca a nivel 2** (`nivelesAsentamiento` = 100% nivel 1 en todo el batch).
`caravanasFundacionLanzadasAcumuladas` se queda en **0** durante los 3000 ticks completos: el NPC de
gobernanza (`expandirSiPuede`) nunca tuvo NI UN SOLO asentamiento candidato al que ofrecerle un destino. Esto
confirma, a mayor escala y con una fuente de XP de combate real de por medio (no solo comercio), el mismo
techo que ya documentó `Diario_Simulaciones_Batch_100.md` (Hallazgo #1: techo de Artesanos/candado circular
de nivel 2) y que `Diario_Simulaciones_Batch_100_Asentamientos_Comercio.md` no pudo diagnosticar (Hallazgo
#5, "no se puede confirmar aquí"). Con datos algo mejores esta vez (`artesanosTotal` y
`edificiosTransformacionActivos` sí se registraron): a tick 3000, entre los 3 supervivientes hay **55
Artesanos y 3 edificios de transformación activos en total** — suficiente en NÚMERO para el gate (50
Artesanos + 3 edificios), pero sin datos por asentamiento individual no se puede confirmar si están
concentrados en uno solo (que sí debería haber promovido) o repartidos entre los 3 (ninguno cumpliría el gate
por sí solo). Candidato claro para el próximo batch: registrar el gate completo por asentamiento, no solo el
total agregado.

## Hallazgo #3 — 97% de colapso: el peor resultado de los 3 batches comparables

| Batch | Escala | Ticks | Supervivencia final |
|---|---|---|---|
| `Batch_100` (sin comercio) | 300 asentamientos (100×3 Facciones) | 900 | 47.7% |
| `Batch_500_Comercio` | 1500 asentamientos (500×3) | 900 | 33.3% |
| `Batch_100_Asentamientos_Comercio` (20 Facciones×5) | 100 asentamientos | 1000 | 58% |
| **Este batch** (100 Facciones×1) | 100 asentamientos | 3000 | **3%** |

La caída es brutal y rápida: 100→23 vivos a tick 100 (77% muerto en la ventana crítica de siempre, justo tras
la gracia de Mantenimiento), →8 a tick 200, →6 a tick 300, →5 a tick 400-500, →4 hasta tick 1700, →3 desde
tick 1800 hasta el final. **No se puede aislar con este batch una única causa** — hay al menos dos factores
que cambiaron a la vez respecto al batch anterior, y no se corrió una versión de control con cada uno por
separado:
- **Sin red de comercio desde el arranque**: con 1 solo asentamiento por Facción, `avanzarAutoComercioSimulado`
  nunca empareja déficit/superávit (exige 2+ propios) — a diferencia del batch anterior (20×5), aquí ningún
  asentamiento tuvo NUNCA a quién pedirle un recurso que le faltara.
- **Reclutamiento activo compite por la misma población que la auto-construcción necesita**: cada
  `milicia_lanceros` reclutado saca de golpe 25 Pesants de la población civil (tope 25 por jugador, hasta 125
  por asentamiento si los 5 residentes reclutan) — justo en la ventana de tick 0-100 donde la supervivencia ya
  es más frágil según los 3 diarios anteriores.

Ambos son candidatos razonables, ninguno confirmado por separado — un batch de seguimiento con
reclutamiento apagado (todo lo demás igual) aislaría cuánto pesa cada uno.

## Hallazgo #4 — El sistema queda completamente congelado desde tick ~1800

Desde tick 1800 hasta tick 3000 (1200 ticks, el 40% de la duración total del batch), **absolutamente ninguna
métrica cambia**: 3 vivos, 55 Artesanos, 3 edificios de transformación, 375 tropas vivas, 382 reclutamientos
acumulados, 35 campamentos destruidos acumulados, XP máxima de Facción congelada en 805 — todo idéntico foto
tras foto. Los 3 supervivientes llegaron a un estado estable donde ya no hay más reclutamiento posible (todos
los residentes con escuadrón ya al tope de 25 unidades — `reclutarTropa` empieza a fallar por "ya está al
tope"), aparentemente tampoco más campamentos que atacar con éxito (no se registró `campamentosAtacadosSinExito`
en este batch — gap de instrumentación, ver Nota metodológica), y sin ninguna vía para acumular más XP de
Facción una vez agotadas esas dos fuentes. Es un estado terminal ESTABLE, no un colapso lento — los 3
sobrevivientes no mueren, simplemente dejan de progresar.

## Hallazgo #5 — Reclutamiento y caza de bandidos funcionan, pero se saturan rápido

- **382 reclutamientos exitosos acumulados** (sobre hasta 500 posibles = 100 asentamientos × 5 jugadores, si
  todos hubieran llegado a reclutar antes de colapsar) — confirma que `milicia_lanceros` (edificio
  `centroUrbano`, sin depender de Barracón) es reclutable desde el primer tick, tal como se documentó en el
  plan.
- **35 campamentos de bandidos destruidos acumulados**, todos antes de tick 300 — después de eso, cero
  destrucciones más en 2700 ticks, coincidiendo con el congelamiento general del Hallazgo #4.
- Ambas fuentes de XP funcionan de punta a punta (es lo que llevó a una Facción a nivel 7, Hallazgo #1), pero
  ninguna de las dos es una fuente SOSTENIDA a esta escala: se agotan en las primeras ~300 ticks y no hay
  ningún mecanismo en este escenario que las reactive (la población deja de crecer lo bastante rápido para
  reponer tropas por encima del tope, y/o los campamentos dejan de aparecer o de poder vencerse — no se
  puede distinguir cuál con los datos registrados).

## Hallazgo #6 — Gap de diseño real: Facciones de 1 solo asentamiento nunca reciben Gobernador/Tesorero

Encontrado y corregido en `npcGobernanza.ts` (no en el motor, ver sección de cambios de código arriba):
`avanzarAutoComercioSimulado` (`engine/simulacionAutoComercio.ts`) fue escrito para el escenario de comercio
intra-Facción del batch anterior y solo actúa sobre Facciones con **2 o más** asentamientos propios. Con 100
Facciones de 1 asentamiento cada una, sin este arreglo NINGUNA habría tenido nunca Gobernador ni Tesorero —
ni reserva, ni Mercado, ni nada de lo que depende de esos cargos — durante todo el batch. Vale la pena
señalarlo porque es un supuesto implícito del módulo de comercio (pensado para Facciones ya establecidas con
varias ciudades) que no se sostiene en un escenario centrado en el arranque/expansión desde cero — cualquier
batch futuro con Facciones nuevas de 1 asentamiento necesita este mismo parche del NPC.

## Hallazgo #7 — Motor robusto también en este escenario

Cero excepciones no capturadas en 3000 ticks sobre hasta 100 asentamientos simultáneos, con reclutamiento,
combate contra bandidos y (en potencia, aunque nunca se disparó) expansión corriendo cada tick.

## Nota metodológica

Reversión confirmada limpia (paso 1). El único código nuevo de esta sesión vive en `simulaciones-batch/
npcGobernanza.ts` (persistente, para reusar) — el escenario en sí (mundo, posiciones, bucle de ticks, fotos)
fue un test temporal en `src/engine/__tests__/`, borrado tras generar este diario, igual criterio que los
batches anteriores. Dos limitaciones de instrumentación para tener en cuenta en el próximo batch: (1) no se
registró Gobernador/Tesorero asignados por foto (a diferencia de los batches anteriores), así que el
Hallazgo #6 se confirma por revisión de código, no por dato observado; (2) no se registró
`campamentosAtacadosSinExito` ni el gate de nivel 2 desglosado POR asentamiento, lo que habría permitido
diagnosticar mejor los Hallazgos #2 y #4.

## Sugerencia de siguiente paso

Dos preguntas quedaron abiertas y son directamente accionables con otro batch, sin tocar el motor:
1. **Aislar el efecto del reclutamiento sobre la supervivencia** (Hallazgo #3): repetir este mismo escenario
   con `reclutarParaTodos` desactivado en la config del NPC, todo lo demás igual, y comparar la curva de
   colapso contra este batch.
2. **Diagnosticar el gate de nivel 2 por asentamiento** (Hallazgo #2): registrar en cada foto, para cada
   asentamiento vivo, sus Artesanos y qué edificios de transformación tiene activos — para saber si el
   candado es "está cerca pero repartido entre varios" o "ninguno individual se acerca ni de lejos".
