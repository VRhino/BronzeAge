# Modelo temporal: relojes, determinismo y conversión a tiempo real

Referencia del **tiempo** en BronzeAgeFase0. Se escribe al arrancar la Fase D (2026‑08‑29) porque esa fase
toca precisamente la maquinaria temporal y **no había ningún modelo escrito** — solo `tick: number` por todas
partes y una promesa en el [doc 2](2_Estudio_Evolucion_Backend_Multifrontend.md) ("el motor será independiente
del tamaño del paso") que, al medirla, resultó inalcanzable.

Es una **referencia, no un plan**. Los hitos que la ejecutan son D1–D6 (ver [doc 3](3_Plan_Evolucion_Roadmap.md)
y [doc 4](4_Plan_Evolucion_Tareas.md)). El criterio de "qué es regla y qué es simulación" vive en el
[doc 9](9_Reglas_vs_Simulacion.md); este documento es su eje temporal.

> **Estado a 2026‑08‑30 (verificado contra `src/`).** Fase D **cerrada**: D1–D6 y el cierre de contrato están
> implementados y en verde (685 tests). En el código: `SIMULACION` en `constants.ts`; `instanteDeTick` deriva
> el `instante` del `tick` y nunca se almacena; `domain/tiempo.ts` con `Instante`/`Duracion` branded; el guard
> [`autoridadTemporal.test.ts`](../../src/__tests__/autoridadTemporal.test.ts) activo; snapshot migrado hasta
> **v5** (`tick` fuera del contrato de eventos); reloj de mundo + catch‑up en `RunnerDePartida`. Los dos bugs
> de §7 están cerrados con regresión congelada.
>
> **Pendiente, y fuera del alcance de D a propósito:**
> - **La pasada de rebalanceo en tiempo** (§8, último párrafo). D1–D6 preservaron los valores; con 1 tick =
>   1 min real varios siguen calibrados para "abstracto" (p. ej. población compuesta ~12 %/min). Es el
>   siguiente esfuerzo, apoyado en el laboratorio batch.
> - **El scheduler de comandos programados** (D5): asedios formales y caravanas planificadas. Aterriza con su
>   primera mecánica de Fase 1+, no antes.
> - **El log de comandos** (§5): queda para Fase E (auditoría); `PartidaExportada.estadoRng` se mantiene sin
>   más inversión hasta que un incidente real lo pida.
>
> **Al día 2026-09-15.** Snapshot en **v18**, sin cadena de migraciones (se retiró el 2026-09-09; las subidas
> posteriores no migran). `ContextoComando` lleva además `batallasEnUnity`, y `ContextoSimulacion` las
> `batallas`. Las batallas de Unity usan dos relojes a propósito: sus plazos (`Batalla.expiraEn`) son tiempo de
> mundo, y los tokens de entrada caducan en tiempo real UTC (`Docs/Coordinacion/01` §0), porque una credencial no
> debe seguir valiendo durante una caída del proceso.

---

## 1. El error de vocabulario que hay que deshacer primero

"Determinismo" venía nombrando **tres propiedades distintas** fundidas en una palabra y en un único
presupuesto de esfuerzo (Fases A3 + B3). No se implican entre sí:

| | Qué es | Estado |
|---|---|---|
| **P1 · Autoridad temporal** | Qué reloj gobierna las reglas del juego: el tiempo de mundo, nunca `Date.now()` | **Roto hoy**, se repara en D1. No es opcional |
| **P2 · Motor reproducible con seed** | `avanzarSimulacion` = función pura de (estado, contexto). RNG inyectado, sin reloj | **Sano y ya amortizado** — lo consume el laboratorio de balance |
| **P3 · Invarianza de paso** | Que 6 pasos de 10 s equivalgan a 1 de 60 s | **Inalcanzable y no observable.** Se retira del plan |

Los dos bugs que motivaron este documento (§7) **no son fallos de P2**. Son fallos de P1: un dato de
infraestructura (el reloj de pared) se coló en el estado de dominio. Existirían idénticos en un motor
deliberadamente aleatorio.

### Por qué P3 se retira

El [doc 2](2_Estudio_Evolucion_Backend_Multifrontend.md) pedía *"el motor será independiente del tamaño del
paso: 60 segundos de una vez equivaldrán a seis intervalos de 10 segundos"*. Medido sobre el motor real:

- **El consumo de RNG escala 1:1 con el número de pasos** (`crecimientoEstocastico`, `engine/population.ts`:
  una tirada por asentamiento por tick). Un paso grande consume menos aleatoriedad → estado distinto.
- **El crecimiento de población es compuesto**: `20 × 1.12⁶ = 39.5` frente a `20 × (1 + 6·0.12) = 34.4` —
  **15 % de divergencia en 6 pasos**.
- `avanzarPosicionEnRuta` (`engine/movimiento.ts`) integra por Euler muestreando el coste del terreno en la
  posición actual: también depende del tamaño de paso.

Lograr P3 exigiría rehacer la numérica del motor **y** el modelo de RNG, y aun así rompería P2. Ningún RTS ni
MMO lo hace: **todos fijan el paso**. Y nadie —jugador ni operador— puede *observar* la diferencia entre
"6 pasos de 10 s" y "1 de 60 s", así que no es un requisito, es un objetivo inventado. Se sustituye por:

> **El tamaño de paso es fijo y forma parte del contrato de la partida.** El tiempo real solo decide
> *cuántos* pasos se ejecutan, jamás *cuánto* mide cada uno.

---

## 2. El modelo: mundo = tiempo real

**Decisión del usuario (2026‑08‑29): 1 tick = 1 minuto de reloj real, y el tiempo de mundo avanza al mismo
ritmo que el real.** Sin capa de aceleración. Es el modelo de la estrategia lenta por navegador (OGame): un
día de juego es un día de verdad, el jugador entra un par de veces al día.

La consecuencia que lo hace barato:

```
instante_de_mundo = ÉPOCA + tick × DURACIÓN_TICK
```

`ÉPOCA = 2026-01-01T00:00:00.000Z` (elección del usuario — neutral, ISO limpio; la interfaz muestra "día N
desde la fundación" restando la época). `DURACIÓN_TICK = 60 000 ms`.

El `instante` es **derivado del `tick`, nunca almacenado ni leído del reloj de pared**:

- Una sola fuente de verdad (el `tick`, entero). Cero deriva acumulada por redondeo.
- Un snapshot antiguo (sin campo de instante) **migra sin datos nuevos**: se recalcula del `tick`.
- Reproducible: la misma partida en el mismo `tick` está siempre en el mismo `instante`.

### Una caída no consume tiempo de mundo

**Decisión del usuario, 2026‑09‑05.** Si el servidor se cae 3 horas, al volver la partida **se reanuda en el
tick en que se quedó**: lo que estuviera en construcción sigue igual de lejos de acabarse, y nadie encuentra
su partida saltada tres horas al volver. El mundo avanza solo mientras el reloj de mundo está en marcha, y
por eso `referenciaMs` se ancla a "ahora" al arrancarlo (`RunnerDePartida.iniciarRelojDeMundo`).

> **Esto invierte lo que hacía D5**, que ejecutaba los ticks vencidos en ráfaga. El motivo no fue técnico
> sino de diseño de juego, y llegó por un camino que conviene dejar escrito: la ráfaga era **una sola entrada
> de la cola serial**, así que durante ella ningún jugador podía ejecutar un comando — con el tope de una
> semana, ~18,6 minutos. Se planteó como un problema de infraestructura ("trocear la ráfaga", "ceder la cola
> cada N ticks") hasta que se vio que la pregunta de fondo era otra: **si una caída consume tiempo de mundo**.
> Contestada esa, la infraestructura sobra. Ver [doc 6](6_Sincronizacion_Visibilidad_y_Escala.md) §1.

Lo que **sí** se recupera es la deriva del temporizador: `setInterval` no dispara exacto y las décimas se
acumulan hasta valer un tick entero cada ~10 minutos; sin recuperarlas el mundo correría más lento que el
tiempo real y "1 tick = 1 minuto" dejaría de ser cierto. `MAX_TICKS_POR_PASADA` (5) separa esa deriva —que
produce 2 ticks como mucho— de una congelación del proceso —que produce cientos y se descarta, contándola en
la métrica `ticksOmitidos`—.

**El reloj de pared solo dice cuántos ticks faltan; nunca entra en el estado.** Eso no cambia, y es lo que
hace que lo que se ejecute sea determinista — un uso concreto y cercano de P2.

---

## 3. Los relojes y sus dueños

El [doc 2](2_Estudio_Evolucion_Backend_Multifrontend.md) pide literalmente distinguir "reloj de servidor,
tiempo de simulación, scheduler y unidad interna del motor". En código esa separación **no existe** hoy. La
Fase D la crea:

| Reloj | Qué es | Dónde vive | Puede leer `Date.now()` |
|---|---|---|---|
| **`tick`** | paso de integración, entero | `engine/` — nunca sale del motor | — |
| **`instante`** | tiempo de **mundo** (ISO 8601 / ms), `ÉPOCA + tick × DURACIÓN_TICK` | `session/` — fecha las reglas y viaja al cliente | **No**: es derivado |
| **reloj de pared** | `Date.now()` real | `server/` — nombres de snapshot, TTL de cachés, cálculo del catch‑up | **Sí, es su trabajo** (`RunnerDePartida.ahora`) |
| **intervalo del scheduler** | cada cuántos ms **reales** corre un tick | `server/` — `INTERVALO_TICK_MS`, ya existe | — |

Con "mundo = tiempo real", `DURACIÓN_TICK` y `INTERVALO_TICK_MS` **son el mismo número (60 000)** por defecto:
no hay aceleración que configurar. Se mantienen como constantes separadas porque sirven a cosas distintas
(una calcula el `instante`, la otra alimenta el `setInterval`) y así la puerta a un "servidor rápido" queda
abierta sin rediseñar nada.

---

## 4. La regla de autoridad única, y qué la vigila

> **El tiempo y la aleatoriedad ENTRAN como parámetro. El núcleo puro nunca los lee.**

- El motor recibe `ContextoSimulacion { instante, momento, rng, batallas? }` — nunca llama a `Date.now()` ni a
  `Math.random()`.
- Cada comando recibe `ContextoComando { instante, momento, actor, rng, ids, batallasEnUnity? }` — el `momento`
  lo pasa `session/`, derivado de `instante`, no del reloj de pared.
- `server/` es la única capa que lee el reloj de pared, y lo hace para cosas que **no son estado de partida**.

Esto lo congela un test permanente: **[`src/__tests__/autoridadTemporal.test.ts`](../../src/__tests__/autoridadTemporal.test.ts)**.

| Capa | Prohibido | Permitido |
|---|---|---|
| **Núcleo puro** (`domain`, `constants`, `worldgen`, `world`, `engine`) | nombrar `Date` en absoluto · `Math.random` · `performance.now` · `crypto.*` aleatorio | tiempo como `string`/`number`; comparación lexicográfica de ISO |
| **`session`** | leer el reloj (`Date.now()`, `new Date()` vacío, `performance.now`) · `Math.random` | `new Date(<valor>)` para construir/parsear (`instanteDeTick` computa el ms, `isoDeInstante` lo formatea) — nunca lee el reloj |
| **`server`, `acceso`** | *(fuera del guard a propósito)* | `server` posee el reloj de pared; `acceso` genera tokens de sesión, que deben ser impredecibles |

Por qué el guard es necesario y no bastaba lo que había: el [`arquitectura.test.ts`](../../src/__tests__/arquitectura.test.ts)
vigila la **dirección** de los imports entre capas, no que cada capa **contenga** lo que le toca. Por ese
hueco, `ctx.momento` se pobló con `new Date()` desde `server/` y viajó al estado persistido durante meses sin
que ningún test lo viera — el de determinismo del motor usa un `momento` derivado del tick, no el del servidor
real. Misma clase de fallo que el hito C8 (una regla codificada en la capa equivocada, invisible al contrato
que solo miraba imports).

El guard ya ha cambiado un diseño (2026-09-15): el id de una batalla tiene que ser único entre partidas, y en
`session` no puede salir de `crypto.randomUUID()`. Se deriva del `gameId` y del contador de ids de la partida
(`idDeBatalla`, `session/batallas.ts`).

---

## 5. Para qué sirve el determinismo aquí (P2), y qué se degrada

El análisis (revisión a fondo + cinco ángulos independientes, 2026‑08‑29) concluyó: **el determinismo no era
sobre‑ingeniería, pero se estaba pagando por tres cosas con un solo presupuesto.** Separadas:

**Se conserva P2 (motor reproducible con seed).** Tiene un consumidor real y ya amortizado:

- **Laboratorio de balance** (`scripts/run-batch-sim.ts`, 542 líneas): corre 100 facciones × 3000 ticks en
  segundos. Sus resultados ya están cocidos en `constants.ts` (*"Calibrado por simulación 150‑600 ticks"*,
  *"recalibrado, verificación batch"*). Con "mundo = tiempo real", 3000 ticks son ~50 horas de juego real
  comprimidas a segundos gracias al determinismo. Para un desarrollador solo, esto **sustituye al equipo de
  QA y de balance que no tiene**.
- **Catch‑up tras caída** (§2): ejecutar N ticks vencidos y llegar exactamente al mundo que habría existido
  si el servidor no se hubiera caído.

**Se degrada la bit‑exactitud a nivel de sesión.** `PartidaExportada.estadoRng` (continuar la secuencia
exacta del RNG entre reinicios, Fase B3) **no tiene consumidor**: no hay replay de incidentes, no hay
event‑sourcing, y reconstruir un bug desde un snapshot exige además *la secuencia de comandos que lo siguió*,
que no se registra. Se mantiene mientras no cueste, pero **no se invierte más en ella** hasta que un incidente
real la pida.

**Lo que sustituye a lo que se degrada**: un **log de comandos** (`tick`, `actor`, `tipo`, `params`) — más
barato que `estadoRng`, es lo que de verdad reconstruye un incidente, y de paso es el log de combate legible
que el jugador va a pedir ("¿por qué perdí mi ejército?"). Queda para Fase E (auditoría), no bloquea D.

**El test de determinismo del motor** (`engine/__tests__/determinismo.test.ts`) se mantiene — pero hay que
tener claro que valida P2 (el motor puro), no la costura `session`↔`server` donde estaba el bug. Esa costura
la cubre ahora el guard de §4.

---

## 6. Los tres arquetipos temporales del motor

128 usos de `*Tick` repartidos por el código. No son homogéneos:

| Arquetipo | Ejemplos | Migración (D2–D6) |
|---|---|---|
| **Instantes** | `expiraEnTick`, `fundadoEnTick`, `heridoHastaTick`, `activadaEnTick`, `creadoEnTick`, `bandidosProximoSpawnTick`, `ultimaCaravanaCreadaEnTick`, `estadoMapa.regeneraEnTick` | **Hecho en D2**: `number` → `Instante` (`domain/tiempo.ts`, branded sobre `number` ms). `tick >= x.expiraEnTick` pasó a `ctx.instante >= x.expiraEn` — misma forma; `tsc` señaló los ~55 sitios |
| **Contadores** | `ticksRestantes` (construcción), `rachaMantenimientoSano`, `extractoresTicksSinCupo` | **`ticksRestantes` hecho en D3**: `Edificio.ticksRestantes: number` → `Edificio.completaEn?: Instante` (presente solo `en_construccion`); el motor pasó de decrementar a comparar `instante >= completaEn`. Implementa gratis la regla (a) del [doc 6 §4](6_Sincronizacion_Visibilidad_y_Escala.md#4-reglas-de-protocolo-agnóstico-al-tick): "fechas absolutas, nunca contadores". `rachaMantenimientoSano` y `extractoresTicksSinCupo` son rachas/acumuladores reales (cuentan pasos consecutivos), no deadlines — se quedan como contadores; en D6 su constante umbral pasó a minutos (`minutosSanosParaRecuperarNivel`) pero el campo persistido sigue contando pasos |
| **Tasas** (~15 constantes) | `racionPorSoldadoPorMinuto`, `regeneracionMoralPorMinuto`, `produccionBase*` (`/minuto`), `tributo.cantidadPorMinuto` | **Hecho en D6**: ninguna cambia de valor (el paso es fijo, 1 tick = 1 min); solo cambió la etiqueta de unidad `*PorTick`/`/tick` → `*PorMinuto`/`/minuto`. `tributo.cantidadPorTick` (persistido) migró con el snapshot v3 → v4 |

Con **paso fijo**, el D3 original del [doc 2](2_Estudio_Evolucion_Backend_Multifrontend.md) ("todo a tasas y
acumuladores por tiempo transcurrido") **no hace falta**: solo era necesario con paso variable.

---

## 7. Los dos bugs que esto repara

Ambos verificados empíricamente, ambos con meses en producción, ninguno detectado por la suite.

**a) `ctx.momento` es reloj de pared y se persiste en el estado.** `RunnerDePartida.ejecutar` pasa
`this.ahora()` como `momento`; ese valor termina en `eventosDominio[].momento` y en
`salidasFaccionPorJugador`. El mismo comando, con la misma seed y el mismo estado inicial, produce **snapshots
distintos** según la hora a la que se ejecutó. Viola el principio 6 del doc 2 ("un reinicio no puede alterar
la secuencia") para el log de eventos.

**b) El cooldown anti‑abuso de `crearFaccion` (7 días) corre contra el reloj de pared.** Verificado: con la
partida **congelada en el tick 0**, el cooldown expira a los 7 días reales igualmente. Está desacoplado del
tiempo de mundo.

**D1 los cerró de raíz (2026‑08‑29):** `momento` pasa a derivarse del tick (autoridad temporal única). Como
mundo = tiempo real, "7 días de mundo" = "7 días reales", así que el cooldown de `crearFaccion` se mide contra
`instante` y su ventana sigue siendo 7 días de verdad — sin excepción que documentar ni explotabilidad.

**Regresión congelada**: `runnerDePartida.test.ts` — el mismo comando con relojes de pared distintos produce
un snapshot **idéntico** (antes de D1 fallaba), y los eventos se fechan en tiempo de mundo aunque el reloj de
pared diga 2099.

---

## 8. Fase D re‑planteada

| Hito | Qué | Nota |
|---|---|---|
| ~~**D1**~~ | **Hecho 2026‑08‑29.** `SIMULACION` en `constants.ts` (`epocaInicial` + `duracionTickMs`, `BALANCE_VERSION` → 2, servido en `/v1/balance`); `instanteDeTick(tick)` en `session/estado.ts` (puro, derivado, nunca almacenado); `GameSession.ejecutar` lo deriva del tick en vez de recibir `momento` (parámetro eliminado); `avanzarTick` fecha sus eventos con el instante RESULTANTE; `RunnerDePartida` deja de pasar `this.ahora()` como `momento` (lo reserva para `guardadoEn`/TTL/catch‑up). **Sin cambio de formato de snapshot** — el instante es derivado. 675 → 677 tests, verificado en vivo | Establece la autoridad temporal única. Cierra los dos bugs de §7 |
| ~~**D2**~~ | **Hecho.** `domain/tiempo.ts` (`Instante`/`Duracion` branded + `sumar`/`transcurrido`/`minutos`/`dias`); `constants.ts` gana `ticksComoDuracion(n)` (puente hasta D6); `instanteDeTick(tick): Instante` + `isoDeInstante` en `session/estado.ts`. Los ~10 campos temporales (`domain/types.ts` + `EstadoMapa` + `GameSessionState`) renombrados `*EnTick: number` → `*En: Instante`. `ContextoSimulacion`/`ContextoComando` ganan `instante`. Migración de snapshot **v1 → v2** (`persistenciaPartida.ts`, relación 1:1). 677 → 678 tests, verificado en vivo | `tsc` fue la lista de tareas: la marca de tipo señala todo sitio que aún trate un instante como tick (~55). El guard de §4 **no** se endureció — el único `new Date` que queda en `session` es `instanteDeTick`/`isoDeInstante` construyendo desde un valor, nunca leyendo el reloj |
| ~~**D3**~~ | **Hecho.** `Edificio.ticksRestantes: number` → `Edificio.completaEn?: Instante` (solo `en_construccion`; `en_cola`/`activo` sin campo). `avanzarConstruccion` recibe `instante` y compara `instante >= completaEn` en vez de decrementar; `completaEn` se fija al arrancar la obra (`sumar(instante, minutos(ticks))` tras D6 — D3 usaba el puente `ticksComoDuracion`; la Vía Rápida se aplica ahí). Migración de snapshot **v2 → v3** (`persistenciaPartida.ts`, `migrarSnapshot` encadena v1→v2→v3). Timing tick‑a‑tick idéntico (el snapshot baseline no se movió). 678 → 679 tests, verificado con laboratorio de batch en vivo. De paso: `scripts/run-batch-sim.ts` y varios ids dejaron de meter el tick en el string | Cierra la regla "fechas, no contadores" del doc 6 §4. Único deadline-contador que quedaba |
| ~~**D4**~~ | **Hecho.** `ResumenPartida`, `ResumenPartidaEnDisco`, `EstadoAdmin` y `ProyeccionJugador` ganan `instante: Instante` (ms de mundo) — derivado como `mapaId` (`instanteDeTick(tick)`), nunca almacenado. Es la referencia temporal del contrato: el cliente pinta cuentas atrás localmente (`completaEn - proyeccion.instante`) sin traducir ticks. `tick` sigue viajando, marcado provisional en cada DTO. Esquemas de respuesta (`RESUMEN_PARTIDA_RESPUESTA`, `ESQUEMA_LISTAR_PARTIDAS`) actualizados — Fastify filtra por schema, un campo ausente del schema se descarta. Los campos‑instante de las entidades (`completaEn`, `expiraEn`, `heridoHasta`…) **ya viajaban** como `Instante` absoluto desde D2/D3; los eventos ya llevan `momento` ISO. 679 tests, verificado en vivo sobre las 4 superficies HTTP | Split deliberado: estado vivo en `Instante` ms (aritmética directa contra `completaEn`); eventos en ISO `momento` (legible en un log). No hay campo ISO redundante en los DTOs — `isoDeInstante` es una línea en el cliente |
| ~~**D5**~~ (parcial) | **Reloj de mundo + catch‑up hechos** (= E1). `RunnerDePartida.iniciarRelojDeMundo(intervaloMs)` reemplaza el metrónomo `iniciarTicksAutomaticos`: `estado.tick` sigue al reloj de pared, con referencia anclada (`guardadoEn` del snapshot en `tickAlConstruir`) y ráfaga de ticks vencidos tras un reinicio por la cola serial — sin acumular jitter del `setInterval`. `cargarPartida` → `{ sesion, guardadoEn }`. `RegistroDePartidas` recibe el reloj inyectado y expone `cerrar()` (apagado limpio vía `onClose` de Fastify). 683 tests, verificado en vivo por HTTP. **Comandos programados: aplazados** — asedios formales y caravanas planificadas son Fase 1+ por decisión explícita (docs Game 5/3); construir el almacén + despacho sin consumidor sería infra especulativa. Aterriza con su primera mecánica | El catch‑up mide **tiempo real transcurrido** desde el último tick guardado (§2: "faltan 180 ticks" = 3 h / 1 min), no un salto a la fecha del calendario. El `instante` de cada tick de la ráfaga lo deriva el motor del tick ⇒ determinista, un uso concreto de P2 |
| ~~**D6**~~ | **Hecho.** Constantes de `constants.ts` renombradas de tick a minuto, **sin tocar valores** (1 tick = 1 min): plazos (`tiempoConstruccionMinutos`, `plazoMinutosPorDefecto`, `duracionMinutosPorDefecto`, `duracionHeridoMinutos`, `graciaMinutos`, `cooldownMinutos`, `respawnMinutos`), horizontes (`horizonteMinutosMantenimiento/Comida`, `minutosSanosParaRecuperarNivel`), tasas (`racionPorSoldadoPorMinuto`, `regeneracionMoralPorMinuto`, `decaimientoPorMinuto`, `bonusPorMinutoAlianzaActiva`, `fraccionMuertePorMinutoHambre`, `regeneracionPorMinuto`, `bonusPorMinutoStarved`, `desercionFraccionPorMinutoSinMoral`). **`ticksComoDuracion` eliminado** — el motor usa `minutos()` de `domain/tiempo.ts` directamente; `duracionTickMs` queda como el único sitio que "sabe" cuánto dura un tick. `RelacionPolitica.tributo.cantidadPorTick` → `cantidadPorMinuto` (migración de snapshot **v3 → v4**). `BALANCE_VERSION` 2 → 3. 684 tests | Ningún plazo del juego se declara ya en ticks |
| ~~**Cierre**~~ | **Hecho.** Se retira el `tick` provisional del CONTRATO: `EventoDominio` se queda solo con `momento` (ISO); `EventoLogAdmin.tick` → `momento`; `ContextoSimulacion.tick` desaparece (nada lo leía); `ResumenPartida`/`ResumenPartidaEnDisco`/`ProyeccionJugador` pierden `tick` (ya llevan `instante`). Los helpers `evento`/`eventos`/`desdeCrudos` dejan de recibir `estado`. Migración de snapshot **v4 → v5** (strip `tick` de `eventosDominio`; `historialJugadores` pasa `tick` → `momento`). 685 tests | El `tick` sobrevive SOLO como paso de integración interno del motor (`GameSessionState.tick`, del que se deriva el `instante`) y como diagnóstico en `EstadoAdmin` (heredado del estado). Nada del contrato externo lo menciona |

**Se cae del plan original:**
- **P3** (independencia del tamaño de paso) — medido inalcanzable, no observable. Sustituido por "paso fijo,
  parte del contrato de la partida" (§1).
- **El D3 original** (todo a tasas/acumuladores) — solo hacía falta con paso variable.
- **La justificación de determinismo en los docs** ("reproducir un bug de producción desde snapshot") —
  reencuadrada: el determinismo se conserva por el laboratorio de balance y el catch‑up (§5), no por un
  replay de incidentes que nunca se construyó.

**Sube de prioridad:** el scheduler (estaba en Fase E) — es lo que el backlog de mecánicas necesita.

**Aparte, después de D (no es un hito de D):** la **pasada de rebalanceo en tiempo**. D1–D6 preservan los
valores mecánicamente y el juego sigue corriendo; pero con 1 tick = 1 minuto real, números que hoy son
abstractos pasan a tener consecuencia de ritmo (población a ~12 %/tick compuesto = duplica cada ~6 minutos
reales — absurdo para un juego lento). Ese ajuste es un esfuerzo dedicado, apoyado en el laboratorio de
balance, y se separa a conciencia de la migración estructural para no acoplar dos trabajos grandes.

---

## 9. Cómo usar esta referencia

Al añadir código que toque el tiempo, dos preguntas en este orden:

1. **¿Estoy en el núcleo puro (`domain`/`constants`/`worldgen`/`world`/`engine`) o en `session`?** →
   el tiempo ENTRA como parámetro. Si necesitas `Date.now()`, estás en la capa equivocada: súbelo a `server/`.
2. **¿El valor que estoy guardando en el estado depende del reloj de pared?** → no lo guardes. Deriva del
   `tick` (o del `instante`, que es lo mismo). El reloj de pared solo vive en `server/`, y solo para cosas
   que no son estado de partida.

Y la advertencia que ya costó meses: **un invariante que ningún test ejercita por el mismo camino que
producción no es un invariante.** El guard de §4 corre sobre el árbol de `src/` real; el test de determinismo
del motor, no — sabe lo que valida (P2) y lo que no (la costura donde estaba el bug).
