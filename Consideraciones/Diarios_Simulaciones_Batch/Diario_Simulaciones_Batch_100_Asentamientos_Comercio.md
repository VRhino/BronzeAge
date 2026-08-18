# Diario de descubrimientos — 100 asentamientos simultáneos con auto-comercio activo (1000 ticks)

**Fecha:** 2026-08-18
**Qué es esto:** a diferencia de [Diario_Simulaciones_Batch_100.md](Diario_Simulaciones_Batch_100.md) y
[Diario_Simulaciones_Batch_500_Comercio.md](Diario_Simulaciones_Batch_500_Comercio.md) (cientos de runs
INDEPENDIENTES de 3 asentamientos cada uno), esto es **un único run con 100 asentamientos coexistiendo en el
mismo mundo**, repartidos en 20 Facciones de 5 asentamientos cada una, corriendo 1000 ticks seguidos con
`engine/simulacionAutoComercio.ts` activo — el módulo NPC-solo-para-simulación que asigna Gobernador y
Tesorero, construye Mercado + caravana propia, calibra una reserva manual de madera, y propone trueques
automáticos por déficit/superávit dentro de cada Facción (ver el propio archivo para el detalle del
mecanismo). Petición explícita del usuario: montar este escenario y ver "hasta dónde llegan".

**Script de prueba TEMPORÁNEO**, igual criterio que los diarios anteriores (importa el motor real, no lo
reimplementa) — se escribió, se corrió y se borró tras extraer los datos de este diario; no queda en el repo.

## Cambio de código real hecho para este escenario (a diferencia de los dos diarios anteriores)

Los dos diarios previos corrieron el motor "tal cual, sin tocar ni una línea". Este esta vez sí incluye **un
cambio de configuración real**, a petición explícita del usuario ("reserva de 100 de madera"):
`SIMULACION_AUTO_COMERCIO.reservaMaderaCaravana` (`src/constants.ts`) — antes la reserva manual de madera que
este módulo calibra estaba atada exactamente al costo de la caravana comercial (`CARAVANA_CATALOGO.comercial.
costoConstruccion.madera` = 50); ahora es un valor fijo de **100**, con margen para reponer el gasto de una
caravana y seguir teniendo colchón para la siguiente. `engine/simulacionAutoComercio.ts` se actualizó para
leer la nueva constante. El interruptor `SIMULACION_AUTO_COMERCIO.activo` sigue en `0` (apagado) en el
archivo — solo se puso en `1` dentro del proceso del script de prueba, nunca en el repo.

## Metodología

- **1 solo mundo** (`generarMapa`/`crearMapa`, seed fija, mapa 2000×2000 actual).
- **20 Facciones**, cada una fundada con `nivel` puesto a mano en 6 (solo para pasar el cap de fundación
  `CAP_FUNDACION_POR_NIVEL` durante el setup — en cuanto arranca el bucle de ticks, `avanzarNivelesFaccion`
  recalcula `nivel` desde `experiencia`, que arranca en 0, así que todas vuelven a nivel 1 real en el primer
  tick; el cap alto solo sirvió para poder fundar los 5 asentamientos por Facción de una sola vez).
- **5 asentamientos por Facción × 20 Facciones = 100**, cada uno con **1 solo jugador fundador** (a diferencia
  de los diarios anteriores que usaban 5 jugadores con los 5 cargos repartidos — aquí el único cargo que
  importa para el escenario, Gobernador+Tesorero, lo asigna el propio `simulacionAutoComercio` automáticamente
  al primer fundador). Posiciones en grilla regular espaciada (candidatas filtradas solo por `fundable` —
  dentro del mapa, libre, terreno no-cima — **sin** exigir bosque alcanzable a diferencia de los diarios
  anteriores, ver nota de comparación en el Hallazgo #2).
- **100/100 fundaciones exitosas**, 0 posiciones descartadas por invalidez.
- **1000 ticks**, llamando `avanzarSimulacion` y, a continuación en el mismo tick, `avanzarAutoComercioSimulado`
  (mismo orden que usa `gameStore.avanzarTick` en el juego real cuando el flag está activo). Foto cada 50 ticks.
- **0 excepciones no capturadas** en los 1000 ticks (sobre hasta 100 asentamientos vivos por tick).

## Resumen ejecutivo

| # | Hallazgo | Tipo |
|---|---|---|
| 1 | El mecanismo pedido funciona al 100%: todo asentamiento que sobrevive termina con Gobernador+Tesorero+Mercado+reserva de 100 madera, típicamente en menos de 50 ticks | ✅ Confirma que el mecanismo funciona |
| 2 | 58% de supervivencia a tick 1000 (42/100 colapsados) — sensiblemente MEJOR que los dos diarios anteriores (47.7% y 33.3%), pese a fundar sin filtrar por cercanía a bosque | 🟢 Positivo, con matices |
| 3 | El trueque automático genera actividad sostenida y creciente durante los 1000 ticks: 260 trueques cumplidos, tasa de éxito 52.6% entre los resueltos, sin señales de estancarse | ✅ Confirma que el comercio funciona a esta escala |
| 4 | La reserva de 100 madera (recién pedida) y el colchón de excedente del 30% (ya existente) NO están coordinados: ~50% de los asentamientos vivos tienen MENOS de 100 de madera en cualquier foto tomada después del arranque, de forma sostenida, no transitoria | 🟠 Medio — hallazgo estructural real |
| 5 | Ningún asentamiento alcanzó nivel 2 en 1000 ticks, pese al comercio activo y al cupo de Facción disponible | 🟡 Medio — no diagnosticado en profundidad en este batch |
| 6 | Cero excepciones del motor en 100.000 avances de tick-asentamiento acumulados | ✅ Positivo |

## Hallazgo #1 — El mecanismo pedido funciona al 100% en los supervivientes

En la foto de tick 50 (100 asentamientos, ninguno colapsado todavía), **los 100/100 ya tenían Gobernador y
Tesorero asignados y reserva manual de madera calibrada en 100** — el módulo resuelve esto en muy pocos ticks
tras fundar (asigna ambos cargos y calibra la reserva en la misma pasada, ver
`asegurarInfraestructuraComercial`). El Mercado (que sí tarda 8 ticks de construcción) ya estaba activo en
70/100 a tick 50. De ahí en adelante, en **todas las fotos siguientes, el 100% de los asentamientos VIVOS**
(sin excepción, en las 21 fotos tomadas) tiene Gobernador+Tesorero+Mercado+reserva — el mecanismo no se cae
nunca una vez establecido, incluso mientras otros asentamientos colapsan a su alrededor. La caravana propia
tarda un poco más en generalizarse (69/100 a tick 50, estabiliza entre 47 y 55 del total de vivos de ahí en
más — casi siempre por encima del 85% de los vivos).

## Hallazgo #2 — 58% de supervivencia, mejor que los dos batches anteriores (con una salvedad metodológica)

| Batch | Runs/escala | Ticks | Supervivencia | Filtró por bosque al fundar |
|---|---|---|---|---|
| `Batch_100` (sin comercio) | 300 asentamientos (100×3) | 900 | 47.7% | Sí (`posicionRecomendable`) |
| `Batch_500_Comercio` (agente de prueba simple) | 1500 asentamientos (500×3) | 900 | 33.3% | Sí |
| **Este batch** (auto-comercio real, 1 run) | 100 asentamientos | 1000 | **58%** | **No** — solo `fundable` |

La caída fuerte ocurre en la misma ventana que documentan los dos diarios anteriores: 100→68 vivos entre
tick 50 y tick 100 (justo al terminar la gracia de Mantenimiento de 60 ticks), y tras eso el número se
estabiliza rápido — 60 vivos entre tick 250 y 600, bajando muy despacio a 58 hacia el final (solo 2 colapsos
más en los últimos 700 ticks). El grueso de la mortalidad, otra vez, ocurre en la primera centena de ticks.

Que la supervivencia sea MEJOR aquí que en los dos batches anteriores es notable justamente porque **este
batch no exigió bosque alcanzable al elegir dónde fundar** (los anteriores sí, y aun así colapsaron más) — no
se puede aislar la causa exacta con los datos de este batch (no se registró distancia a bosque por
asentamiento), pero encajan como candidatos plausibles, sin confirmar: (a) el Mercado ya no pide piedra para
la construcción BASE (cambio de esta misma sesión, ver `constants.ts`, corrige el deadlock de piedra que
documentaba el diario `Batch_100` original), (b) la reserva de madera de 100 puede estar dando margen extra
frente al Mantenimiento en el tramo crítico, (c) el trueque de recursos crudos (piedra/cobre/estaño/oro/
livestock) mueve exactamente lo que un asentamiento sin nodo cercano necesitaría para no quedarse en déficit.
Se necesitaría un batch dedicado con/sin cada uno de estos factores por separado para confirmar cuál pesa más.

## Hallazgo #3 — El trueque automático SÍ mueve volumen real y sostenido a esta escala

| Métrica a tick 1000 | Valor |
|---|---|
| Trueques propuestos (activos+cumplidos+expirados) | 545 |
| Trueques cumplidos | 260 (47.7% del total propuesto) |
| Trueques expirados | 234 |
| Trueques todavía activos | 51 |
| Tasa de éxito entre RESUELTOS (cumplido vs. expirado) | 52.6% |
| Caravanas comerciales construidas (activo histórico) | 91 |

El ritmo de trueques cumplidos crece de forma prácticamente lineal desde tick 300 en adelante (71 → 89 → 102
→ 116 → 129 → ... → 260 a tick 1000, +189 en 700 ticks, ~1 trueque completado cada ~3.7 ticks en promedio
sobre toda la economía de 58-60 asentamientos) — sin señales de desacelerar ni saturarse hacia el final del
run. La tasa de éxito (52.6%) es del mismo orden que el 57.1% que documentó `Batch_500_Comercio` con su
agente de prueba simple (ese medía Trueque + Mercado juntos con un agente distinto, así que no es una
comparación exacta, pero confirma que el motor de trueque real sostiene una tasa de cumplimiento similar
también dentro del emparejamiento déficit/superávit intra-Facción de `simulacionAutoComercio`).

## Hallazgo #4 — La reserva de 100 madera y el colchón de excedente (30%) no están coordinados

Este es el hallazgo más interesante desde el punto de vista de diseño. El usuario pidió la reserva de 100
madera específicamente **"para que puedan crear sus caravanas sin problema"** — y en efecto,
`asegurarInfraestructuraComercial` calibra `reservaManual.madera = 100`, lo cual bloquea que la
auto-CONSTRUCCIÓN (Vivienda/Granja/Leñera, etc.) consuma madera por debajo de esa marca.

Pero el propio módulo de auto-comercio usa un mecanismo DISTINTO y más permisivo para decidir cuánta madera
puede OFRECER como pago en un trueque: `mejorRecursoDePago` solo exige que la madera esté por encima del
**30% de la capacidad de almacén** (`SIMULACION_AUTO_COMERCIO.colchonExcedente`, capacidad base 200 ⇒ umbral
real ≈ 60 unidades) — un número menor que la reserva de 100 recién pedida. Resultado: el propio comercio
automático puede (y de hecho lo hace) gastar madera de un asentamiento hasta dejarlo entre 60 y 99 unidades,
por debajo de su propia reserva.

En los datos: el número de asentamientos con madera por debajo de 100 se mantiene estable entre **28 y 35 de
los 58-68 vivos en cada foto desde tick 100 en adelante** (~48-50% de los vivos, en TODAS las fotos, no solo
al principio) — pese a que el promedio de madera de la población sube sin parar (de 97.5 a tick 100 hasta
214.2 a tick 1000). No es un problema transitorio de arranque: es un patrón estable donde, en cualquier
momento dado, la mitad de los asentamientos vivos tiene menos madera que su propia reserva declarada — porque
la reserva protege contra la auto-construcción pero no contra el propio trueque que la calibró. Si la
intención es que 100 de madera sea un piso de verdad (no solo "protegido de la construcción automática"), el
`colchonExcedente` del trueque tendría que respetar `reservaManual` también, no solo su propio 30% fijo —
punto de diseño para decidir en sesión aparte, no corregido aquí.

## Hallazgo #5 — Nivel de asentamiento: 0/58 alcanzó nivel 2 en 1000 ticks

Pese a 1000 ticks, comercio activo y madurando, y cupo de Facción disponible en teoría (12 de las 20
Facciones ya en nivel de Facción ≥2 a tick 1000, lo cual habilita `CUPO_NIVEL_ASENTAMIENTO.maxNivel2 ≥ 1`
para ellas), **ningún asentamiento de los 58 supervivientes llegó nunca a nivel 2** — los 58/58 siguen en
nivel 1 en la última foto. El diario `Batch_100` original documentó un candado circular específico (techo de
Artesanos ≤12 vs. requisito de 50) que, por los cambios de esta sesión (`POBLACION.artesanos.
tasaCrecimientoBase` subido y el techo numérico de Artesanos retirado, según el diff de `constants.ts`/
`population.ts` visto en el working tree), puede que ya no aplique tal cual — pero este batch no registró
población de Artesanos ni edificios de transformación activos por asentamiento, así que **no se puede
confirmar aquí si el motivo es el mismo candado, uno nuevo, o simplemente que 1000 ticks siguen sin ser
suficientes** para que Fundición/Curtiduría/Armería se construyan Y la población de Artesanos llegue a 50 en
paralelo. Candidato claro para un batch de seguimiento que sí registre esas dos variables por asentamiento.

## Hallazgo #6 — Motor robusto también con 100 asentamientos coexistiendo

Cero excepciones no capturadas en 1000 ticks sobre una población que llegó a tener hasta 100 asentamientos
vivos simultáneos (100.000 avances de tick-asentamiento acumulados), con hasta 120 trueques activos a la vez
y 91 caravanas comerciales circulando — ninguna combinación de estado produjo un error del motor.

## Nota metodológica

A diferencia de los dos diarios anteriores, este SÍ incluye el cambio de configuración descrito arriba
(`reservaMaderaCaravana: 100`), a petición explícita del usuario — el resto del motor no se tocó. El script
de prueba (temporal, borrado tras generar este diario) fundó sin exigir bosque alcanzable, a diferencia de
los diarios anteriores — ver la salvedad del Hallazgo #2. `SIMULACION_AUTO_COMERCIO.activo` permanece en `0`
en el repo; el batch lo puso en `1` solo dentro de su propio proceso.

## Sugerencia de siguiente paso

Ninguno de los hallazgos #2/#4/#5 se ha corregido — quedan aquí para decidir en sesión aparte. Los más
accionables: **#4** (decidir si `colchonExcedente` del trueque debe respetar `reservaManual`, ya que hoy la
reserva pedida por el usuario no es un piso real frente al propio comercio que la usa) y **#5** (un batch de
seguimiento que registre Artesanos y edificios de transformación por asentamiento para saber si el candado de
nivel 2 sigue vigente tras los cambios de esta sesión).
