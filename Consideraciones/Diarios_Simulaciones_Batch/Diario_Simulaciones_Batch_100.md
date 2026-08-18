# Diario de descubrimientos — batch de 100 simulaciones (900 ticks)

**Fecha:** 2026-08-08
**Qué es esto:** resultados de correr el motor de simulación (`src/engine/*`) tal cual está hoy, sin
tocar ni una línea de código — solo un script de prueba externo al repo que importa las funciones del
motor para fundar asentamientos, activar políticas y avanzar ticks mecánicamente. El objetivo era
estresar el estado ACTUAL (post rediseño de progreso + reclutamiento por equipo, ver entradas #28-31 de
`Correcciones_Durante_Desarrollo.md`) con muchas combinaciones de semilla/ubicación/políticas para ver
qué patrones emergen a escala que no se ven jugando manualmente unos pocos ticks.

Este documento es un diario de OBSERVACIONES, no de decisiones — a diferencia de
`Correcciones_Durante_Desarrollo.md`, aquí no se ha aplicado ningún cambio de código. Si algo de esto
amerita una corrección real, se decide y se registra allí en una sesión aparte.

## Metodología

- **100 runs independientes**, cada uno con su propio `seed` de mundo (`generateWorld`) y sus propias
  posiciones de fundación — mundo y ubicaciones distintos en cada run.
- Cada run funda **1 asentamiento por Facción** (Micenas/Troya/Ugarit, las 3 del `gameStore` real) = 3
  asentamientos por run, 300 en total sobre el batch.
- Cada asentamiento se funda con **5 jugadores**, y los 5 reciben cada uno un cargo local distinto
  (Gobernador/Tesorero/General/Maestro de Obras/Sacerdote) — más Rey/Embajador a nivel de Facción.
- Al fundar, cada cargo activa políticas aleatorias hasta llenar sus slots disponibles (Gobernador elige
  de la pool completa, el resto solo de la suya) — la combinación de políticas activas varía de run a run.
- Cada run avanza **900 ticks** llamando `avanzarSimulacion` directamente (sin intervención manual
  después de fundar) y se toma una foto de cada asentamiento vivo cada 30 ticks.
- **Importante — alcance real de la prueba:** esto solo ejercita el bucle AUTÓNOMO de simulación
  (auto-construcción, población, políticas, mantenimiento, crecimiento de zona, nivel de Facción,
  reputación, títulos). NO ejercita acciones manuales de jugador (reclutar tropa, combate, trueque,
  colocar orden de mercado, diplomacia) porque el script no las invoca — cualquier estadística de
  "0 reclutamientos" más abajo es un artefacto del alcance de la prueba, no un hallazgo sobre el sistema
  de reclutamiento en sí.
- Motor sin tocar: el script solo importa `engine/*` — no se modificó ningún archivo fuente para esta prueba.

## Resumen ejecutivo

De los 3 hallazgos, el **#1 es el más importante con diferencia**: es una condición matemática, no de
mala suerte de mapa, y explica por sí sola por qué **0 de los 300 asentamientos simulados llegó nunca a
nivel 2** en 900 ticks, sin importar ubicación, semilla o políticas activas.

| # | Hallazgo | Severidad |
|---|---|---|
| 1 | Nivel 2 de asentamiento es matemáticamente inalcanzable con los números actuales (deadlock circular) | 🔴 Crítico |
| 2 | ~52% de colapso general; asentamientos sin bosque a distancia ≤30 colapsan casi siempre hacia el tick ~85 | 🔴 Crítico (ya documentado como riesgo aceptado, pero la escala real es mucho mayor de lo asumido) |
| 3 | Interbloqueo de cola "general": un proyecto que nunca puede completarse (por recurso geográficamente inalcanzable) ocupa su slot para siempre y bloquea a los demás | 🟠 Alto |
| 4 | Ruido de punto flotante: `cuero` negativo por ~4.4e-16 en descuentos exactos | ⚪ Cosmético |
| 5 | Cero excepciones del motor en 90.000 avances de tick — el bucle de simulación en sí es robusto | ✅ Positivo |

## Hallazgo #1 — Nivel 2 de asentamiento es matemáticamente inalcanzable (crítico)

**El dato:** de los 300 asentamientos simulados (100 runs × 3), **ninguno alcanzó nivel 2** en 900 ticks.
Ni uno. Incluyendo los 33 casos "ideales" donde Fundición + Curtiduría + Armería (las 3 exigidas por el
gate de nivel 2) estaban las tres activas a la vez, con Pesants muy por encima del umbral (528-708 vs. los
200 requeridos).

**Por qué:** `NIVEL_ASENTAMIENTO.requisitos[2]` (`constants.ts`) exige `artesanos: 50` además de las 3
edificaciones. Pero la población de Artesanos tiene un TECHO duro —`capacidadArtesanos` en
`asentamientoQuery.ts`— igual a la suma de `trabajadoresRequeridos` de los edificios de transformación
ACTIVOS, evaluados en su `nivelInterno` actual. En nivel interno 1 (el único alcanzable sin subir de
nivel de asentamiento):

| Edificio | `trabajadoresRequeridos` nivel interno 1 |
|---|---|
| Fundición | 4 |
| Curtiduría | 4 |
| Armería | 4 |
| Carpintería | 0 (y además exige asentamiento nivel ≥2 solo para EMPEZAR a construirla) |
| **Techo máximo posible de Artesanos** | **12** |

Confirmado en los datos: entre los 300 asentamientos del batch, el máximo de Artesanos observado en
CUALQUIER momento fue exactamente **12** (media 2.95, incluyendo los que nunca construyeron nada).

Subir esos edificios a nivel interno 2 (que sí eleva `trabajadoresRequeridos` a 8 cada uno) exige
`requisitoNivelAsentamiento: 2` — es decir, hace falta ya estar en nivel de asentamiento 2 para poder
mejorar los edificios que elevarían el techo de Artesanos por encima de 12. Y llegar a nivel de
asentamiento 2 exige 50 Artesanos. Es un candado circular cerrado sobre sí mismo: **el techo (12) es
permanentemente menor que el requisito (50), y la única forma de subir el techo requiere haber cruzado
el requisito primero.**

Esto no depende de mapa, semilla, ubicación ni políticas — es aritmética fija en `constants.ts`. Pasa en
el 100% de los casos, incluidos los mejor posicionados.

**Consecuencia en cascada:** al no alcanzarse nunca nivel 2, tampoco se alcanza nunca nivel 3 (Fase 0
tiene un tope de 3, y la subida es monótona gate-por-gate desde el nivel actual), por lo que Carpintería
(exige nivel 2), Palacio (exige nivel 3) y Nobleza (exige Palacio) quedan inalcanzables en la práctica
para cualquier partida de duración similar a este batch — consistente con los datos: Nobleza en 0/300,
Palacio en 0/300, Carpintería en 0/300.

**No es un problema de "el placeholder de 50 es alto"** en abstracto — es que 50 es *estructuralmente*
mayor que el techo que el propio sistema permite alcanzar antes de necesitar ese mismo 50. Cualquier
número de Artesanos requeridos por encima de 12 tiene este mismo problema mientras la única vía de subir
el techo (mejorar de nivel interno) siga gateada por el nivel de asentamiento que ese requisito bloquea.

## Hallazgo #2 — Riesgo de "sin bosque cercano" (ya documentado, pero mucho más frecuente de lo asumido)

La entrada #31 de `Correcciones_Durante_Desarrollo.md` documentó este riesgo como aceptado, enmarcado
como el caso de "un asentamiento mal ubicado". Los datos del batch muestran que no es un caso raro — es
básicamente una moneda al aire determinada por la densidad de bosques del mapa (25 bosques de radio
30-80 sobre 1000×1000):

| Distancia inicial al bosque más cercano | Asentamientos | Colapsados | % colapso |
|---|---|---|---|
| Dentro de un bosque (≤0) | 76 | 0 | 0% |
| 0-30 (dentro del radio inicial) | 72 | 6 | 8.3% |
| 30-60 (alcanzable solo si la zona crece) | 61 | 60 | **98.4%** |
| 60-120 | 68 | 68 | **100%** |
| >120 | 23 | 23 | **100%** |

**Total: 157/300 (52.3%) de los asentamientos simulados cayeron en ruinas** — el 90% de esos colapsos
ocurrió entre el tick 79 y el tick 100 (justo al terminar la gracia de Mantenimiento de 60 ticks +
margen), todos en nivel 1. En 14 de los 100 runs, los 3 asentamientos de la partida colapsaron. Solo en
10 de los 100 runs sobrevivieron los 3.

Esto no contradice la decisión ya tomada (el usuario aceptó el riesgo conscientemente), pero conviene
que la magnitud real quede registrada: con la densidad de bosques actual, fundar sin comprobar
visualmente la cercanía a un bosque tiene ~9 de cada 10 probabilidades de acabar en colapso si la
posición cae a más de 30 unidades de cualquier bosque — no es un caso límite raro, es el resultado más
probable de una fundación "a ciegas".

## Hallazgo #3 — Interbloqueo de cola "general" cuando el recurso bloqueante es inalcanzable (no solo escaso)

Trazado en detalle sobre un asentamiento reproducible (run 1, `asentamiento-0-927-457`, con bosque a
distancia 17 pero SIN piedra alcanzable en su zona): Curtiduría (`{madera:80, piedra:30}`) y Almacén
(`{madera:50, piedra:30}`) entran en cola (`en_cola`) en el tick 15-16 y **se quedan ahí, literalmente sin
moverse, durante 884 ticks seguidos, hasta el tick 900** — porque la piedra se queda fija en 20 (el
inicial de fundación) para siempre: no hay ningún nodo de piedra dentro de una zona que, a su vez, nunca
vuelve a crecer (ver Hallazgo #1/#3 combinados). Mientras tanto la madera se acumula sin usarse hasta el
tope del almacén (200, luego estable en 197-200 durante 800+ ticks).

El problema no es solo que esos dos proyectos nunca se completen — es que, al quedarse en estado
`en_cola`, **siguen ocupando uno de los 2 slots "generales" de la cola de construcción
(`NECESIDADES.maximoEnCola - slotsReservadosSupervivencia - slotsReservadosExtractores`) para siempre**,
bloqueando silenciosamente que cualquier OTRA necesidad de tipo general (más Vivienda, un segundo
Almacén, etc.) pueda encolarse alguna vez, aunque haya madera de sobra disponible.

Es la misma familia de interbloqueo que ya se documentó y arregló dos veces antes (Granja/Leñera
compitiendo por slots, y Curtiduría/Armería compitiendo con Cantera por slots — ambos en
`Correcciones_Durante_Desarrollo.md`), pero con una diferencia importante: aquellos arreglos asumían que
el recurso bloqueante era *escaso pero eventualmente disponible*. Aquí el recurso (piedra) puede ser
*permanentemente inalcanzable* para ese asentamiento concreto (sin nodo en su zona, y la zona no puede
crecer más — hallazgo #1), así que la reserva de slots no ayuda: el proyecto nunca decide "renunciar" y
liberar su slot.

Consistente con esto: **los 143 asentamientos que sobrevivieron a tick 900 tienen el `radioPotencial`
IDÉNTICO entre el tick 300 y el tick 900** (143/143, sin una sola excepción) — el crecimiento de zona
(ligado a completar edificios, corrección #28) se detiene por completo una vez agotadas las
oportunidades "fáciles" (Leñeras alcanzables + lo que la política de turno desbloqueó), y no hay ningún
mecanismo que lo reactive.

## Hallazgo #4 — Ruido de punto flotante (cosmético)

En 4 de los 100 runs se detectó `almacen.cuero` con un valor negativo del orden de `-4.44e-16` justo
después de consumir cuero hasta dejarlo en (casi) cero. Es el clásico error de redondeo de `Number` en
JS al restar dos floats casi iguales — no tiene ningún efecto observable (ninguna lógica del motor
compara `> 0` de forma estricta contra esto de forma que rompa algo), pero si se quiere prolijidad,
`descontarRecursos` (`engine/almacen.ts`) podría clampear a `Math.max(0, resultado)`.

## Hallazgo #5 — El bucle de simulación en sí es robusto (positivo)

Cero excepciones no capturadas en 90.000 avances de tick (100 runs × 900 ticks) sobre 300 asentamientos
con combinaciones aleatorias de ubicación y políticas, y cero valores `NaN`/`Infinity` detectados salvo
el ruido de punto flotante ya mencionado. El nivel de Facción sí avanza con normalidad (313 subidas de
nivel de Facción registradas en el batch) porque ese sistema no depende de los mismos gates de
edificios/Artesanos que el nivel de asentamiento — buena señal de que el problema es localizado
(el gate de nivel de asentamiento) y no un problema estructural del motor.

## Otros datos del batch

- **Reclutamiento de tropas: 0/300** — esperado dado el alcance de la prueba (ver Metodología), no es un
  hallazgo sobre `reclutarTropa`.
- **Correlación políticas activas ↔ colapso:** ninguna política se aparta de forma significativa del
  ~48-56% de colapso general (`comercio_abierto` 53.5%, `via_rapida` 51.8%, `proteccion_riesgos` 52.1%,
  etc.) — coherente con que los dos modos de fallo dominantes (bosque lejano, techo de Artesanos) no son
  cosas que ninguna política actual del catálogo pueda mitigar.
- **Mantenimiento en los 143 supervivientes:** medidor en 100/100 en todos los casos a tick 900 (ninguno
  quedó en déficit sostenido) — el Mantenimiento en sí funciona bien una vez superada la ventana crítica
  de colapso temprano (~tick 85-100); el problema no es de balance de Mantenimiento sino de progresión.

## Sugerencia de siguiente paso

Ninguno de estos hallazgos se ha corregido — quedan aquí para que se decida en una sesión aparte, mismo
criterio que las correcciones #28-31. El más urgente de revisar es el #1 (deadlock circular de nivel 2):
sin resolverlo, Fase 0 no tiene forma de que ningún asentamiento progrese más allá del nivel inicial en
una duración de partida razonable, lo cual toca directamente Nobleza, Palacio, Carpintería y las mejoras
de nivel interno de todos los edificios de transformación.
