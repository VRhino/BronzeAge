# Desglose de tareas: evolución a backend multijugador

Este es el archivo de trabajo día a día. Cada tarea se marca `[x]` al completarse;
si se aborda parcialmente, anotar entre paréntesis el estado y seguir marcada
`[ ]`. Cuando se cierran todas las tareas de un hito, marcar ese hito en
[3_Plan_Evolucion_Roadmap.md](3_Plan_Evolucion_Roadmap.md).

Las tareas de fases B–E son más gruesas porque dependen de decisiones que se
toman al ejecutar la Fase A (p. ej. qué forma toma `GameSession`, qué motor de
persistencia se elige). Se subdividen más a medida que cada fase se acerca.

---

## Fase A — Núcleo reutilizable

### A1. Suite de pruebas estable — ✅ completada 2026-08-24
- [x] Revisar y corregir los tests fallando en `src/engine/__tests__/anclasSatelites.test.ts` (ya pasaban al re-ejecutar: 30/30 en ambos ficheros)
- [x] Revisar y corregir los tests fallando en `src/engine/__tests__/lineas_produccion.test.ts`
- [x] Confirmar `tsc --noEmit` limpio
- [x] Confirmar suite completa en verde y anotar commit de referencia aquí: `5b47cda` (working tree, 40 archivos / 270 tests en verde; hay cambios staged pendientes de commit, ver `git status`)

### A2. Motor sin dependencias de UI/app — ✅ completada 2026-08-24, con test permanente añadido más tarde
- [x] Grep de imports en `src/engine/**` hacia `src/app/`, `src/ui/`, `src/main.ts` (dio vacío)
- [x] Si aparece algún acoplamiento, moverlo a `app/` o `world/` según corresponda (no aplica, no se encontró acoplamiento)
- [x] Dejar constancia de la verificación (fecha + resultado) en este archivo: todos los imports de `engine/**` (incl. `__tests__`) resuelven solo a `../domain`, `../constants`, `../world`, `../worldgen` y módulos internos de `engine`; sin imports dinámicos (`import(`)
- [x] **Convertida en test permanente**: [src/__tests__/arquitectura.test.ts](../../src/__tests__/arquitectura.test.ts) — congela la dirección de dependencias entre TODAS las capas (`domain`/`constants`/`worldgen`/`world`/`engine`/`ui`/`app`/`lab`/`main`), no solo motor↔app. Usa `import.meta.glob` de Vite (no `node:fs`) para no introducir `@types/node` como dependencia nueva — requirió añadir `src/vite-env.d.ts` (`/// <reference types="vite/client" />`, ausente hasta ahora en el proyecto). Verificado que detecta violaciones reales antes de confiar en él (falla correctamente si se inyecta un import `engine -> app`)

### A3. RNG inyectable — ✅ completada 2026-08-24
- [x] Listar todos los `Math.random()` dentro de `src/engine/` — 6 call sites: `population.ts:18`, `combate.ts:58,59,229,291`, `bandidos.ts:103`
- [x] Diseñar una interfaz `Rng` — se reutilizó `RandomFn = () => number` ya existente en `worldgen/rng.ts` (mulberry32), sin añadir estado exportable propio: **decisión deliberada**, ver nota abajo
- [x] Sustituir usos en el subsistema de población (`crecerPoblacion`/`crecimientoEstocastico`, `population.ts`)
- [x] Sustituir usos en el subsistema de combate (`resolverCombate`, `iniciarAsedio`, `combateCampoAbierto`, `interceptarCaravana`, `atacarCampamentoBandidos`, `combate.ts`) — estas son comandos de jugador, no solo tick automático, así que también quedaron con `rng` inyectado
- [x] Sustituir usos en el subsistema de bandidos (`avanzarAtaquesBandidos`, `bandidos.ts`)
- [x] `avanzarSimulacion` y `avanzarNpcGobernanza` ahora exigen `rng: RandomFn` como parámetro — no hay valor por defecto ni fallback a `Math.random`, a propósito: un parámetro opcional habría reintroducido el mismo problema en cualquier call site que lo olvidara
- [x] `GameStore` gana un campo `private rng: RandomFn`, reseteado con la seed del mundo en `regenerarMundo`/`importarSimulacion` (mismo patrón que ya usa para el mapa)
- [x] Actualizados todos los call sites: `gameStore.ts`, `npcGobernanza.ts`, `lab/main.ts`, `scripts/run-batch-sim.ts` y 14 ficheros de test — varios tests dejaron de usar el monkeypatch global `mockMathRandomDeterminista` y pasan un `createRng(seed)` explícito en su lugar (más simple y ya no dependen de mutar `Math.random` global)
- [x] Verificación: `tsc --noEmit` limpio en `src/`, suite completa 40/40 archivos y 270/270 tests en verde, `scripts/run-batch-sim.ts` sin errores de tipo nuevos (fuera de `tsconfig include`, comprobado aparte)
- [x] **Estado del RNG persistido en `GameSession.exportar`/`importar`** (2026-08-25). Motivo para resolverlo ya y no esperar al guardado en disco: es justo lo que hace falta para reconstruir un bug de producción — cargar el snapshot de un incidente y avanzar tiene que reproducir la MISMA continuación que ocurrió en el servidor, no una cualquiera. Antes, todo `importar` reiniciaba el RNG desde la seed del mundo (posición 0 de la secuencia mulberry32), así que dos reinicios de la partida real —dos despliegues, dos caídas— consumían exactamente la misma racha de números; cualquier cosa influida por `ctx.rng` (varianza de población, combate, aparición de bandidos) quedaba correlacionada entre reinicios en vez de continuar
  - `RandomFn` gana `estado()` (el contador de 32 bits de mulberry32, antes atrapado en la clausura) y `worldgen/rng.ts` gana `restaurarRng(estado)`, que retoma la secuencia exacta — a diferencia de `createRng(seed)`, que siempre vuelve al inicio (`worldgen/rng.ts`)
  - `PartidaExportada.estadoRng` (opcional) lo lleva; `GameSession.importar` lo usa si está y cae de vuelta a la seed si no — el formato de archivo v2 de `GameStore` (anterior a esto) no lo guarda, así que sigue con la limitación vieja ahí, documentada en vez de oculta
  - Verificado con el efecto observable, no solo con el mecanismo: una partida que sigue en memoria y otra reconstruida en el mismo punto y avanzada por los mismos ticks llegan al estado EXACTAMENTE igual (`session/__tests__/gameSession.test.ts`)
  - De paso, un test de `app/__tests__/exportarImportar.test.ts` que agotaba un yacimiento llamando a `mapa.extraer()` sobre la fachada de `getMapa()` dejó de probar lo que decía: desde que `Mapa` copia el estado en vez de aliasarlo (commit anterior, B3 mutaciones), esa fachada ya no es un asa de escritura sobre la partida — el test pasaba por una coincidencia de caché, no por lo que afirmaba. Reescrito para agotar el nodo en el archivo (`recursos[].cantidad`), que es como le llegaría un save real con un yacimiento vacío
  - Tests: 352 → 357

### A4. IDs desacoplados de `GameStore` — ✅ completada 2026-08-24
- [x] Extraer el contador de IDs de `src/app/gameStore.ts` a un módulo propio: `src/app/idGenerator.ts` (`GeneradorIds`) — vive en `app/`, no en `engine/`: lo consumen `GameStore` y `npcGobernanza.ts` (capa de aplicación), el motor solo recibe el número ya generado como parámetro
- [x] Verificar que el generador es serializable — `actual()`/`fijar()` exponen el contador como `number` plano, sin estado adicional que envolver
- [x] Actualizar `GameStore` para usar el generador extraído sin cambiar comportamiento observable — campo `private ids = new GeneradorIds()` reemplaza a `private contadorAcciones = 0`; los 7 usos de `this.contadorAcciones++` pasan a `this.ids.siguiente()` (misma semántica postfix), y los 2 usos de lectura/escritura directa (NPC de gobernanza) a `this.ids.actual()`/`this.ids.fijar(...)` — **no se resetea** en `regenerarMundo`/`importarSimulacion`, igual que el comportamiento previo (no había reset ahí tampoco)
- [x] Verificación: `tsc --noEmit` limpio, 40/40 archivos y 270/270 tests en verde

### A5. Eventos de dominio estructurados — ⚠️ SOLO LA BASE, NO COMPLETADA — migración progresiva pendiente

> **Estado real: base lista (2026-08-24), contenido NO migrado.** Todo evento que emite hoy el motor sigue
> siendo texto libre envuelto bajo `codigo: 'legado'` — ver marcador de progreso abajo. No confundir "A5 tiene
> checkmarks" con "A5 está terminada": el hito solo se da por cerrado cuando el marcador llegue a 100%.
> Decisión con el usuario: hacer solo la base en esta sesión (definir el tipo + que el motor lo devuelva) y
> dejar la migración subsistema-por-subsistema para tareas futuras, dado el tamaño (13 subsistemas de
> `engine/` producen hoy `string[]` de texto libre dentro del tick).
>
> **⚠️ Gate añadido 2026-08-25: los 13 subsistemas se migran ANTES de entrar en Fase C.** No bloquean el resto
> de la Fase B (persistencia, API HTTP, `RunnerDePartida` — siguen desacoplados, ver nota en doc 7 §7.1) pero
> sí bloquean el arranque de Fase C. Motivo: Fase C construye autorización y proyecciones por audiencia sobre
> los comandos, y la auditoría/replay de Fase E que `eventosDominio` existe para servir necesita códigos
> estables para poder filtrarse por tipo — construir C encima de 13 eventos todavía sin migrar dejaría ese
> trabajo por rehacer más caro más adelante, con más código ya dependiendo de la forma final.

**Base (completada):**
- [x] Definir tipo `EventoDominio` — módulo nuevo `src/domain/eventos.ts` (no `domain/types.ts`, que es solo contratos de entidades de juego): `{ codigo, mensaje, tick, asentamientoId? }`. Todo evento hoy usa `codigo: 'legado'` — no hay catálogo de códigos todavía, eso es justo lo que falta migrar
- [x] Hacer que `engine/simulation.ts` (`avanzarSimulacion`) devuelva `EventoDominio[]` junto al estado — nuevo campo `eventosDominio` en `ResultadoTick`, poblado en paralelo a `eventos` (helper `comoEventosDominio`) en cada punto donde el tick ya empujaba texto a `eventos`; incluye `asentamientoId` en los eventos del bucle por asentamiento, ausente en los globales (comercio, mercado, tributos, nivel de facción, títulos, regeneración de yacimientos, ataques de bandidos)
- [x] Test de contrato nuevo: `src/engine/__tests__/eventos_dominio.test.ts` — mismo contenido 1:1 que `eventos`, tick correcto, `asentamientoId` presente/ausente donde corresponde
- [x] Verificación de la base: `tsc --noEmit` limpio, 41/41 archivos y 271/271 tests en verde

**Preparación para tiempo real (completada 2026-08-24):**
- [x] Añadir `momento` (ISO 8601) a `EventoDominio` — es el campo temporal DEFINITIVO que viaja a los clientes; `tick` queda marcado como provisional en el propio tipo. Implementa la regla (b) de [6_Sincronizacion_Visibilidad_y_Escala.md](6_Sincronizacion_Visibilidad_y_Escala.md) §4
- [x] Introducir `ContextoSimulacion { tick, momento, rng }` — cumple el punto 3 de la Fase A del doc 2 ("Introducir contexto de partida para RNG e IDs"). `avanzarSimulacion(estado, mapa, contexto)` y `avanzarNpcGobernanza(estado, mapa, contexto, config)` comparten ahora la misma forma de llamada. **El motor no lee nunca el reloj ni la aleatoriedad global**: ambos se inyectan, que es lo que mantiene la simulación reproducible. Al pasar a tiempo real, `tick` desaparece de este contexto sin cambiar ninguna firma
- [x] `GameStore.contextoDeTickActual()` — la capa de aplicación es la dueña del reloj (`new Date()`); tests y batch derivan `momento` del tick para no romper determinismo (`contextoDeTest` en `engine/__tests__/fixtures.ts`)

**Pendiente — no hecho a propósito:**
- [ ] Adaptar `GameStore` para traducir `EventoDominio[]` a las entradas de log en texto actuales — `GameStore.avanzarTick` sigue leyendo `resultado.eventos` (sin cambios), `eventosDominio` no se consume aún en ningún sitio; no hace falta tocar `GameStore` hasta que un consumidor real (ej. el futuro backend) lo necesite

#### 📊 Marcador de progreso — migración por subsistema: **0 / 13 migrados (0%)**

Migrar un subsistema = darle su propio `codigo` estable (dejar de usar `'legado'`) y mover lo que hoy es
texto libre a `payload` tipado en `EventoDominio`, dentro de la función `avanzarX` de ese módulo. Marcar aquí
cada uno al migrarlo y actualizar la fracción del encabezado — **A5 no se considera cerrada en el roadmap
hasta que este contador llegue a 13/13 (100%)**.

- [ ] `engine/construction.ts` (`avanzarConstruccion`)
- [ ] `engine/politicas.ts` (`avanzarPoliticas`)
- [ ] `engine/tropas.ts` (`avanzarMantenimientoTropas`)
- [ ] `engine/mantenimiento.ts` (`avanzarNivelAsentamiento`, `avanzarMantenimiento`)
- [ ] `engine/population.ts` (`avanzarNutricionPoblacion`, `crecerPoblacion`)
- [ ] `engine/trade.ts` (`avanzarComercio`)
- [ ] `engine/expansion.ts` (`avanzarCaravanasFundacion`)
- [ ] `world/mapa.ts` (`avanzarRegeneracion`)
- [ ] `engine/bandidos.ts` (`avanzarSpawnBandidos`, `avanzarAtaquesBandidos`)
- [ ] `engine/market.ts` (`avanzarMercado`)
- [ ] `engine/diplomacia.ts` (`avanzarTributos`)
- [ ] `engine/faccion.ts` (`avanzarNivelesFaccion`)
- [ ] `engine/titulos.ts` (`narrarCambiosDeTitulo`)

**Fuera de alcance de este marcador (a propósito):** los resultados de comandos de combate
(`engine/combate.ts`: `iniciarAsedio`, `combateCampoAbierto`, `interceptarCaravana`, `atacarCampamentoBandidos`)
y los mensajes que `GameStore` arma a mano para sus propios comandos (fundar, crear facción, comprar casa...)
no pasan por `avanzarSimulacion`/`ResultadoTick` — son eventos de COMANDO, no de tick. Estructurarlos es
trabajo relacionado pero distinto, más cercano al punto 6 de
[2_Estudio_Evolucion_Backend_Multifrontend.md](2_Estudio_Evolucion_Backend_Multifrontend.md) ("comandos deberían
devolver un resultado estructurado") que a este marcador — no sumar ni restar de la fracción 0/13 por ellos.

### A6. Diseño de contratos de identidad y permisos (solo documento, sin código) — ✅ completada 2026-08-24
- [x] Redactar definición de `Usuario`, `Sesion`, `Jugador`, `Rol`, `Partida`, `Membresia`
- [x] Redactar matriz de autorización por tipo de comando — construida sobre la superficie real de `GameStore` (~35 comandos mutables), no una lista abstracta
- [x] Diferenciar explícitamente roles técnicos (admin de partida, moderador) de cargos de juego (rey, gobernador, tesorero) — dos ejes ortogonales, ver documento
- [x] Guardar como [5_Contratos_Identidad_Permisos.md](5_Contratos_Identidad_Permisos.md) — incluye preguntas abiertas (ej. `crearFaccion`/`renombrarAsentamiento` sin condición de dominio definida) para no perderlas antes de Fase C

---

## Fase B — Backend provisional con ticks

> Objetivo de escala acordado 2026-08-24: **mínimo 500 jugadores conectados en una misma partida**. Ver
> [6_Sincronizacion_Visibilidad_y_Escala.md](6_Sincronizacion_Visibilidad_y_Escala.md) para las mediciones y
> las decisiones de arquitectura de conexiones/visibilidad que salen de ese número.

- [x] Definir la forma de `GameSession`: qué recibe, qué expone, qué NO debe contener — **hecho 2026-08-24**, ver [7_Diseno_GameSession.md](7_Diseno_GameSession.md). Resumen de lo decidido: `GameSession` es **síncrona, sin E/S, sin cola y sin reloj propio**; vive en una **capa nueva `src/session/`** (no en `app/`, para que la frontera sea verificable por el test de arquitectura); los comandos devuelven `ResultadoComando { ok, codigoError, detalle, eventos, version }` en vez de escribir texto en un log; la **cola serial queda fuera**, en un `RunnerDePartida` posterior, que es también donde se resolverá el tick bloqueante de ~1.9 s
  - Hallazgo del inventario: `GameStore` **no usa ninguna API de navegador** (`Blob`/`FileReader`/`document` viven en `main.ts`), así que la extracción es más limpia de lo que anticipaba el doc 2
  - Hallazgo con impacto en escala: `notify()` hace `structuredClone` del estado COMPLETO y se invoca desde **38 sitios** (cada comando, no solo el tick) — a ~2.4 MB de estado eso no sobrevive al servidor. `GameSession` no lleva historial ni clona
- [x] Migrar de `GameStore` a `GameSession` las responsabilidades: cargar/crear/guardar partida, ejecutar tick y NPC, traducir errores de dominio a resultados estructurados — **paso 1 del plan de 4 hecho 2026-08-24**: `src/session/gameSession.ts` existe con estado, `crear`/`exportar`/`importar`, `fundarAsentamiento`, `crearFaccion`, `avanzarTick`, `avanzarFaccionesNpc` — todos devolviendo `ResultadoComando`, ninguno leyendo el reloj. **Todavía nadie la usa** (`GameStore` no ha cambiado su implementación interna). 14 tests propios en `src/session/__tests__/gameSession.test.ts`, incluida una prueba de determinismo igual en espíritu a la del motor
  - **Efecto colateral necesario**: `npcGobernanza.ts` e `idGenerator.ts` se movieron de `app/` a `session/` (con `git mv`, historia preservada) — `GameSession.avanzarFaccionesNpc` los necesitaba y `session/` no puede importar de `app/`. Cero cambios de import dentro de esos dos archivos (misma profundidad relativa a `domain/`/`engine/`); sí se actualizaron sus 5 consumidores (`gameStore.ts`, `run-batch-sim.ts`, comentarios en `tropas.ts`/`main.ts`/`faccionNpc.test.ts`) y se movió `nucleoMilitarNpc.test.ts` junto con el archivo que prueba
  - ~~**Pendiente**: el comando administrativo para fijar `faccionesNpcIds`~~ — **hecho**: `comandos/alternarFaccionNpc.ts`, idempotente (pedir el estado en el que ya está no muta ni versiona, lo que lo hace seguro ante reintentos por reconexión)
  - ⚠️ **Orden del plan corregido 2026-08-24** (doc 7 §6): los pasos "convertir `GameStore` en adaptador" y "mover comandos" estaban al revés. `GameStore` tiene **70 mutaciones directas** de `this.state`, así que mientras quede un comando sin migrar habría dos fuentes de verdad divergiendo. El adaptador solo puede montarse cuando ya no queda ninguna
  - ✅ **Migración de comandos COMPLETA: 31 / 31 comandos reales** — fundación/facción (2), cargos y ciudadanía (6), diplomacia (5), comercio (3), militar (5), construcción (7) y expansión (`lanzarCaravanaFundacion`, `desarmarCaravanaFundacion`). Más las 2 operaciones de sistema `avanzarTick`/`avanzarFaccionesNpc`
  - `alternarAutoConstruccion` **unifica** los dos métodos separados de `GameStore` (`pausarAutoConstruccion`/`reanudarAutoConstruccion`) en un comando con bandera, igual que `alternarFaccionNpc` — misma intención con dos valores, y dos handlers idénticos salvo un booleano sería duplicación. También idempotente
  - **De 34 a 31**: tres de los "comandos" de `GameStore` resultaron no serlo (aclaración del usuario sobre `regenerarMundo`, ver [7_Diseno_GameSession.md](7_Diseno_GameSession.md) §2.ter):
    - `regenerarMundo` e `importarSimulacion` son **creación/reconstrucción de partida**, no operaciones sobre una partida viva — ya resueltas por `GameSession.crear()`/`importar()`. En el backend, "regenerar el mundo" de una partida en curso es **descartarla y crear otra**: operación destructiva de administración, no un botón más
    - `actualizarBalance`/`restaurarBalance` mutan `constants.ts` **en el sitio** vía `app/balanceConfig.ts`: son estado del PROCESO, no de la partida. Migrarlos como comandos fingiría que son transiciones de estado y consolidaría el error que el doc 2 marca como riesgo. Quedan para Fase C, con el resto del trabajo de balance versionado por partida
  - **Dos bugs latentes corregidos al migrar**, ambos heredados de `GameStore`:
    1. Los comandos de cargos hacían `.find(...)!` sobre Facción/asentamiento; si la entidad no existía, el `!` dejaba pasar `undefined` y el motor reventaba con un `TypeError` en vez de rechazar. Los handlers comprueban antes y devuelven `faccion.no_existe` / `asentamiento.no_existe`
    2. `romperRelacion` **no tenía try/catch**, pero el motor lanza `DiplomaciaInvalidaError` si la relación no existe (`engine/diplomacia.ts`) — la excepción llegaba cruda a la interfaz. Ahora pasa por `rechazoDesdeError` como el resto
    - Ambos cubiertos por tests que en la versión anterior habrían fallado con excepción
    - El patrón `.find(...)!` resultó ser **sistemático**, no puntual: reaparece en `crearCaravana`, en los 5 comandos militares (asentamiento, caravana y campamento) y en los 7 de construcción. Todos corregidos, con códigos propios `caravana.no_existe` / `campamento.no_existe` además de `asentamiento.no_existe`
    - **Caso peor en construcción**: `pausarAutoConstruccion`, `reanudarAutoConstruccion`, `calibrarReservaManual` y `renombrarAsentamiento` ni siquiera tenían try/catch **y** usaban `.find(...)!`, así que con un id inexistente el `TypeError` saltaba al leer `asentamiento.id` para componer el mensaje de log — sin ninguna red que lo atrapara
  - **Rehecho 2026-08-24 antes de migrar el resto** (ver doc 7 §2.bis): los comandos pasan de métodos de `GameSession` a **funciones puras en `session/comandos/`, un archivo por comando**. La forma anterior habría llevado a ~1100+ líneas con los 34 comandos, reproduciendo el objeto-dios de `GameStore`. Ahora `gameSession.ts` son 139 líneas y **no crece** al añadir comandos. Incluye `ContextoComando { momento, actor, rng, ids }` (el `actor` ya está listo para la autorización de Fase C) y transiciones puras `(estado, mapa, ctx, params) -> { estado, resultado }`, que son las que permitirán al runner de B3 descartar el estado si falla la persistencia
- [x] **Triar las 31 consultas de `GameStore`** — hueco detectado al revisar el diseño: no estaban contempladas en ninguna capa. Clasificadas en [8_Triaje_Consultas.md](8_Triaje_Consultas.md): 11 → `engine/` (cálculos de dominio puros), 8 → `session/`, 5 → proyecciones de Fase C (filtradas por visibilidad; hoy filtrarían información de rivales), 4 → cliente, 3 → administración
- [x] Añadir las reglas de capa de `session/` al [test de arquitectura](../../src/__tests__/arquitectura.test.ts) — hecho junto con lo anterior: `session` no puede importar de `app`/`ui`; `app` puede importar de `session`
- [x] Mapear los **13 tipos de error de dominio** del motor a códigos estables de `ResultadoComando.codigoError` — `src/session/erroresDeDominio.ts`, tabla `Function -> string` por constructor de error (ej. `FundacionInvalidaError -> 'fundacion.invalida'`), documentada ahí mismo
- [ ] Diseñar el `RunnerDePartida` (cola serial, scheduler, persistencia, difusión) — **después** de que `GameSession` exista, para no mezclar los dos problemas. Aquí se resuelve el tick bloqueante
  - Su contrato nace **asíncrono** (`async avanzarTick()`, `async ejecutar(comando)`) aunque por dentro no espere nada todavía: así migrar a cesión cooperativa del event loop o a un worker es un cambio interno, invisible para los llamadores (ver [7_Diseno_GameSession.md](7_Diseno_GameSession.md) §8.4)
  - **Una partida por proceso** (decidido 2026-08-24): Node ejecuta JS en un hilo, dos partidas compartirían hilo y el tick de una dejaría a la otra sin atender comandos. `GameSession` recibe `gameId` y no es singleton, así la decisión es reversible
- [ ] **Multihilo: NO construir ahora** (decidido 2026-08-24, ver doc 7 §8). Corrección de escala relevante: 500 jugadores ≈ 70-100 asentamientos, no 500 (`CIUDADANIA.casasBasePorAsentamiento` = 5 residentes + 2 por nivel), lo que da **~170-300 ms/tick** en vez de 1.9 s — tolerable sin multihilo. Además el multihilo **no resuelve** que no se apliquen comandos durante el tick (restricción lógica: un solo mutador del estado a la vez), solo evita bloquear el event loop. **Disparador para reconsiderarlo**: un tick medido por encima de ~500 ms en partida real, o desconexiones de WebSocket atribuibles al bloqueo
- [x] Elegir runtime/framework del proceso backend Node.js — **decidido: Node.js + Fastify**. Motivo: el cuello de botella medido es la CPU del tick (~1.9 s a 500 asentamientos), no el HTTP, así que el req/s del framework es irrelevante; se elige Fastify por equilibrio de ecosistema, validación de esquemas integrada (sirve al punto 10 del doc 2) y `@fastify/websocket` maduro para la Fase C
- [x] Elegir estrategia de persistencia inicial (snapshots simples vs. eventos+snapshots) y justificar la elección — **decidido: snapshot JSON por partida**, escritura atómica (`.tmp` + `rename`) con versión de concurrencia. Motivos: el estado completo mide ~2.4 MB a 500 asentamientos y serializarlo cuesta <1 ms (≈0.1% del tick); la cola serial por `gameId` elimina la concurrencia de escritura, que es la ventaja principal de SQLite; y el modelo de datos cambia entero en Fase D, así que definir esquema SQL ahora sería diseñar para algo que se va a tirar. **Disparadores para migrar a SQLite** (anotados, no ahora): auditoría/event-sourcing de Fase E, estado por encima de ~5 MB, o necesidad de consultar historial sin cargar la partida entera
- [x] **⚠️ B3 — Mutaciones de `Mapa` hechas explícitas** (2026-08-25; era prerrequisito del guardado, ver [7_Diseno_GameSession.md](7_Diseno_GameSession.md) §7.3). Era lo único no funcional puro que quedaba del motor: `extraer` (desde `engine/construction.ts`) y `avanzarRegeneracion` (desde `engine/simulation.ts`) escribían el `estadoMapa` de la partida por dentro de una fachada compartida, fuera del valor de retorno. Al persistir tras cada tick se guarda *estado + eventos*, así que un fallo de escritura descartaba el estado nuevo pero **no** revertía los yacimientos ya vaciados: snapshot en disco y partida en memoria dejaban de coincidir. Cómo se resolvió:
  - **`Mapa` ya no aliasa el estado que recibe: lo copia.** La fachada escribe en su copia y expone `estadoActual()`. Quien la creó decide si adopta el resultado o lo tira — la mutación deja de ser un efecto lateral y pasa a ser un valor
  - **El tick lo devuelve**: `ResultadoTick.estadoMapa`, que `session/comandos/avanzarTick.ts` adopta explícitamente igual que el resto del estado. Con esto **ningún comando** tiene ya estado no reconstruible desde su valor de retorno
  - **`GameSession.ejecutar` crea una fachada nueva por comando** sobre el estado actual. Es lo que cierra el resquicio de raíz: lo que un comando escriba y no devuelva se va con la fachada
  - **Y si se le olvida, salta**: un comando aceptado que escribió el mapa sin devolver `estadoMapa` lanza excepción en vez de dejar la partida desincronizada de su `version`. Un comando rechazado no la dispara — ahí descartar es lo correcto
  - **Coste**: la fachada dejó de instanciarse una vez por partida para pasar a una vez por comando, así que sus índices (ids, rejilla espacial, orden de generación) se cachean por `MapaGenerado` en un `WeakMap`, como ya se hacía con los contornos de bosque. Lo que queda por comando es copiar dos registros de números
  - **La superficie sigue siendo exactamente esos 2 puntos**, verificado por `grep` al cerrar. Tests: 349 → 352
- [x] **Paso 3 del plan de migración — `GameStore` convertido en adaptador delgado** (2026-08-25, doc 7 §6). `GameStore` ya no posee estado de simulación: la partida vive en `GameSession` y él queda como adaptador de navegador (suscripciones, historial de depuración, traducción de rechazos a log). Resultado medido:
  - **1580 → 1072 líneas**, y las **70 mutaciones directas de `this.state` → 0**
  - La interfaz **no cambió ni una línea**: `GameSessionState` es superconjunto de lo que era `GameState`, y un getter privado `get state()` hace que las 31 consultas derivadas sigan leyendo `this.state.X` sin tocarlas
  - Los tests de `GameStore` (export/import, NPC, integración) pasaron **sin modificarse** — es la prueba real de que la delegación preserva el comportamiento
  - `avanzarAutoComercio` (trueque automático de simulación, apagado por defecto) migró como tercera operación de sistema, conservando su orden: tick → auto-comercio → NPC
  - **Balance resuelto sin fingir que es partida**: `actualizarBalance`/`restaurarBalance` no pasan por `GameSession.ejecutar`; usan `registrarEventoAdministrativo`, un método explícitamente temporal para hechos que no son comandos de partida. Lo sustituye la auditoría real de Fase C
- [x] **Implementar guardado de: estado, tick, RNG, IDs, configuración, eventos, con versión de concurrencia** (2026-08-25). `src/server/persistenciaPartida.ts` — capa NUEVA (`server/`), no `session/`: `GameSession` sigue sin E/S por diseño, así que lo que toca `fs` vive aparte. `server` puede importar de `session` pero no al revés, y no de `app`/`ui`/`lab` (regla añadida al test de arquitectura)
  - **Todo lo que había que guardar ya estaba resuelto**: `GameSession.exportar()` (estado, tick, `eventosDominio`, mapa/yacimientos, IDs, y desde el commit anterior el RNG) ya devolvía `PartidaExportada` completo. Este módulo solo añadió el CÓMO: escritura atómica (`.tmp` + `rename`, decidido en este doc) y la comprobación de versión
  - **Escritura atómica**: un fallo a mitad de escritura nunca deja un archivo a medias — el nombre final o tiene la versión anterior completa, o la nueva completa, nunca algo entre medias
  - **Versión de concurrencia, como red de seguridad y no como mecanismo principal**: la cola serial por `gameId` del futuro `RunnerDePartida` ya elimina la concurrencia de escritura en operación normal (motivo por el que se descartó SQLite, ver más abajo en este mismo doc). `guardarPartida` rechaza sobreescribir con una versión de partida MENOR que la que ya hay en disco (`ConflictoDeVersionError`) — detecta el síntoma de un bug real (dos procesos escribiendo el mismo `gameId`) en vez de perder datos en silencio. Guardar la MISMA versión dos veces no lanza: es un reintento válido tras un fallo de escritura, no un conflicto
  - `cargarPartida` reconstruye una `GameSession` operable y en continuidad de RNG — verificado a través del disco de verdad, no solo en memoria, con el mismo patrón de prueba que la continuidad de RNG de `session/`
  - Tests: 357 → 366, en un directorio temporal real por test (`mkdtemp`/`rm`) — un fake en memoria no habría ejercitado `rename` en absoluto
- [x] Persistir `eventosDominio` en el snapshot desde el principio — cubierto por lo de arriba: `PartidaExportada.state.eventosDominio` viaja dentro del snapshot sin que este módulo tuviera que hacer nada especial, ya era parte del estado de partida
- [x] **Definir y documentar el esquema de snapshot/versión de formato de partida** — `SnapshotPartida { formatoVersion, guardadoEn, partida: PartidaExportada }` en `src/server/persistenciaPartida.ts`, documentado en el propio código: tres versiones DISTINTAS y no confundibles conviven en el archivo — `formatoVersion` (forma del envoltorio, sube solo si cambia la forma del JSON), `partida.state.version` (versión de LA PARTIDA, sube en cada comando aceptado, es la de concurrencia) y `partida.worldgenVersion` (versión del algoritmo de generación de mundo, ya existía). Cargar un snapshot valida las tres: formato no soportado (`FormatoSnapshotNoSoportadoError`) y worldgen distinto (`WorldgenVersionNoCoincideError`, mismo criterio que ya usaba `GameStore.importarSimulacion` para el formato v2) se rechazan explícitamente en vez de cargar algo que no es lo que dice ser
- [ ] Exponer API HTTP administrativa mínima (crear partida, avanzar tick, consultar estado)
- [ ] Migrar `main.ts` para hablar con esa API en vez de con `GameStore` local (puede convivir temporalmente con un modo local para desarrollo)
- [ ] **Resolver el bloqueo de la cola serial por ticks largos** — un tick de ~1.9 s a 500 asentamientos son ~1.9 s sin procesar comandos de nadie (el doc 2 no lo contempla). Vías a evaluar: comandos por lotes entre ticks, partir el tick en fases cedibles, o mover el tick a un worker aparte del hilo que atiende comandos. **Decisión pendiente** — se aborda dentro del diseño del `RunnerDePartida` (ver [7_Diseno_GameSession.md](7_Diseno_GameSession.md) §4): al quedar la cola FUERA de `GameSession`, estas tres vías se pueden probar y cambiar sin tocar la lógica de partida

## Fase C — Multijugador sobre ticks

> **⚠️ Gate de entrada (decidido 2026-08-25): el marcador de A5 tiene que estar en 13/13 antes de empezar
> cualquier tarea de esta fase.** Ver la nota en A5, arriba. No es necesario para el resto de la Fase B.

- [ ] Implementar `Usuario`, `Sesion`, `Membresia` según el diseño de A6
- [ ] Implementar chequeo de autorización antes de aplicar cada comando (rol + facción + asentamiento + cargo)
- [x] Elegir transporte para tiempo real (WebSocket o SSE) — **decidido: WebSocket, UNA sola conexión permanente por jugador con canales lógicos multiplexados encima** (patrón Phoenix Channels / Socket.IO rooms). Descartada la alternativa de una conexión por pantalla: abrir una conexión cuesta 2-3 round trips (100-300 ms en móvil) más re-autenticación, así que el jugador pagaría ese coste en cada cambio de pestaña; mantenerla ociosa cuesta ~10-30 KB de RAM y 500 conexiones son triviales para Node. Ver [6_Sincronizacion_Visibilidad_y_Escala.md](6_Sincronizacion_Visibilidad_y_Escala.md) §2
- [ ] Implementar el protocolo de suscripciones sobre esa conexión única (`suscribir`/`desuscribir` a canales tipo `mapa/general`, `asentamiento/X`) — el servidor solo empuja lo que el jugador tiene suscrito
- [ ] Implementar reconexión de cliente sin duplicar comandos (idempotencia vía `idempotencyKey`) — **incluye re-suscripción**: al reconectar se pierden las suscripciones y el cliente debe restablecerlas solo
- [ ] Diseñar y construir DTOs/proyecciones por audiencia: jugador, facción, admin, observador
- [ ] **Diseñar `ConocimientoJugador`** (entidad de dominio nueva, hoy inexistente): qué sabe cada jugador, desde cuándo y hasta cuándo es válido. Fuentes de visibilidad: espacial (zona de influencia + valor de visualización), contacto (trueque, mientras siga vivo) y alianza
- [ ] **Implementar la proyección "último conocido"** — **decidido 2026-08-24**: un jugador ve el estado que la entidad ajena tenía en el momento del contacto, NO el actual (patrón de niebla de guerra de RTS). Evita filtrar telemetría en vivo de rivales y reduce el tráfico a cero para lo ajeno. Cada dato congelado va marcado con el momento en que se supo, para que el frontend pueda mostrar "última información conocida: hace X"
- [ ] Construir frontend de jugador (acciones y datos restringidos a su proyección)
- [ ] Migrar balance de módulo global mutable a configuración versionada por partida/temporada, con auditoría de cambios (actor, fecha, versión anterior/nueva)
- [ ] Separar rutas/endpoints de administración de las de jugador, protegidas por rol técnico

## Fase D — Conversión temporal total

- [ ] Introducir reloj de simulación y campos de fecha en el estado, sin retirar aún el tick
- [ ] Migrar construcción, políticas y cooldowns de `ticksRestantes`/`enTick` a duraciones o fechas de finalización
- [ ] Migrar producción/consumo/población/hambre/mantenimiento a tasas o acumuladores por tiempo transcurrido
- [ ] Migrar caravanas (salida, velocidad, tiempo transcurrido en vez de posición por tick)
- [ ] Migrar comercio, bandidos, NPC y combate a eventos temporales
- [ ] Escribir migración explícita de partidas guardadas en formato "ticks" a formato "tiempo real"
- [ ] Actualizar DTOs y frontends: contadores → fechas/duraciones/eventos
- [ ] Recalibrar valores de balance con simulaciones de referencia tras el cambio de modelo temporal
- [ ] Persistir `ultimoProcesadoEn` y procesar eventos vencidos tras un reinicio de servidor

## Fase E — Operación persistente

- [ ] Implementar scheduler temporal definitivo (reemplaza el avance manual de tick como mecanismo principal)
- [ ] Recuperación de eventos vencidos tras caída/reinicio, verificada con pruebas
- [ ] Pipeline de auditoría (quién, qué comando, cuándo, resultado, versión de partida)
- [ ] Backups automáticos + prueba de restauración documentada
- [ ] Métricas: duración de tick/procesamiento, tamaño de cola, tasa de errores, clientes conectados
- [ ] Herramientas de moderación para administradores
- [ ] Diseño e implementación de ciclos de servidor, Maravilla, legado NPC, temporadas

---

## Backlog de riesgos a mitigar (de 2_Estudio_Evolucion_Backend_Multifrontend.md)

Marcar cuando la mitigación correspondiente esté implementada y verificada, no
solo diseñada.

- [ ] IDs resueltos exclusivamente en servidor, nunca confiados desde el cliente
- [ ] Cola serial o control de versión por partida para comandos concurrentes
- [x] RNG determinista con estado persistido (partidas reproducibles tras reinicio) — `PartidaExportada.estadoRng` + `src/server/persistenciaPartida.ts` (2026-08-25)
- [ ] DTOs/proyecciones por audiencia (nunca enviar `GameState` completo a un cliente no-admin)
- [ ] Snapshots y retención para el historial (nunca clones ilimitados en RAM)
- [ ] Balance versionado y ligado a partida/temporada (no global mutable)
- [ ] Frontends y endpoints de admin vs. jugador separados con roles técnicos distintos
