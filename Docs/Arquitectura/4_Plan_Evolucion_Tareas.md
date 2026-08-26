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

### A5. Eventos de dominio estructurados — ✅ completada 2026-08-25

> **Gate de Fase C cerrado.** Los 13 subsistemas de `engine/`+`world/` que producían texto libre dentro del
> tick ya tienen `codigo` estable y `payload` tipado — ver marcador de progreso abajo (13/13). Motivo del gate
> (seguía vigente hasta hoy): Fase C construye autorización y proyecciones por audiencia sobre los comandos, y
> la auditoría/replay de Fase E que `eventosDominio` existe para servir necesita códigos estables para poder
> filtrarse por tipo — con esto cerrado, Fase C ya puede empezar.

**Base (completada):**
- [x] Definir tipo `EventoDominio` — módulo nuevo `src/domain/eventos.ts` (no `domain/types.ts`, que es solo contratos de entidades de juego): `{ codigo, mensaje, tick, asentamientoId? }`. Todo evento hoy usa `codigo: 'legado'` — no hay catálogo de códigos todavía, eso es justo lo que falta migrar
- [x] Hacer que `engine/simulation.ts` (`avanzarSimulacion`) devuelva `EventoDominio[]` junto al estado — nuevo campo `eventosDominio` en `ResultadoTick`, poblado en paralelo a `eventos` (helper `comoEventosDominio`) en cada punto donde el tick ya empujaba texto a `eventos`; incluye `asentamientoId` en los eventos del bucle por asentamiento, ausente en los globales (comercio, mercado, tributos, nivel de facción, títulos, regeneración de yacimientos, ataques de bandidos)
- [x] Test de contrato nuevo: `src/engine/__tests__/eventos_dominio.test.ts` — mismo contenido 1:1 que `eventos`, tick correcto, `asentamientoId` presente/ausente donde corresponde
- [x] Verificación de la base: `tsc --noEmit` limpio, 41/41 archivos y 271/271 tests en verde

**Preparación para tiempo real (completada 2026-08-24):**
- [x] Añadir `momento` (ISO 8601) a `EventoDominio` — es el campo temporal DEFINITIVO que viaja a los clientes; `tick` queda marcado como provisional en el propio tipo. Implementa la regla (b) de [6_Sincronizacion_Visibilidad_y_Escala.md](6_Sincronizacion_Visibilidad_y_Escala.md) §4
- [x] Introducir `ContextoSimulacion { tick, momento, rng }` — cumple el punto 3 de la Fase A del doc 2 ("Introducir contexto de partida para RNG e IDs"). `avanzarSimulacion(estado, mapa, contexto)` y `avanzarNpcGobernanza(estado, mapa, contexto, config)` comparten ahora la misma forma de llamada. **El motor no lee nunca el reloj ni la aleatoriedad global**: ambos se inyectan, que es lo que mantiene la simulación reproducible. Al pasar a tiempo real, `tick` desaparece de este contexto sin cambiar ninguna firma
- [x] `GameStore.contextoDeTickActual()` — la capa de aplicación es la dueña del reloj (`new Date()`); tests y batch derivan `momento` del tick para no romper determinismo (`contextoDeTest` en `engine/__tests__/fixtures.ts`)

**Antes pendiente, resuelto por otra vía:**
- [x] Traducir `EventoDominio[]` a las entradas de log en texto — no hizo falta tocar `GameStore` (ya no existe: retirado en la migración de `main.ts` a la API, ver más abajo en este doc). Lo resuelve `exito()` en `session/comandos/tipos.ts`: `log: [...eventos.map(e => ({tick: e.tick, mensaje: e.mensaje})), ...estado.log]` — el log en texto que ya muestra la interfaz sale de `eventosDominio.mensaje`, no de un array aparte

#### 📊 Marcador de progreso — migración por subsistema: **13 / 13 migrados (100%)**

Migrar un subsistema = darle su propio `codigo` estable (dejar de usar `'legado'`) y mover lo que hoy es
texto libre a `payload` tipado en `EventoDominio`. Mecanismo (2026-08-25): `EventoDominio` gana un campo
`payload?: unknown`, y se introduce `EventoCrudo` (`domain/eventos.ts`) — `string | {codigo, mensaje, payload?}`
— como lo que un subsistema empuja a su array `eventos` DENTRO del tick, antes de que `engine/simulation.ts` le
añada `momento`/`tick`/`asentamientoId` centralmente (`comoEventosDominio`). Permitió migrar subsistema por
subsistema sin tocar los demás cada vez: uno sin migrar sigue empujando `string` plano (atajo "legado"), uno
migrado empuja el objeto — `mensaje` se mantiene en ambos casos, así que el log en texto que ya mostraba la
interfaz no cambia. Cada subsistema exporta sus propias interfaces de payload (ej. `PayloadEdificioCompletado`)
en vez de una unión discriminada global de ~30 variantes — más simple de mantener, y ningún consumidor real
existe todavía que necesite narrowing exhaustivo por `codigo`.

- [x] `engine/construction.ts` (`avanzarConstruccion`) — 5 códigos: `construccion.mejora_completada`, `.edificio_completado`, `.yacimiento_agotado`, `.iniciada`, `.necesidad_detectada`
- [x] `engine/politicas.ts` (`avanzarPoliticas`) — `politica.expirada`
- [x] `engine/tropas.ts` (`avanzarMantenimientoTropas`) — `tropas.desercion`
- [x] `engine/mantenimiento.ts` (`avanzarNivelAsentamiento`, `avanzarMantenimiento`) — 5 códigos: `asentamiento.nivel_subio`, `mantenimiento.recuperado`, `.colapsado`, `asentamiento.ruinas`, `mantenimiento.deficit`
- [x] `engine/population.ts` (`avanzarNutricionPoblacion`, `crecerPoblacion`) — 3 códigos: `poblacion.primeros_artesanos`, `.primeros_nobles`, `.hambruna_muerte`
- [x] `engine/trade.ts` (`avanzarComercio`) — 5 códigos: `comercio.caravana_llega`, `.peaje`, `.trueque_cumplido`, `.trueque_expirado`, `.caravana_sale`
- [x] `engine/expansion.ts` (`avanzarCaravanasFundacion`) — 3 códigos: `expansion.caravana_perdida`, `.asentamiento_fundado`, `.fundacion_fallida`
- [x] `world/mapa.ts` (`avanzarRegeneracion`) — `mapa.yacimiento_regenerado`
- [x] `engine/bandidos.ts` (`avanzarSpawnBandidos`, `avanzarAtaquesBandidos`) — 3 códigos: `bandidos.campamento_aparece`, `.caravana_interceptada`, `.caravana_escapa`
- [x] `engine/market.ts` (`avanzarMercado`) — `mercado.compra`
- [x] `engine/diplomacia.ts` (`avanzarTributos`) — `diplomacia.tributo_pagado`
- [x] `engine/faccion.ts` (`avanzarNivelesFaccion`) — `faccion.nivel_subio`
- [x] `engine/titulos.ts` (`narrarCambiosDeTitulo`) — 2 códigos: `titulo.cambia_manos`, `titulo.nace`

Verificación: `tsc --noEmit` limpio y suite completa en verde tras cada subsistema (no en un solo golpe al
final) — 400 → 432 tests (un test nuevo por código, salvo un puñado ya cubiertos por tests existentes a los
que solo hizo falta ajustar la aserción de texto libre a `codigo`/`payload`, ej. `trade_peaje.test.ts`,
`regeneracion_nodos.test.ts`, `hambruna.test.ts`). El contrato general (`eventos_dominio.test.ts`) se aflojó a
propósito: ya no exige `codigo === 'legado'` en todos los eventos (dejó de ser cierto), solo que `codigo` nunca
venga vacío — la cobertura de QUÉ código concreto lleva cada evento vive en el test de su propio subsistema.

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
- [x] **`RunnerDePartida` implementado** (2026-08-25) — `src/server/runnerDePartida.ts`. Cola serial + ciclo "aplicar -> persistir -> confirmar" (doc 7 §2(a)) + scheduler de ticks automáticos, sobre UNA `GameSession` y el `persistenciaPartida.ts` del commit anterior. La **difusión** (a clientes, vía WebSocket) queda fuera a propósito: necesita el protocolo de suscripciones de Fase C, que todavía no existe
  - Su contrato nace **asíncrono** (`ejecutar()`/`avanzarTick()` devuelven `Promise`) aunque por dentro no espere nada más que la propia escritura a disco: migrar a cesión cooperativa del event loop o a un worker es un cambio *dentro* de este archivo, invisible para quien lo llama (doc 7 §8.4)
  - **Cola serial sin mutex**: JS ya es de un solo hilo, así que basta con encadenar promesas (`this.cola = this.cola.then(trabajo)`) para que dos llamadas lanzadas sin esperar la primera se apliquen en el orden de llegada, nunca intercaladas — verificado lanzando 3 comandos "a la vez" y comprobando que el estado resultante refleja el orden exacto
  - **"Aplicar -> persistir -> confirmar" con descarte real**: `GameSession` no tiene "deshacer" (es deliberadamente sin E/S), así que si `guardarPartida` falla, el runner reconstruye la sesión desde el snapshot previo a la operación (`GameSession.importar`, barato: `exportar()` solo copia referencias, no clona) y relanza el error — el comando queda fuera de la partida en memoria tanto como del disco, nunca a medias entre los dos
  - **`esperarColaVacia()`**: se resuelve cuando todo lo encolado hasta ese instante termina. Sirve para un apagado limpio del proceso (esperar a que la última escritura termine antes de cerrar) y es lo que hizo falta para probar el scheduler de forma determinista bajo carga (ver nota de tests abajo)
  - **Scheduler con timers reales en los tests, no `vi.useFakeTimers()`**: cada tick dispara E/S real a disco, y avanzar un reloj falso no adelanta una escritura de archivo de verdad — con timers falsos la mayoría de los ticks programados no llegaban a completar su persistencia antes de que el test comprobara el resultado (falso negativo, detectado al escribir el test). Con timers reales e intervalos de 10 ms el coste es de un puñado de cientos de ms por test
  - **Una partida por proceso** (decidido 2026-08-24, sigue vigente): Node ejecuta JS en un hilo, dos partidas compartirían hilo y el tick de una dejaría a la otra sin atender comandos. `GameSession` recibe `gameId` y no es singleton, y ahora tampoco `RunnerDePartida` — así la decisión sigue siendo reversible
  - Capa `server/` (no `session/`): mismo criterio que `persistenciaPartida.ts`, ya cubierto por la regla del test de arquitectura del commit anterior
  - Tests: 366 → 375
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
- [x] **Exponer API HTTP administrativa mínima (crear partida, avanzar tick, consultar estado)** (2026-08-25) — `src/server/api.ts` (Fastify, decidido en este mismo doc) + `src/server/index.ts` como punto de arranque (`npm run server`, puerto y directorio por variables de entorno). Exactamente los 3 endpoints pedidos, nada más:
  - `POST /partidas { gameId, seed, region? }` — crea una partida nueva, o RETOMA la que ya hubiera en disco para ese `gameId` (mismo criterio que `RunnerDePartida.cargarOCrear`). No hay endpoint aparte de "reanudar": con solo tres endpoints mínimos, esta es la única forma de que un proceso reiniciado reabra una partida existente — verificado con un test que crea un servidor Fastify nuevo apuntando al mismo directorio ("reinicio del proceso") y comprueba que el tick no vuelve a 0
  - `POST /partidas/:gameId/tick` — avanza un tick a través del `RunnerDePartida` (cola + persistencia ya incluidas, gratis)
  - `GET /partidas/:gameId` — estado completo. Sin proyección ni filtrado: es endpoint de ADMINISTRACIÓN, el mismo criterio que ya distinguía el doc 5 — un cliente de jugador real nunca debe llegar a este endpoint tal cual, eso es Fase C
  - **Registro en memoria por `gameId`** dentro de `crearServidor()`: no es un caché de conveniencia, es lo que impide que dos peticiones de creación concurrentes para el mismo `gameId` acaben con dos `RunnerDePartida` escribiendo el mismo archivo — 409 si ya está abierto en este proceso
  - Validación de esquema con la propia integración de Fastify (motivo por el que se eligió en su día, ver más abajo en este doc) — un `POST /partidas` sin `seed` o con una `region` que no exista en `RegionId` se rechaza con 400 antes de tocar `RunnerDePartida`
  - Probado con `app.inject()` (sin abrir sockets reales) y además con un smoke test end-to-end real: proceso arrancado de verdad, `curl` contra los 3 endpoints, snapshot confirmado en disco
  - Tests: 375 → 384 (nota: `src/world/__tests__/exportUnity.test.ts` tiene un timeout de 5 s intermitente bajo la suite completa por contención de CPU, preexistente a esta sesión — verificado con `git stash` contra el commit anterior; pasa siempre en solitario, no relacionado con este trabajo)
  - Añade `fastify` (dependencia de producción) y `tsx` (devDependency, para `npm run server` sin compilar)
- [x] **Migrar `main.ts` para hablar con la API HTTP** (2026-08-25) — reemplazo COMPLETO, no modo dual (decidido con el usuario: revisa la redacción original de esta tarea). `main.ts` ya no posee ninguna `GameSession` en memoria del navegador; sin `npm run server` corriendo no se puede jugar.
  - **Endpoint genérico de comandos** (`POST /partidas/:gameId/comandos { tipo, params }`, `src/server/api.ts`) — la pieza que faltaba: la API solo tenía 3 endpoints (crear, tick, consultar), ninguno de los 31 comandos de partida tenía cómo llegar al backend. Despacha por un registro por nombre (`src/session/comandos/registro.ts`) que reutiliza los mismos manejadores — el propio `GameSession.ejecutar` ya anotaba que este registro haría falta "en Fase C, cuando la API reciba comandos serializados"; se construyó un ciclo de fase antes de lo previsto
  - **Separación jugador/administración (revertida el mismo día)**: balance, importar partida y el slider de histórico de ticks son operaciones de ADMINISTRACIÓN, no de jugador — mismo criterio que ya distingue `GET /partidas/:gameId` en el doc 5 — y se retiraron de `GameStore`/`main.ts` sin dejarlos a medias (incluye borrar `app/balanceConfig.ts`, huérfano tras el retiro); quedan sin dueño, pendientes de una tarea futura. Regenerar mundo se probó primero en un punto de entrada propio, `admin.html`/`src/admin.ts` (mismo patrón que `lab/laboratorio.html`), pero el usuario corrigió el criterio: hoy no hay separación real entre "cliente de jugador" y "herramienta de administración" — una sola interfaz sirve a los dos propósitos, a propósito, hasta que exista un cliente de jugador de verdad (Unity, agnóstico de este backend). `admin.html`/`admin.ts` se eliminaron y `regenerarMundo` volvió a `GameStore`/`main.ts` (con confirmación explícita del usuario antes de llamarlo, por ser destructivo — `POST /partidas` con `forzar: true`)
  - **`GameStore` pasa a ser un adaptador de red puro** — ya no importa `GameSession` ni los manejadores de comando; cachea la última foto del estado (`estadoCache`, refrescada con un `GET` tras cada comando aceptado) y sus ~29 acciones de jugador son ahora `async`. `main.ts` no se reescribe en un IIFE: usa `await` de nivel de módulo (soportado por el `target: ES2022` del proyecto), así que el resto del archivo sigue leyendo `gameStore` igual que antes
  - **`RunnerDePartida.avanzarTick()` corregido para bundlear auto-comercio + turno NPC** — antes solo aplicaba el tick puro del motor; `GameStore.avanzarTick()` (ahora retirado) los encadenaba los tres. Sin este fix el comportamiento habría cambiado en silencio al migrar
  - **Dos bugs de integración real encontrados y corregidos verificando en navegador** (no los detectaba `tsc` ni la suite, por diseño — son de la capa HTTP): (1) `POST /partidas` sin `forzar` devuelve 409 si el `gameId` ya está abierto en el proceso — correcto para el panel de admin, pero rompía la reconexión normal de `main.ts` en cada recarga de página (ya había una partida abierta de una carga anterior); `GameStore.crear()` ahora trata ese 409 como "ya está lista, conectate" en vez de propagarlo. (2) `apiCliente.ts` mandaba `content-type: application/json` en TODAS las peticiones, incluida `POST /tick` sin body — Fastify rechaza con 400 un `content-type: application/json` sobre un cuerpo vacío; el header ahora solo se manda cuando de verdad hay body
  - Cobertura de NPC portada de `app/__tests__/faccionNpc.test.ts` (probaba el `GameStore` local síncrono, ya no existe en esa forma) a `session/__tests__/npcGobernanza.test.ts`, contra `GameSession` directo — mismo comportamiento blindado, sin necesitar red. `app/__tests__/exportarImportar.test.ts` se retira sin reemplazo: probaba `importarSimulacion`, que ya no existe (admin, sin dueño todavía)
  - Tests: 405 → 400 (neto: -10 de las dos suites de `app/__tests__/` retiradas, +6 de `npcGobernanza.test.ts`, +9 del endpoint de comandos/`forzar`/bundling de tick en el commit anterior)
- [ ] **Resolver el bloqueo de la cola serial por ticks largos** — un tick de ~1.9 s a 500 asentamientos son ~1.9 s sin procesar comandos de nadie (el doc 2 no lo contempla). Vías a evaluar: comandos por lotes entre ticks, partir el tick en fases cedibles, o mover el tick a un worker aparte del hilo que atiende comandos. **Decisión de LA SOLUCIÓN sigue pendiente, a propósito** (doc 7 §8.2: a la escala de arranque el tick mide ~170-300 ms, tolerable; el disparador para reconsiderar es un tick medido por encima de ~500 ms en partida real). Lo que sí se hizo en `RunnerDePartida` (2026-08-25) fue la preparación de coste cero del doc 7 §8.4: `ejecutar()`/`avanzarTick()` ya son `async` de cara a quien los llama, así que cualquiera de las tres vías se puede implementar DENTRO del runner el día que haga falta, sin cambiar su interfaz ni tocar `GameSession`

## Fase C — Multijugador sobre ticks (**solo servidor**)

> **✅ Gate de entrada cumplido (2026-08-25): A5 llegó a 13/13.** Ver la nota en A5, arriba.
>
> **✅ Gate ampliado el mismo día**: los eventos de COMANDO (30 llamadas a `eventoLegado` + `engine/combate.ts`,
> `engine/fusion.ts`, `rebelionVasallo`) también se migraron a `codigo`/`payload`. A5 había cerrado el lado del
> TICK, pero Fase C construye autorización y proyecciones **sobre los comandos** — y los eventos de combate,
> los más sensibles a visibilidad, seguían siendo texto libre. Ver el bloque "Eventos de comando" más abajo.

### Replanteo por separación de repositorios (2026-08-25)

Este repositorio pasa a ser **solo servidor**. El cliente de jugador vive en otro repositorio y el de
administración ya existe fuera. Lo que cambia respecto al plan original de Fase C:

- **Sale de alcance** "construir frontend de jugador".
- **La API pasa a ser el producto**: dos consumidores externos, en repos que no comparten build ni tipos con
  este. Cada forma que sale por el cable es un contrato caro de coordinar.
- **Las proyecciones dejan de ser refinamiento y pasan a ser frontera de seguridad**: hoy
  `GET /partidas/:gameId` devuelve el estado completo (~2,4 MB, todas las facciones, log global). Con un
  cliente de jugador externo, todo lo que el servidor mande el jugador lo tiene, lo pinte o no su interfaz.
- **Entra lo que no estaba**: CORS (hoy lo evita el proxy de Vite del cliente), versionado de la API,
  publicación del contrato como OpenAPI, y una respuesta de comando autosuficiente (el patrón actual
  "comando → `GET` del estado entero" no sobrevive a las proyecciones).

**Orden**: C1 → C2 → C3 son secuenciales. C4 se puede *diseñar* en paralelo y conviene hacerlo pronto: es lo
que los otros repos necesitan saber para avanzar. Prioridad de superficie: **administración primero** (su
cliente ya existe y no necesita proyecciones, así que valida C1–C3 sin esperar a C4).

### C0. Extracción del cliente — ✅ completada 2026-08-25

- [x] `index.html`, `laboratorio.html`, `src/main.ts`, `src/app/`, `src/ui/`, `src/lab/` y `vite.config.ts` movidos a `cliente/` con `git mv` (historial conservado), con `package.json`/`tsconfig.json`/`vite.config.ts` propios y `README.md`
- [x] ~~**La costura queda en un solo sitio**: al sacar la carpeta a su repo se repunta el alias `@motor/*` — tres opciones documentadas en `cliente/README.md`~~ — **premisa falsa, corregida el 2026-08-26**. Repuntar el alias mueve el motor de sitio, no lo elimina: existe porque el cliente CALCULA lo que el servidor no expone (26 consultas derivadas) y porque el terreno que el servidor manda es inservible sin `worldgen/`. Ver [Diagnóstico de aislamiento del cliente](#diagnóstico-de-aislamiento-del-cliente-2026-08-26) más abajo y los hitos C8–C13. `cliente/src/lab/` (13 imports) ya no cuenta: eliminado
- [x] Backend adelgazado: fuera Vite de `package.json`, fuera `DOM` de `tsconfig.lib`, `scripts` reducidos a `server`/`typecheck`/`test`
- [x] `arquitectura.test.ts`: capas `app`/`ui`/`main`/`lab` retiradas del contrato (ya no existen aquí) y lectura del árbol pasada de `import.meta.glob` (Vite) a `node:fs` — su comentario justificaba el glob con "proyecto 100% navegador/Vite, sin `@types/node`", premisa que dejó de ser cierta
- [x] Corregido de paso un fallo latente: `vite build` fallaba por el `await` de nivel de módulo de `main.ts` (Vite no heredaba el `target: ES2022` del tsconfig). `vite dev` funcionaba, así que nadie lo había visto
- [x] Verificado en vivo: backend y cliente arrancados desde sus nuevas ubicaciones, partida persistida retomada, log correcto, 444/444 tests y `tsc` limpio en ambos proyectos

### C1. Identidad y autenticación — parcialmente completada 2026-08-25

- [x] `Usuario`, `Sesion`, `Membresia`, `IdentidadVinculada` (`src/acceso/tipos.ts`) según el diseño de A6. `Jugador` NO se declara: hoy no tendría consumidor (`Membresia.jugadorId` ya lleva el vínculo), y una interfaz sin uso es peso muerto — el contrato sigue en el doc 5
- [x] **Autenticación tras un puerto intercambiable** (patrón Strategy/Ports & Adapters, pedido explícito): `ProveedorIdentidad` (`identidad/proveedorIdentidad.ts`) es el único contrato que un proveedor debe implementar — verificar una credencial y devolver `{proveedor, sujetoId, email?}`. `identidad/registroProveedores.ts` los indexa por esquema de `Authorization`, igual que `REGISTRO_COMANDOS` indexa comandos. Añadir/quitar un proveedor real es escribir/borrar un adaptador + una línea en `proveedoresPorDefecto()`, sin tocar `servicioAutenticacion.ts` ni las rutas HTTP
- [x] `identidad/proveedorDesarrollo.ts`: único adaptador activo hoy (esquema `dev`, no verifica nada criptográficamente) — a retirar del registro cuando haya un proveedor real
- [x] `identidad/repositorio.ts`: puerto `RepositorioIdentidad` + implementación en memoria (se pierde al reiniciar — aceptable mientras el único proveedor sea el de desarrollo)
- [x] `identidad/servicioAutenticacion.ts`: `autenticar()` (login: cabecera de proveedor -> Usuario find-or-create -> Sesion nueva) y `resolverSesion()` (cabecera `Authorization: sesion <id>` -> Usuario/Sesion vigente)
- [x] Wired en `server/api.ts`: `POST /sesiones` (login) y `GET /sesiones/actual` (whoami) — 27 tests nuevos (unitarios + HTTP vía `.inject()`)

### C2. Autorización por comando — completada 2026-08-25

- [x] `server/autorizacion/matriz.ts`: matriz de autorización (doc 5) convertida en datos — una fila por cada uno de los 30 comandos de `session/comandos/registro.ts`, con `rolesPermitidos` + `condicionJugador` (Facción propia / residente / cargo local / rey-embajador, según el comando). Exhaustividad garantizada en tiempo de COMPILACIÓN (tipo indexado `{ [T in TipoComando]: EntradaMatriz<T> }`, no solo un test) y reforzada por un test que compara claves contra `REGISTRO_COMANDOS`
- [x] `server/autorizacion/condiciones.ts`: predicados de dominio (`esFaccionPropia`, `esResidente`, `tieneCargoLocal`, `esReyOEmbajadorDe`) — conviene "fail-open" cuando la entidad referenciada no existe (deja que el propio comando la rechace con su código `*.no_existe`, en vez de un 403 engañoso)
- [x] `server/autorizacion/verificar.ts`: `verificarAutorizacion(tipo, params, estado, actor)` — filtro de rol técnico primero, condición de dominio solo si el actor entra como `'jugador'` (doc 5: "sin restricción si es admin")
- [x] Dos filas que faltaban en la tabla del doc 5 (`proponerRelacion`, `proponerTrueque` — de los 30 comandos reales, no estaban en la versión original de la tabla): añadidas ahí y en la matriz, con la regla inferida documentada en ambos sitios
- [x] Wired en `server/api.ts`: `POST /partidas/:gameId/comandos` ahora exige `Authorization: sesion <id>` (401 si falta/expiró), `Membresia` para ese `gameId` (403 si no existe), y pasa la `verificarAutorizacion` (403 con el motivo si falla) antes de despachar al manejador — el actor que llega al motor es `Membresia.jugadorId`, nunca un id que el body afirme tener
- [x] Endpoint nuevo `POST /partidas/:gameId/jugadores` — unirse a una partida como jugador (crea `Membresia(rol='jugador', faccionId=null)`, reutiliza `usuarioId` como `jugadorId` en vez de sumar un generador de ids); es el bootstrap mínimo para que los 29 comandos gateados por rol `'jugador'` sean alcanzables y comprobables de punta a punta
- [x] 27 tests nuevos (18 unitarios sobre la matriz con estado de partida genuino vía la fixture de `session/__tests__/`, 9 HTTP vía `.inject()`) — 495/495 en total, `tsc --noEmit` limpio
### C2.1. Separación negocio / infraestructura — completada 2026-08-25

Revisión pedida tras C2: cuanto más limpia sea la frontera, más fácil es tocar un lado sin afectar al otro.
El diagnóstico fue que `server/autorizacion/` era **negocio disfrazado de infraestructura** — la matriz
acabó ahí para satisfacer una restricción de dependencias (`RolTecnico` vivía en `server/`, y `session` no
puede importar de `server`), y para lograrlo reimplementó reglas de juego que ya existían en `engine/`.

- [x] **Capa `acceso/` nueva**: `Usuario`, `Sesion`, `Rol`, `Membresia`, `IdentidadVinculada`, los puertos `ProveedorIdentidad`/`RepositorioIdentidad` y el servicio de autenticación. Es el DOMINIO DE ACCESO — negocio, no infraestructura — y **no importa nada** (test propio que lo vigila): un `Usuario` existe fuera de cualquier partida y su autenticación no sabe de HTTP
- [x] **`server/identidad/` queda como adaptadores**: `proveedorDesarrollo`, `repositorioEnMemoria`, `proveedoresActivos` (qué adaptadores están dados de alta) y `cabeceraAutorizacion` (parseo de `Authorization`, que el servicio de autenticación ya no conoce — la política es la misma si mañana la credencial llega por WebSocket)
- [x] **La matriz vuelve a `session/comandos/autorizacion.ts`**, junto a los manejadores: es donde el original de `comandos/tipos.ts` la quería, y ahora se puede porque `session` ve `acceso`. Sus condiciones ya no reimplementan reglas, delegan en el motor
- [x] **`engine/pertenencia.ts`** (módulo hoja): única definición de residencia, titularidad de cargo y autoridad de Rey/Embajador. Elimina duplicación medida — el mapa `CargoTipo -> campo` estaba **4 veces** (2 en `cargos.ts`, 1 en `politicas.ts`, 1 en autorización) y la regla de residencia **3** (`tropas.ts`, `faccion.ts`, autorización); ahora 1 y 1. Distingue explícitamente `cargoOcupado` (¿hay Gobernador? — lo que pregunta el motor) de `tieneCargoLocal` (¿eres TÚ el Gobernador? — lo que pregunta la autorización)
- [x] **`Membresia.faccionId` retirado**: era una segunda fuente de verdad frente a `Faccion.ciudadanosIds` y quedaba obsoleta tras `anexionar`/`fusionar`. La Facción del actor se deriva del estado en cada chequeo. Ver la nota en el doc 5
- [x] **Dos guardas nuevos en `arquitectura.test.ts`**, verificados rompiéndolos a propósito: "nada fuera de `server` importa de `server`" y "`acceso` no importa nada". El contrato de capas solo vigilaba la DIRECCIÓN de los imports, no que cada capa contuviera lo que le toca — por eso los 495 tests pasaban con reglas de juego dentro de infraestructura
- [x] Los tests de `acceso/` se satisfacen con dobles locales, sin tocar `server/`: si algún día necesitaran un adaptador real, sería la señal de que la separación se rompió
- [x] 504/504 tests, `tsc` limpio. Un test tuvo que corregirse por un fallo REAL que el refactor destapó: usaba una Facción inexistente para probar "Facción ajena", y con la convención de fail-open eso es un rechazo del comando (200 + código), no un 403 — ahora hay un test para cada uno de los dos casos

### C3. Superficies separadas admin / jugador — completada 2026-08-25

- [x] **`/admin/*`**: `POST /admin/partidas` (crear o retomar), `POST /admin/partidas/:gameId/tick`, `GET /admin/partidas/:gameId` (estado completo), `POST /admin/partidas/:gameId/comandos`. Exige `administrador_global` o `Membresia` de administración en esa partida
- [x] **`/jugador/*`**: `POST /jugador/partidas/:gameId/membresia` (unirse) y `POST /jugador/partidas/:gameId/comandos`. Exige rol `jugador`. **Sin endpoint de lectura de estado a propósito** — el único `GET` que existe devuelve el estado completo, que para un jugador es una fuga; llega con las proyecciones de C4. Exponerlo "de momento" sería el atajo que luego nadie retira
- [x] **Cerrado el pendiente de C2**: crear partida, avanzar tick y leer estado ya no son endpoints abiertos. Las rutas sin prefijo se retiraron sin dejar alias de compatibilidad — un alias abierto habría mantenido el agujero que C3 venía a cerrar (hay un test que verifica el 404)
- [x] `acceso/rolesDePartida.ts`: política de superficies como negocio testable sin HTTP — `puedeAdministrar`, `puedeJugar`, `puedeCrearPartida`, `puedeDescartarPartida`. Un `moderador` NO puede descartar una partida (doc 5: "sin acceso a balance/regeneración de mundo"); un administrador NO puede jugar por serlo
- [x] **`Membresia.hasta` por fin se honra** (`esVigente`): el campo existía desde A6 y hasta ahora ninguna comprobación lo miraba, así que revocar una membresía no revocaba nada
- [x] `server/identidad/administradoresGlobales.ts`: quién administra la instancia, configurado por identidad externa (`proveedor:sujetoId`) vía `ADMINISTRADORES`. **Vacío por defecto** — sin configurarlo nadie puede crear partidas; conceder administración por omisión es la clase de default que llega a producción sin que nadie lo vea. Una config escrita pero ilegible falla al arrancar, no con un 403 tardío
- [x] Quien crea una partida recibe `Membresia` de `administrador_partida` sobre ella: sin eso, un administrador global no podría ejecutar ni los comandos que la matriz le reserva, porque esa matriz razona sobre roles DE PARTIDA
- [x] `api.ts` pasa a ser raíz de composición (66 líneas); las rutas viven en `server/rutas/` (`sesiones`, `admin`, `jugador`, `comandos` compartido, `contexto`) y el registro de partidas abiertas se extrajo a `server/registroDePartidas.ts`
- [x] `GET /sesiones/actual?gameId=` informa del rol en esa partida — para que un cliente sepa qué superficie tiene sin ir coleccionando 403
- [x] **Cliente actualizado** (`cliente/`): habla `/admin/*` (es lo que siempre hizo: crea partidas, avanza tick, lee estado completo), con login de desarrollo y sesión en memoria. No es desarrollo de cliente, es repuntar el wrapper para que el repo no quede roto
- [x] 532/532 tests, `tsc` limpio en ambos proyectos, y verificado en vivo sobre HTTP real: 401 sin sesión, 403 sin rol, 403 al jugador que intenta leer el estado de admin, 403 al admin que intenta jugar, y el cliente renderizando contra la superficie autenticada

- [ ] **Pendiente**: `Membresia` sigue viviendo en memoria (se pierde al reiniciar el proceso, igual que `Sesion`); no hay endpoint para revocar ni para otorgar `moderador`/`observador`, solo se crean por unirse o por crear partida
- [ ] ~~`POST /partidas` y `POST /partidas/:gameId/tick` SIGUEN sin exigir sesión~~ — cerrado en C3
- [x] ~~`fundarAsentamiento` fabrica ids sintéticos en vez de usar el actor real~~ — corregido 2026-08-26

### Fundador real en `fundarAsentamiento` — completada 2026-08-26

El hueco que dejó abierto C3: `fundarAsentamiento` seguía generando `numJugadores` fundadores ficticios
(`jugador-<faccionId>-<n>`) en vez de usar el actor autenticado. Con la Facción del actor ya derivada de
`Faccion.ciudadanosIds` (revisión negocio/infra), esos ids ficticios dejaban al jugador real sin ninguna
forma de hacerse ciudadano: creaba la Facción, fundaba, y la ciudadanía se la quedaban cinco jugadores que no
existían.

- [x] `session/comandos/fundarAsentamiento.ts`: el fundador es SIEMPRE `ctx.actor` — sin `numJugadores`, sin lista. La gobernanza NPC no se toca (funda con sus propios ids `npc-<faccionId>-<n>`, no pasa por este comando)
- [x] **Fundación grupal (hasta 5 cofundadores) queda diferida a propósito**: el motor la soporta, pero exponerla exigiría un mecanismo de CONSENTIMIENTO que hoy no existe — sin él, un cliente podría meter a cualquier jugador en una Facción sin que lo pidiera, y como solo se pertenece a una (Doc 0), dejarlo bloqueado para la que quería. Es una vía de acoso, no una función; documentado en el propio comando
- [x] Autorización (`comandos/autorizacion.ts`): nueva `puedeFundarEn` — ciudadano de la Facción, **o** caso de arranque (Facción sin ningún ciudadano todavía Y actor sin Facción propia). La excepción es estrecha: fundar consume el cap de fundación de la Facción (Doc 1.7), así que no se abre a cualquiera, solo al primer ciudadano de una Facción recién creada — sin ella, `crearFaccion` → `fundarAsentamiento` sería imposible
- [x] Cliente (`cliente/`) repuntado: quitado el campo "Jugadores fundadores" del panel de Mundo y el parámetro del método `gameStore.fundarAsentamiento`
- [x] 534 tests (arreglados 2 que asumían fundación grupal ficticia, añadidos los del caso de arranque y su exclusión), `tsc` limpio en ambos proyectos, y verificado en vivo sobre HTTP real: `jugadoresFundadoresIds`/`ciudadanosIds` quedan con el `usuarioId` real de quien funda; un segundo jugador sin Facción es rechazado (`condicion_dominio`) en cuanto la Facción deja de estar vacía. "Escuadrones propios del jugador o de otros residentes autorizados" (doc 5, fila de combate) sigue sin comprobarse: la condición implementada solo exige residencia en el asentamiento atacante, no la propiedad de cada `escuadronId` — pendiente

### C4 Slice 1. Proyección de jugador sin niebla de guerra — completada 2026-08-26

Alcance decidido explícitamente con el usuario: la niebla de guerra completa (`ConocimientoJugador`, las 3
fuentes de visibilidad) necesita un **radio de visualización** — número de BALANCE, no de arquitectura — que
no está definido en ningún doc de este repo (las referencias "Doc 1.2", "Doc 2.5" etc. apuntan a un documento
de diseño externo). Inventar ese número habría sido una decisión de diseño de juego disfrazada de código, así
que se separó en dos: esta pasada resuelve la fuga de seguridad (nunca `GameState` completo a un jugador) con
una regla conservadora; la niebla de guerra queda como Slice 2, con sus parámetros pendientes de definir.

- [x] `session/proyecciones/jugador.ts`: `proyectarParaJugador(estado, jugadorId)` — Facción propia COMPLETA (asentamientos, escuadrones, colas, almacén); las demás Facciones no aportan ni un asentamiento, ni resumido. Deliberadamente conservador: mejor "no ves nada del rival" que exponer un nivel de detalle que nadie ha decidido que sea seguro
- [x] Público sin filtrar, por no ser información táctica: `facciones` (nombre/nivel/reputación/Rey/Embajador — necesario para que la pantalla de diplomacia tenga con qué pintarse), `relaciones`, `titulos` (un ranking que no se puede ver no sirve como ranking), `caminos`, `campamentosBandidos` (entidades del mundo, no de ninguna Facción), `mapa`/`estadoMapa` (geografía, no secreta — lo que se filtra son las entidades sobre el mapa)
- [x] Filtrados por `asentamientoId` propio: `caravanas` (origen o destino), `acuerdos`, `ordenes`, `eventosDominio` (sin `asentamientoId` = global, o uno propio — mismo criterio que evita la fuga que el doc 7 §7.1 señalaba en el log administrativo)
- [x] `GET /jugador/partidas/:gameId` (Fase C3 lo había dejado explícitamente sin implementar): exige rol `jugador` — un administrador sigue sin poder leerlo, aunque administre esa misma partida
- [x] 20 tests nuevos (13 unitarios con estado de partida genuino, 5 HTTP), 552/552 en total, `tsc` limpio en ambos proyectos. Verificado en vivo sobre HTTP real: un jugador sin Facción ve `facciones` pero `asentamientos: []`; el fundador ve el suyo; un admin recibe 403
- [ ] **Slice 2, pendiente y con parámetros por definir**: `ConocimientoJugador` (entidad qué-sabe-cada-jugador-desde-cuándo), visibilidad espacial (zona de influencia + radio de visualización — falta el número), contacto (trueque, mientras siga vivo — falta decidir si decae o se congela indefinidamente) y alianza (visibilidad en vivo de aliados). La decisión de fondo ya está tomada (2026-08-24): se muestra el ÚLTIMO ESTADO CONOCIDO, no el actual — patrón de niebla de guerra de RTS, evita filtrar telemetría en vivo de rivales
- [x] ~~Construir frontend de jugador (acciones y datos restringidos a su proyección)~~ — **fuera de alcance de este repositorio** desde el replanteo de 2026-08-25: el cliente de jugador vive en otro repositorio. Lo que sí es responsabilidad de aquí es que su proyección exista y esté documentada (tareas de proyecciones y de contrato)

### C5. WebSocket, canales e idempotencia — completada 2026-08-26

- [x] **Idempotencia de comandos** (`RunnerDePartida.ejecutar`, parámetro `idempotencyKey` opcional): una clave repetida por el mismo actor —ya esté el primer intento en curso o ya haya resuelto— devuelve el MISMO resultado sin volver a aplicar el comando ni subir la versión. Se guarda la PROMESA, no solo el resultado: cubre con un solo mecanismo tanto el reintento mientras el primero sigue en cola como el reintento después de resuelto. Si la persistencia falla (excepción), la clave NO queda cacheada — un reintento legítimo puede volver a intentarlo. No compara `manejador`/`params` entre usos de la misma clave (confía en que el cliente no la reutiliza para comandos distintos): compararlo exigiría igualdad profunda, y `JSON.stringify` da falsos positivos por orden de claves
- [x] `EjecutarComandoBody.idempotencyKey` (opcional) en `POST .../comandos`, en ambas superficies
- [x] `session/canales.ts` (negocio, no infraestructura — mismo motivo que `comandos/autorizacion.ts`): `CANAL_GENERAL` (`mapa/general`, eventos sin `asentamientoId`, abierto a cualquier jugador de la partida) y `asentamiento/<id>` (solo si es de la Facción propia — MISMA regla que `proyectarParaJugador`, Slice 1 de C4: nada de lo ajeno, ni en tiempo real)
- [x] `server/difusion/hub.ts` (infraestructura): registro de conexiones abiertas por partida y `difundir(gameId, eventos)` — un `JSON.stringify` por evento, no uno por conexión, pensando en los ~500 jugadores/partida del objetivo de escala
- [x] `GET /jugador/partidas/:gameId/tiempo-real` (`@fastify/websocket`): autenticación en `preValidation`, ANTES de completar el *handshake* — sin sesión ni membresía de jugador, el rechazo llega como 401/403 normal a la petición de upgrade, no como un socket que se abre y se cierra solo. Protocolo: cliente manda `{accion, canal}`; servidor responde `{tipo: suscrito|desuscrito|error}` y empuja `{tipo: evento, canal, evento}`. Un administrador de la partida NO puede conectar (para jugar hace falta ser jugador, doc 5)
- [x] **Sin mensaje sintético de "conectado"**: el `open` nativo de WebSocket ya lo dice, y como la autenticación ocurre en `preValidation` antes del *handshake*, para cuando `open` dispara la conexión ya está autenticada. Se retiró tras un hallazgo real depurando los tests: mandarlo SÍNCRONAMENTE en el mismo tick en que arranca el handler compite con que el cliente termine de engancharse al evento `message` y se pierde — una carrera real del transporte (confirmada también con un cliente `ws` real sobre TCP real, no solo con el arnés de pruebas), no una peculiaridad de `injectWS`. Evitar el envío por completo es más simple y más robusto que retrasarlo con un `setImmediate`
- [x] Al reconectar se pierden las suscripciones (doc 6 §2): no hay estado de suscripción que sobreviva al cierre del socket — el cliente se re-suscribe solo. Deliberado: más simple que reconstruir "qué tenía suscrito", y coherente con que las suscripciones describen QUÉ se quiere ver, no un historial que recuperar
- [x] 25 tests nuevos (11 idempotencia en `RunnerDePartida`, 8 `canales.ts`, 14 WebSocket con `injectWS`, 1 idempotencia HTTP end-to-end), 581/581 en total, `tsc` limpio en ambos proyectos. Verificado en vivo con servidor real (`:3000`) y cliente `ws` real (no `injectWS`): conexión, suscripción y evento difundido, de punta a punta
- [ ] **Pendiente**: los comandos siguen yendo por HTTP, no por el WebSocket (doc 6 §2 ya lo decidía así: "HTTP: comandos; WebSocket: solo notificar cambios") — nada que resolver aquí, es el diseño. Sin métricas de conexiones activas expuestas todavía (`hub.conexionesAbiertas` existe pero no hay endpoint que la lea) — llega con Fase E3

### C6. Contrato publicable: versionado, CORS, OpenAPI, respuesta autosuficiente — completada 2026-08-26

- [x] **Versionado por prefijo de ruta**: todo bajo `/v1` (`app.register(async (v1) => {...}, {prefix: '/v1'})` en `api.ts`). Se eligió prefijo y no cabecera porque cualquier cliente HTTP lo soporta sin configuración especial, se ve en cualquier log de acceso, y es lo que ya asume `servers` del propio `openapi.json`. Sin alias de compatibilidad en las rutas viejas sin `/v1` — mismo criterio que C3 con las rutas sin superficie: un alias abierto habría mantenido vivo lo que esto viene a cerrar. Cliente (`cliente/`) y su proxy de Vite repuntados
- [x] **CORS** (`@fastify/cors`), configurado por `ORIGENES_PERMITIDOS` (lista separada por comas). **Vacío por defecto** — mismo criterio que `ADMINISTRADORES`: sin configurarlo, `origin: false` desactiva CORS del todo, ningún origen cruzado pasa. Un array vacío se traduce a `false` explícitamente, no se deja a como `@fastify/cors` interprete `[]` por su cuenta
- [x] **OpenAPI** (`@fastify/swagger`) publicado sin autenticar en `GET /v1/openapi.json` — es lo primero que un cliente nuevo necesita leer, antes de poder hacer login. Esquemas de request/response añadidos a las 8 rutas HTTP reales (antes solo 2 tenían `schema.body`); dos securitySchemes `apiKey` sobre la misma cabecera `Authorization` (`sesionAuth` para el resto de peticiones, `credencialProveedor` para el login) — OpenAPI no puede expresar "el esquema depende del prefijo del valor", así que es lo más preciso declarable sin inventar una convención que el servidor no sigue. El WebSocket de `/tiempo-real` no aparece: OpenAPI 3.0 no describe WebSocket
- [x] **Sin `schema.response` para los cuerpos grandes o de forma variable** (estado completo de administrador, proyección de jugador, `resultado.datos` que cambia según el comando) — decisión de seguridad, no de pereza: el `response` de Fastify no es solo documentación, es un FILTRO DE SERIALIZACIÓN (`fast-json-stringify`) — un campo real ausente del schema se DESCARTA de la respuesta en caliente. Modelar esos cuerpos con un schema aproximado arriesgaba romper payloads de verdad en silencio; se prefirió dejarlos sin cuerpo de respuesta documentado (la ruta sigue apareciendo con método, parámetros y seguridad) antes que correr ese riesgo. Verificado tras cada ruta añadida: la suite entera sigue en verde, ningún campo real desapareció
- [x] **Respuesta de comando autosuficiente, solo en `/jugador/*`**: la respuesta de `POST .../comandos` ahora incluye `proyeccion` (el resultado de `proyectarParaJugador` sobre el estado YA actualizado) — el cliente deja de necesitar el viaje aparte que hacía antes. `ejecutarComandoHttp` (compartido con `/admin/*`) gana un parámetro opcional `camposExtra` en vez de bifurcarse; `/admin/*` no lo usa porque su `GET` de estado completo ya es barato de pedir aparte, y adjuntarlo ahí repetiría el problema de tamaño que esto viene a evitar
- [x] 11 tests nuevos (3 CORS, 6 OpenAPI, 2 respuesta autosuficiente), 592/592 en total, `tsc` limpio en ambos proyectos. Verificado en vivo: `/sesiones` sin `/v1` da 404, `/v1/sesiones` funciona; `openapi.json` describe las 9 rutas reales; el cliente de administración funcionando de punta a punta contra `/v1/*` a través del proxy de Vite actualizado
- [x] Servir el balance — hecho en C7, ver abajo. La parte "versionado por partida/temporada, con auditoría de cambios" queda sin dueño (mismo estado que `actualizarBalance`/`restaurarBalance` desde que se retiraron en Fase B — ver arriba, "De 34 a 31")
- [x] ~~Separar rutas/endpoints de administración de las de jugador, protegidas por rol técnico~~ — hecho en C3 (`/admin/*` y `/jugador/*`)

### C7. Balance servido (2026-08-26)

- [x] **`GET /v1/balance`** (`server/rutas/balance.ts`), registrado en `api.ts` junto a `openapi.json` — sin
  autenticar, mismo criterio: es regla pública (T1, doc 9), no estado de partida, y es lo primero que un
  cliente nuevo puede necesitar antes incluso de hacer login. Devuelve las **39 tablas completas** de
  `constants.ts` agrupadas por el mismo criterio que la tabla "por consumidor de interfaz" del doc 9
  (`catalogos`, `cuposYNiveles`, `costesYEconomia`, `geometriaUrbana`, `mundoYMilitar`, `caravanas`,
  `reputacion`, `internas`), más `version`. Sin `schema.response`: modelar 39 tablas en JSON Schema es una
  segunda fuente de verdad que mantener a mano, mismo motivo que ya usa `esquemas.ts` para los cuerpos grandes
- [x] **Decisión del usuario sobre las 7 tablas que el doc 9 marcaba "a revisar antes de publicarlas"**
  (`CAMPAMENTOS_BANDIDOS`, `REGENERACION_NODOS`, `SCORE_BANDAS`, `EXTRACTOR_DESEMPATE`, `LINEAS_PRODUCCION`,
  `EXTRACCION_MAXIMOS`, `SIMULACION_AUTO_COMERCIO`): **publicarlas todas**, sin lista de exclusión que
  mantener — quedan agrupadas bajo `internas` para que no se confundan con balance de formulario, pero viajan
  igual que el resto. Doc 9 actualizado
- [x] **`BALANCE_VERSION`** (`constants.ts`, primer valor `1`) se estampa en `PartidaExportada.balanceVersion`
  al exportar (`GameSession.exportar`), como registro de qué balance corría al crear/guardar la partida. A
  diferencia de `WORLDGEN_VERSION`, un desajuste **no se rechaza** al cargar (`persistenciaPartida.ts` no
  gana una nueva excepción): el balance no hace falta para reconstruir el snapshot ya guardado, solo cambia
  qué reglas rigen los próximos comandos y ticks — documentado en el propio comentario de `BALANCE_VERSION`
- [x] **Alcance NO cubierto, a propósito** (evitar sobre-construir sin un consumidor real): "por
  partida/temporada" con overrides de verdad. Hoy sigue siendo un único valor de proceso — no hay mecanismo
  para que dos partidas abiertas a la vez corran versiones de balance distintas. El panel que mutaba
  `constants.ts` en caliente (`app/balanceConfig.ts`) sigue sin dueño desde que se retiró en Fase B
- [x] 4 tests nuevos (`server/__tests__/balance.test.ts`: 200 sin sesión, tabla real sin transformar,
  presencia en el OpenAPI sin `security`; `session/__tests__/gameSession.test.ts`: `balanceVersion` estampada
  al exportar). 604/604 en total, `tsc` limpio. Verificado en vivo: servidor real levantado con
  `ADMINISTRADORES='dev:jefa'`, `curl http://localhost:.../v1/balance` responde 200 con `version:1` y
  `EDIFICIO_CATALOGO` real dentro de `catalogos`
- [ ] Absorbe el grupo "de tabla" del hito C10 (`capFundacion`, `cupoVivienda`, `slotsPoliticaDisponibles`,
  `nivelFaccionInfo`, `CATALOGOS`): con el balance servido, un cliente sin motor ya puede resolverlas por
  *lookup* — pero eso lo hace el cliente cuando exista uno sin `@motor/*` (C8-C13), no este hito

### Diagnóstico de aislamiento del cliente (2026-08-26)

Pregunta que lo abrió: *"si me llevo el cliente de administración a otro repo, ¿funciona?"* No, y las razones
resultaron ser bastante peores que la que C0 había anotado. **Decisión del usuario tras el diagnóstico: el
cliente no debe depender del motor de ninguna forma — solo puede hablar con el backend por red.** Eso deja
obsoletas las tres opciones que `cliente/README.md` recomendaba (copiar el motor, submódulo, paquete npm) y
convierte `@motor/*` en un defecto a eliminar, no en una costura a repuntar.

**Premisa falsa que arrastraba C0.** El segundo bullet de C0 dice que al sacar la carpeta "se repunta ESE
alias". Es cierto mecánicamente y falso en sustancia: el alias existe porque el cliente **calcula lo que el
servidor no expone**. Repuntarlo mueve el motor de sitio, no lo elimina.

#### Hallazgo 1 — el cliente de administración ya está roto, aquí, sin moverlo

Verificado ejecutando el servidor real (`crearServidor` + `inject`) con `ADMINISTRADORES='dev:jefa'`:

```text
crear partida     -> 201
tick              -> 200
comando crearFaccion         -> 403 {"error":"no autorizado (rol_insuficiente)"}
comando fundarAsentamiento   -> 403 {"error":"no autorizado (rol_insuficiente)"}
comando crearCaravana        -> 403 {"error":"no autorizado (rol_insuficiente)"}
comando alternarFaccionNpc   -> 403 {"error":"no autorizado (rol_insuficiente)"}
```

`apiCliente.ejecutarComando` manda a `/v1/admin/partidas/:gameId/comandos`. Ahí el actor entra con
`rolEnPartida(actor)`, que **cortocircuita a `'administrador_global'` antes de mirar la `Membresia`**
(`acceso/rolesDePartida.ts:36`) — y ese rol no figura en `rolesPermitidos` de ninguna de las 30 filas de
`MATRIZ_AUTORIZACION`. Los ~30 botones de acción de la interfaz mueren en 403 desde C2/C3.

El comentario de `rutas/admin.ts` ("hoy solo `alternarFaccionNpc` admite administración") es **falso**: esa
fila permite `administrador_partida`, no `administrador_global`, y quien crea la partida siempre es global.

No se detectó en la verificación en vivo de C3/C6 porque solo se ejercitaron crear mundo, tick y render —
las dos únicas rutas que sí pasan. Fallo de cobertura de la verificación, no del diseño: que un rol técnico
no conceda autoridad de juego es exactamente la regla del doc 5. Lo que nunca se decidió es **qué superficie
habla este cliente**: `gameStore.ts` se declara "cliente de JUGADOR" en su cabecera y `apiCliente.ts` habla
la superficie de administración. → hito **C8**.

> **De las cuatro filas de la tabla, tres siguen 403 hoy — correctamente**: `crearFaccion`, `fundarAsentamiento`
> y `crearCaravana` son de rol `jugador` puro, y ya no están expuestas en la interfaz de administración (C8,
> UI corregida). La cuarta, `alternarFaccionNpc`, **ya no da 403** desde el fix de `rolEnPartida` del
> 2026-08-26 — era el bug real de esta fila, no las otras tres. El comentario de `rutas/admin.ts` citado abajo
> es ahora **cierto**, no falso.

#### Hallazgo 2 — no es un acoplamiento, son cuatro, y cada uno se rompe distinto

| # | Acoplamiento | Qué es | Sale con |
|---|---|---|---|
| 1 | **Tipos** (`domain/types`, `GameSessionState`, `ParamsDe`/`DatosDe`, `ResultadoComando`, `EventoDominio`) | Solo compilación, coste cero en ejecución | Cliente generado del OpenAPI — **bloqueado**: ver hallazgo 3 |
| 2 | **Balance y catálogos** (8 módulos de `constants`) | Alimentan `CATALOGOS` (todos los formularios) y los cálculos de coste | **C7 hecho** (`GET /v1/balance`) — sale cuando `cliente/` deje de importar `constants` y lea el endpoint en su lugar |
| 3 | **26 consultas derivadas** | No existen en el servidor; solo dentro de `gameStore.ts` | **C10** (migración del doc 8) |
| 4 | **El terreno** | `MapaGenerado.elevacion`/`.fertilidad` son *parámetros de ruido*, no rásteres | **C11** |

#### Hallazgo 3 — el contrato publicado tiene el agujero justo donde el cliente lo necesita

`ESQUEMA_EJECUTAR_COMANDO` (`server/rutas/comandos.ts`) declara `params: {}`. **30 comandos, cero descritos.**
Hoy `ParamsDe<T>` se deriva de `Parameters<typeof manejador>` — es TypeScript leyendo el código fuente del
servidor, que es precisamente la dependencia a eliminar. Sin esquema por comando no hay cliente tipado
generable desde `openapi.json`, y además un `params` malformado revienta dentro del manejador y sale como 409
en vez de 400 (ya anotado como pendiente en C2, sin dueño hasta ahora). → hito **C9**, resuelto el mismo día
que este diagnóstico (ver sección "C9" más abajo): `ParamsDe<T>` sigue derivándose del código fuente para
TypeScript, pero el CLIENTE ya no lo necesita — `ESQUEMAS_PARAMS` (`session/comandos/esquemas.ts`) es la
misma forma expresada como datos, publicada en `openapi.json`.

#### Hallazgo 4 — la mitad de las 26 consultas no puede ser una petición HTTP

`render()` se dispara en **cada `mousemove`** sobre el lienzo (`cliente/src/main.ts:2512`). Dentro llama a
`getMapa()`, `getZonasFusionadas`, `chokepointsControl`, `getTrazadoAsentamiento` y `viabilidadFundacion`.

> **Corregido 2026-08-26**: `chokepointsControl` y `viabilidadFundacion` se eliminaron ese día (ver "C10
> partido en dos" más abajo) en vez de migrarse. Como consecuencia, el propio `mousemove` que las disparaba ya
> no llama a `render()` en la vista de mundo — no queda nada ahí que recalcular en cada movimiento del ratón.

- Las cuatro primeras solo cambian por tick → se resuelven mandando la geometría **ya calculada** dentro de
  la proyección, no con un endpoint por consulta.
- `viabilidadFundacion(hover)` es la difícil: función continua de un punto arbitrario. O rejilla de
  viabilidad precalculada, o el *preview* deja de ser *hover* y pasa a clic. Es una decisión de diseño de
  interacción, no de arquitectura — no se toma aquí.

El resto (`mantenimientoInfo`, `poblacionInfo`, `produccionInfo`, `caravanasInfo`, `manoObraInfo`, los
`nivel*Info`, los cupos, `precioReferencia`, `poderMilitarInfo`, `infoMejoraEdificio`) cambia por tick y cabe
en la proyección sin más.

> **Corregido 2026-08-26**: `getLigas` salió de esta lista — estaba mal puesta aquí. `computeLigas` es T2a en
> el doc 9 (`relaciones` + `facciones`, ambos ya públicos en la proyección): el cliente la calcula localmente
> y **ya lo hace hoy** (`gameStore.getLigas`). Ponerla en el grupo (b) habría significado mandarla calculada
> desde el servidor sin necesidad — el mismo tipo de sobre-ingeniería que el resto de este documento evita a
> propósito. Detectado auditando qué corre hoy en `cliente/` de verdad, no reclasificando de memoria.

#### Hallazgo 5 — el terreno que el servidor manda es indibujable sin `worldgen/`

`MapaGenerado.elevacion` es un `CampoElevacion` (parámetros de ruido + región) y `.fertilidad` un
`CampoRuido`; el bioma **no se guarda**, se deriva bajo demanda (`worldgen/types.ts:39-61`). `drawTerreno`
evalúa `biomaEn`/`elevacionEn` **por píxel** llamando a `evaluarBioma`/`evaluarElevacion`.

Es la forma más aguda del acoplamiento: el servidor ya manda estos datos y son **inútiles sin el código que
los interpreta**. Salidas: el servidor rasteriza (PNG o tiles — determinista por seed, así que se cachea para
siempre), o los evaluadores de `worldgen/` se publican como librería, que es justo la dependencia a eliminar.
→ hito **C11**.

#### Hallazgo 6 — huecos de operación que nadie había listado

- **No hay endpoint para listar partidas** (ni `/admin` ni `/jugador`). Un cliente externo no puede descubrir
  a qué conectarse; el `gameId` llega fuera de banda.
- **No hay fuente de ticks**: solo `POST /admin/.../tick`, sin scheduler. Un cliente conectado a un
  despliegue real se queda en un mundo congelado salvo que alguien lo empuje a mano.
- **No hay endpoints de exportación**: `exportarSimulacion` y `exportarMapaUnity` corren hoy en el navegador
  usando `world/exportUnity` y `session/estado.proyectarLog`. El doc 8 ya los clasificaba como administración.
- → hito **C12**.

#### Hallazgo 7 — C5 y C6 dejaron dos costuras de eficiencia que solo se ven con un cliente externo

> **Corregido y medido 2026-08-26** tras la pregunta del usuario ("¿está viajando el mapa completo en cada
> consulta?"). La respuesta es sí, pero la primera versión de este hallazgo apuntaba al culpable equivocado.
> Medición completa y sus consecuencias en
> [6_Sincronizacion_Visibilidad_y_Escala.md §6.4](6_Sincronizacion_Visibilidad_y_Escala.md#64-medición-qué-viaja-hoy-de-verdad).

- **El mapa viaja entero en cada acción** (125,4 KB), y desde C6 cada comando de jugador devuelve una
  proyección. Pero medirlo cambió el diagnóstico: esos 125,4 KB son **idénticos byte a byte** en el tick 0, el
  50 y el 200 — el mapa se deriva de la seed y no cambia jamás. **No es estado, es un asset.** No necesita
  `ETag`/deltas: necesita servirse una vez, cacheado por `seed`+`worldgenVersion` con `Cache-Control:
  immutable`. Eso lo saca de la proyección del todo (−78%) y cae dentro de **C11**, que se abarata mucho
- **`eventosDominio` es el problema de verdad**: 2,0 KB → 41,0 KB → 102,0 KB entre el tick 0 y el 200, con
  solo 4 facciones, y viaja entero cada vez. Crece sin techo. A tick 1000 con 500 asentamientos es el
  verdadero cuello. Se corta con un cursor `?desde=<version>` — que además es para lo que el WebSocket de C5
  ya existe
- **El WebSocket difunde `EventoDominio`, no deltas de estado.** Un cliente sin motor no puede aplicarlos a
  su proyección cacheada, así que la única reacción correcta a cualquier evento es *refetch* completo. C5
  entrega avisos, no datos
- → hito **C13**, con el objetivo corregido: el enemigo era el log de eventos, no el mapa.

#### Hallazgo 8 — la frontera correcta no es "motor sí / motor no" (2026-08-26)

Investigados los patrones de la industria a petición del usuario, que dudaba de si su premisa ("el cliente no
debe tener el motor") venía de una mentalidad de aplicaciones web mal trasladada a videojuegos. Análisis
completo, con fuentes, en
[6_Sincronizacion_Visibilidad_y_Escala.md §6](6_Sincronizacion_Visibilidad_y_Escala.md#6-modelo-de-sincronización-reglas-al-cliente-simulación-en-el-servidor).
Resumen de lo que afecta al plan:

- **Lockstep determinista queda descartado** (el patrón de AoE/StarCraft: solo viajan comandos, cada cliente
  simula el mundo entero). Es **estructuralmente incompatible con la niebla de guerra** —de ahí los maphacks
  de StarCraft— y eso choca con C4, donde las proyecciones son frontera de seguridad. Segundo motivo: existe
  para esconder latencia a 60 Hz, y aquí el tick mide ~1,9 s de CPU. Se confirma el modelo que C4/C5 ya
  construyen: servidor autoritativo, cliente sin simulación
- **Pero la premisa sí necesitaba un matiz**: la frontera es **reglas vs. simulación**, no "motor sí/no". La
  simulación es solo del servidor; las **reglas** (costes, cupos, validez) el cliente las necesita para
  responder al instante, y la industria se las manda **como datos, no como código** (el *Static Data Export*
  de EVE Online). Eso es C7, y ya está medio hecho: `EDIFICIO_CATALOGO`, `POLITICAS` y `NIVEL_FACCION` son
  tablas
- **Consecuencia: C10 se parte en dos** (ver abajo)

#### C10 partido en dos: qué sirve el servidor y qué calcula el cliente

Consecuencia directa del hallazgo 8. Las 26 consultas no son homogéneas:

**(a) Tabla — las resuelve C7, no C10.** `CATALOGOS`, `capFundacion`, `cupoVivienda`,
`slotsPoliticaDisponibles`, `nivelFaccionInfo`. Son *lookup* sobre constantes de balance. Sirviendo el balance
(C7), el cliente las resuelve sin reimplementar nada y sin viaje de red.

**(b) Fórmula sobre estado vivo — DISUELTO por el doc 9 (corregido 2026-08-26, ver nota abajo).** Esta lista
original —`produccionInfo`, `mantenimientoInfo`, `manoObraInfo`, `poblacionInfo`, `infoMejoraEdificio`,
`caravanasInfo`, `cupoNivelInfo`, `nivelAsentamientoInfo`, `poderMilitarInfo`— se escribió el mismo día que el
hallazgo 8, pero ANTES de [9_Reglas_vs_Simulacion.md](9_Reglas_vs_Simulacion.md), que es más preciso: casi
todas resultan T2a (entrada propia — `manoObraInfo`, `estadoMejoraEdificio`, `calcularCostoMantenimiento`,
`poderEscuadron`, `nivelActualDe`, `cupoCaravanas`, `poblacionTotal`... todas sobre **un** asentamiento/
escuadrón/Facción **propios** — doc 9, tabla T2a). El cliente las resuelve solas en cuanto tenga el balance
(C7), sin que el servidor tenga que calcular ni mandar nada nuevo — **cero trabajo de C10 aquí**. El roadmap
(doc 3) nunca contó este grupo como pendiente de C10, y tenía razón; esta sección de doc 4 sí lo hacía y
quedó sin corregir hasta ahora. (`getLigas` tampoco va aquí — es T2a, ver la nota del hallazgo 4.)

> **Matiz que SÍ sigue abierto, pero no es de C10**: `mantenimientoInfo`/`poblacionInfo`/`caravanasInfo`/
> `infoMejoraEdificio` aplicados a un asentamiento RIVAL (no propio) sí necesitarían proyección filtrada por
> visibilidad — es la clasificación "Proyección por audiencia (5)" del triaje original del doc 8. Bloqueado en
> lo mismo que siempre: **C4 Slice 2** (niebla de guerra), sin radio de visualización definido en ningún doc.
> No es un hueco de C10 — es un hueco de C4 que C10 no puede cerrar por su cuenta.

> **`precioReferencia` — hecho 2026-08-26.** Primera de este grupo en moverse: `RunnerDePartida.preciosReferencia()`,
> caché con TTL de un minuto real (no un `setInterval` — se recalcula perezosamente en la siguiente lectura
> tras vencer el TTL, mismo contrato observable sin sumar un temporizador que limpiar por partida). Expuesta
> en `EstadoAdmin`/`ProyeccionJugador`. El cliente dejó de importar `@motor/engine/market` — verificado en
> vivo, el precio mostrado en pantalla coincide con el cálculo del servidor. 8 tests nuevos (5 unitarios de
> TTL/caché, 3 HTTP en las tres superficies de lectura). 613/613 en total.

**(c) Geometría por frame — viaja precalculada dentro de la proyección (C10). Hecho, 2026-08-26.** `getZonas`,
`getZonasFusionadas`, `getTrazadoAsentamiento`. Solo cambian por tick, pero `render()` las pide en cada
`mousemove`: no pueden ser un endpoint, tienen que llegar ya resueltas — `RunnerDePartida.geometriaAsentamientos()`,
ver la sección "C10" más abajo para el detalle completo.

> **`chokepointsControl` sale de este grupo — eliminada, no migrada (2026-08-26).** Decisión del usuario: la
> mecánica de chokepoints entera (geometría en `worldgen/`, control por zona de influencia, peaje en oro) "no
> me está dando nada en este momento". `WORLDGEN_VERSION` sube a 15; el pipeline termina un paso antes
> (chokepoints era el último), así que bosques/nodos/ríos/elevación salen bit a bit idénticos a v14 para la
> misma seed — solo se regeneró el snapshot de caracterización, verificado que el diff es exclusivamente las
> líneas de chokepoints. Alcance completo: `worldgen/chokepoints.ts`, `engine/chokepoints.ts`, el bloque de
> peaje dentro de `engine/trade.ts` (`avanzarComercio`/`avanzarCaravanas` pierden el parámetro `zonas`, que ya
> no necesitan para nada más), `CHOKEPOINTS_PEAJE`, el tipo de dominio `Chokepoint`, `Mapa.listarChokepoints`/
> `chokepointMasCercano`, y el renderizado en `canvas.ts` (la capa cacheada de `drawTerreno` y el anillo de
> control en `draw()`). 600/600 tests tras regenerar el snapshot; verificado en vivo que el mapa renderiza sin
> los rombos ámbar de antes.

**(d) `viabilidadFundacion(punto)` — retirada del cliente, no resuelta (2026-08-26).** Era la difícil por ser
función continua de un punto arbitrario que el usuario mueve con el ratón — la decisión de diseño de
interacción (rejilla precalculada vs. pasar a clic) seguía sin tomarse. Decisión del usuario tras la
auditoría: "era un helper que ahora está causando más problemas que otra cosa", fuera del cliente. La función
en sí **no desapareció** — `session/npcGobernanza.ts` la sigue usando en vivo para decidir dónde funda un NPC
sin jugador (evita emplazamientos sin bosque cercano) — pero se retiró el *preview* de *hover* completo:
`gameStore.viabilidadFundacion`, el div `#fundacion-viabilidad`, `drawPreviewFundacion` (canvas.ts, sin otro
consumidor), y el propio *listener* de `mousemove` que solo existía para refrescarlo — al no quedar nada que
recalcular en la vista de mundo, `render()` deja de dispararse en cada movimiento del ratón ahí.

**Coste aceptado en (a)**: la fórmula acaba existiendo dos veces (servidor por autoridad, cliente por
presentación) y pueden divergir. Se acepta a conciencia —la alternativa es un viaje de red por *tooltip*, que
es inusable— y se mitiga manteniendo esas reglas como tabla pura, para que el cliente haga *lookup* en vez de
reimplementar lógica. Es el mismo trato que hace la industria.

> **La clasificación completa de todo el motor —qué es regla, qué es simulación, y cuál de las dos cosas puede
> salir del servidor— está en [9_Reglas_vs_Simulacion.md](9_Reglas_vs_Simulacion.md)** (escrito 2026-08-26 a
> petición del usuario, como referencia para futuras decisiones). Añade un eje que este desglose no tenía y
> que corrige el grupo (c) de arriba: las consultas de geometría por frame no son solo un problema de
> latencia, son además **entrada privilegiada** (`computeTodasLasZonas` mira todos los asentamientos). No
> pueden ser un endpoint *y* no pueden calcularse en el cliente: por eso tienen que viajar precalculadas
> dentro de la proyección. (De las cuatro originales, `chokepointsControl` y `viabilidadFundacion` se
> eliminaron el mismo día en vez de migrarse — ver las notas de (c)/(d) arriba.)

#### Eliminado: el laboratorio visual

`cliente/laboratorio.html` + `cliente/src/lab/` (3 ficheros, 822 líneas, 13 imports de motor) ejecutaban
`generarMapa`, `avanzarSimulacion` y `fundarAsentamiento` **directamente en el navegador, sin servidor**. No
era un cliente de la API y no podía llegar a serlo, así que aislarlo por red es imposible por definición y no
podía acompañar a `cliente/` a otro repositorio.

Se elimina (decisión del usuario, 2026-08-26). Era una herramienta útil —varios comentarios de
`engine/trazado.ts` y `engine/construction.ts` citan bugs reales detectados con ella— y queda en el historial
de git por si conviene rescatarla como herramienta de desarrollo de ESTE repo, donde el alias `../src` es
correcto y gratis. `vite build` nunca la incluyó (no hay `rollupOptions.input` multipágina): solo existía en
`vite dev`.

### C11a. El mapa deja de ser estado — completada 2026-08-26

Primer hito ejecutado tras el análisis del modelo de sincronización (doc 6 §6). El mapa se medía como el 78%
de cada proyección y era idéntico byte a byte tick tras tick — es un asset, no estado, y viajaba en cada
comando desde C6 sin necesidad.

- [x] `idDeMapa(mapa): string` (`session/estado.ts`) — función PURA de `mapa.config.seed` + `.region` +
  `mapa.version` (`WORLDGEN_VERSION`). Legible a propósito (`v14-s42-anatolia`), no un hash: se puede leer en
  un log o una URL sin decodificar nada. `ancho`/`alto` no entran en la fórmula porque nunca varían (siempre
  `MAPA_DEFAULT`) — ver el comentario del propio código si eso deja de ser cierto algún día
- [x] `vistaAdminDeEstado(estado): EstadoAdmin` (`session/estado.ts`) — `Omit<GameSessionState, 'mapa'> &
  {mapaId}`. Usada por `GET /admin/partidas/:gameId`, que YA NO manda `mapa`. `GameSessionState` (el estado de
  dominio real) no se toca: sigue llevando `mapa: MapaGenerado` completo, porque el motor y los 30 comandos sí
  lo necesitan de verdad — el recorte es solo en la frontera HTTP
- [x] `ProyeccionJugador.mapa` → `ProyeccionJugador.mapaId` (`session/proyecciones/jugador.ts`)
- [x] `ResumenPartida` (`server/rutas/contexto.ts`) gana `mapaId` — presente en TODA respuesta que ya llevaba
  `resumenDe` (crear partida, tick, comando), así el cliente sabe si su mapa cacheado sigue vigente sin una
  petición aparte. **Se añadió a `RESUMEN_PARTIDA_RESPUESTA` en el mismo cambio** — el `schema.response` de
  Fastify es un filtro de serialización (lección de C6): olvidarlo aquí habría descartado el campo en
  silencio de la respuesta `201` de `POST /admin/partidas`, y el primer run de tests lo confirmó fallando
- [x] `GET /{admin,jugador}/partidas/:gameId/mapa/:mapaId` (`server/rutas/mapa.ts`, nuevo, compartido por las
  dos superficies) — `Cache-Control: private, max-age=31536000, immutable`. El handler **ignora** `:mapaId`:
  no hay historial de mapas que servir, solo el vigente, así que el segmento es puro cache-buster para el
  navegador del cliente — mismo patrón que un asset estático con hash en el nombre de archivo
- [x] Cliente (`cliente/`) actualizado para no romperse — es el único consumidor real hoy. `apiCliente.ts`:
  `consultarEstado` devuelve `EstadoAdmin`, nueva `obtenerMapa(gameId, mapaId)`. `gameStore.ts`: nuevo
  `mapaGeneradoCache` cacheado aparte de `estadoCache` (invariante: nunca `null` tras el constructor, que
  ahora es async y pide el mapa una vez antes de devolver el `GameStore`), y `sincronizarMapa(mapaId)` — solo
  vuelve a pedir el mapa si el id cambió, lo que en la práctica solo pasa tras `regenerarMundo`
- [x] 8 tests HTTP nuevos (`api.test.ts`: sin `mapa` en el estado admin, sirve el mapa real en ambas
  superficies, el id es cache-buster puro, mismo `mapaId` en las dos superficies, 401/403), 5 unitarios
  (`estado.test.ts`, nuevo: estabilidad, invariancia frente al tick, distinción por seed, legibilidad), 2
  esquemas nuevos verificados en `openapi.test.ts`. 605/605 en total, `tsc` limpio en los dos proyectos
- [x] **Verificado en vivo** (servidor + cliente reales, Fase de verificación reforzada tras la lección de C3/C6
  — ver memoria de sesión): `GET .../local` → 200 sin `mapa`; `GET .../local/mapa/v14-s1` → 200 con el mapa
  real; el mapa renderiza en pantalla igual que antes. Tras `POST .../tick`, la petición de red confirma que
  **no** se repite el `GET .../mapa/` — `mapaId` no cambió y `sincronizarMapa` lo reconoció sin red

### C11b. Rasterizar el terreno — descoped, sin implementar

`elevacion`/`fertilidad` en `MapaGenerado` son parámetros de ruido, no rásteres, y el bioma no se guarda: se
evalúa por píxel con `evaluarBioma`. Lo que sale de C11a sigue siendo indibujable sin `worldgen/`.

Se deja fuera de esta pasada, a propósito, por dos razones:

- **Sin consumidor real hoy.** El único cliente que existe (`cliente/`) sigue importando `@motor/*` y dibuja
  con la fachada `Mapa` en el navegador — no necesita un ráster. Un cliente sin motor que sí lo necesite está
  bloqueado detrás de C8-C10 de todas formas
- **Exige una decisión de dependencia nueva** (librería de rasterización a PNG — no hay ninguna en
  `package.json` hoy) que no hay razón para tomar antes de que exista quien la consuma

Queda anotado como pendiente explícito de C11, no como "hecho": lo mismo que costó una premisa falsa en C0
—dar algo por resuelto porque una parte relacionada lo está— no se repite aquí a propósito.

### C8. El administrador observa, no interactúa — interfaz corregida, autorización resuelta (2026-08-26)

Corrección de rumbo del usuario sobre el planteamiento original de C8: la pregunta no era "¿cómo le damos al
administrador permiso de jugador?", era "¿qué hace de verdad un administrador?". La respuesta ya estaba en
[5_Contratos_Identidad_Permisos.md §"Matriz de autorización por comando"](5_Contratos_Identidad_Permisos.md)
(escrita el 2026-08-25, **antes** de implementar nada): de 30 comandos, solo `alternarFaccionNpc` admite rol
técnico. Los 27 restantes — fundar, comprar casa, diplomacia, comercio, combate — son `jugador`, sin
excepción. El 403 nunca fue un bug: era la interfaz de depuración ofreciendo, desde antes de C3, botones para
27 acciones que un administrador nunca debió poder ejecutar.

- [x] **Tab "Acciones" eliminada por completo** (Fundar Facción, Cargos, Ciudadanía, Políticas, Diplomacia,
  Fusión/Anexión) — sus 6 tarjetas eran 100% formularios de acción sin ninguna vista de solo lectura propia;
  lo que mostraban ya vive en las pestañas Facción/Jugadores/Registros
- [x] **Tab "Guerra"**: tarjeta "Combate" eliminada (sin vista propia — los escuadrones ya se ven en
  Asentamientos→Militar); "Reclutamiento" reducida a catálogo de solo consulta (tropa + costo + poder, sin
  jugador/origen/botón de reclutar). Roster de tropas intacto
- [x] **Tab "Comercio"**: tarjeta "Orden de Mercado" eliminada (sin vista propia); "Trueque" y "Flota de
  Caravanas" reducidas a paneles de solo consulta (materiales comerciables, estado de flota) — se quitan
  recurso/cantidad/precio y los botones de enviar. Sub-tab renombrada de "⚒️ Acciones" a "🔍 Detalle"
- [x] **Detalle de asentamiento** (dentro de "Asentamientos", que ya era 95% lectura): se quita el input de
  renombrar, los sliders de reserva protegida (ahora valores de solo lectura), el botón "Mejorar ahora", el
  toggle de auto-construcción, y el control completo de cola (mover/quitar/añadir) — la tabla de cola se queda,
  sin columna de acciones
- [x] `alternarFaccionNpc` (Facción → toggle "Controlada por NPC") es la única acción interactiva que queda en
  toda la interfaz — es la única que la matriz permite a un rol técnico
- [x] Verificado en vivo con un asentamiento real (fundado vía la superficie de `/jugador/*` con una sesión
  aparte, no desde la UI de administración — así se prueba la vista de solo lectura contra datos genuinos, no
  contra un mock): cola de construcción, mejoras, reserva protegida y auto-construcción muestran los datos
  correctos sin ningún control de escritura. 600/600 tests, `tsc` limpio en los dos proyectos
- [x] **Defecto encontrado en la misma verificación, corregido el mismo día**: el toggle de `alternarFaccionNpc`
  —la única acción que debía funcionar— respondía 403 igual que las 27 que se acababan de quitar. Mismo bug
  de fondo del hallazgo original de C8: `rolEnPartida` (`acceso/rolesDePartida.ts`) devolvía
  `'administrador_global'` en cuanto `esAdministradorGlobal` era cierto, sin mirar la `Membresia` real, y la
  fila de `alternarFaccionNpc` en la matriz solo admite `['jugador', 'administrador_partida']`. **Fix**: la
  `Membresia` manda sobre `esAdministradorGlobal`, no al revés — quien crea la partida ya tiene Membresia
  `administrador_partida` (`otorgarAdministracion`), así que ahora cae en esa fila. Dos tests preexistentes
  codificaban el bug como esperado (`acceso/__tests__/rolesDePartida.test.ts`,
  `server/__tests__/api.test.ts`) y se corrigieron junto con el código; test nuevo de camino feliz
  (admin cede una Facción real al NPC, `resultado.ok: true`). Verificado en vivo contra el servidor real.
  606/606 tests, `tsc` limpio

### C9. Esquema de `params` por comando (2026-08-26)

- [x] **`session/comandos/esquemas.ts`**: `ESQUEMAS_PARAMS`, un JSON Schema por cada uno de los 30 comandos,
  tipado `Record<TipoComando, EsquemaJson>` — exhaustividad en compilación, mismo mecanismo que
  `MATRIZ_AUTORIZACION`. Vive en `session/`, no en `server/`: la forma de `params` es parte del contrato de
  negocio del comando, no un detalle de Fastify
- [x] **`server/rutas/comandos.ts`**: `ESQUEMA_EJECUTAR_COMANDO.body` pasa de `{tipo, params: {}}` a
  `{type:'object', oneOf: [30 ramas]}`, una por comando (`properties.tipo: {const: tipo}` + el esquema de
  `ESQUEMAS_PARAMS[tipo]`). ajv rechaza con 400 antes de que `ejecutarComandoHttp` llegue a mirar el cuerpo —
  `esTipoComandoValido` queda como red de seguridad para quien llame a la función sin pasar por Fastify, no
  como el camino real de un cliente HTTP. El `oneOf` aparece en `GET /v1/openapi.json` sin más trabajo:
  `@fastify/swagger` lo traduce (y traduce `const` a `enum` de un solo valor, porque OpenAPI 3.0 no tiene
  `const` — llegó en JSON Schema draft 6)
- [x] **Regla seguida con disciplina, tras un test que la violó primero**: solo FORMA, nunca duplicar una
  regla de dominio que ya tiene su propio código. `crearFaccion` con `nombre: ''` tenía un test que esperaba
  200/`ok:false`/`faccion.nombre_vacio` (rechazo de dominio) — el primer borrador de este esquema traía
  `minLength:1` en `nombre` y lo convertía en 400, rompiendo ese test. Investigado caso por caso, no solo
  arreglado el que falló:
  - `nombre` (`crearFaccion`) → domain: `faccion.nombre_vacio` (`crearFaccion.ts`)
  - `nombre` (`renombrarAsentamiento`) → vacío es válido, significa "usar el id" (`construccion.ts`)
  - `nuevoNombre` (`fusionar`) → vacío cae a un nombre por defecto (`params.nuevoNombre || 'Facción
    Fusionada'`, `diplomacia.ts`)
  - `cantidad`/`cantidadA`/`cantidadB` (trueque, mercado) → domain: `TruequeInvalidoError`/`OrdenInvalidaError`
    para `<= 0` (`engine/trade.ts`, `engine/market.ts`) — ninguno lleva `minimum` en el esquema
- [x] **`EdificioTipo`/`RecursoTipo`/`CargoTipo`/`CargoConstructor` SÍ como `enum` cerrado**, con arrays
  exhaustivos nuevos (`RECURSOS_TIPO`, `EDIFICIOS_TIPO`, `CARGOS_TIPO` en `domain/types.ts`;
  `CARGOS_CONSTRUCTOR` en `construccion.ts`), mismo mecanismo `Record<Union, true>` que fuerza en compilación
  listar cada miembro exactamente una vez. Diferencia con el punto anterior: estos SÍ son tipos discriminados
  cerrados por diseño, y un valor fuera del catálogo no tiene un rechazo de dominio limpio esperándolo —
  confirmado leyendo el código, no supuesto:
  - `EDIFICIO_CATALOGO[tipo].costo` (`engine/construction.ts`, dentro de `anadirEdificioManualmente`) no
    comprueba que `tipo` exista antes de indexar: un `EdificioTipo` inválido llegado por HTTP producía un
    `TypeError` sin capturar por `rechazoDesdeError` (no es un error de dominio reconocido, así que lo
    relanza), que `ejecutarComandoHttp` convertía en 409 — exactamente el síntoma que motivó C9
  - `direccion === 'arriba' ? indice - 1 : indice + 1` (`engine/construction.ts`, `moverEnCola`) y
    `params.tipo === 'vasallaje' ? ... : ...` (`diplomacia.ts`, `proponerRelacion`) son ternarios SIN rama de
    rechazo: un valor inválido no revienta, se **interpreta silenciosamente como el otro lado del par** — un
    bug más grave que un crash, porque no se nota. El `enum` lo corta en el borde
- [x] **`coerceTypes: false`** (`server/api.ts`, `Fastify({ajv: {customOptions: {...}}})`) — Fastify lo trae en
  `true` por defecto, pensado para query/params de URL (siempre texto). Sobre un `body` JSON (ya tipado por
  `JSON.parse`) silenciaba el propio esquema: `{nombre: 123}` pasaba coaccionado a `{nombre: "123"}` en vez de
  rechazarse. Encontrado por un test nuevo que esperaba 400 y recibía 200
- [x] 6 tests nuevos en `server/__tests__/api.test.ts` (falta un campo requerido, tipo JS equivocado, enum
  fuera de catálogo, comando bien formado sigue aceptándose, `nombre` vacío sigue siendo rechazo de dominio,
  el contrato publicado trae el `oneOf`). 612/612 en total, `tsc` limpio. Verificado en vivo contra el
  servidor real: los 5 casos de arriba, uno por uno, con `curl`

### C10. Geometría por frame servida — última pieza del hito (2026-08-26)

- [x] **`trazadoParaAsentamiento`** (nuevo, `engine/trazado.ts`) — mueve al motor la orquestación de
  `redDeCalles`/`segmentosDeRed`/`edificiosInternos`/`celdaMinimaDeEdificio`/`tamanoDeEdificio` que antes SOLO
  vivía en `cliente/src/app/gameStore.ts` (`getTrazadoAsentamiento`). Doc 9 T2a: es una consulta de UN
  asentamiento propio, así que le corresponde al motor, no a la capa de aplicación de un cliente concreto —
  `cliente/` sigue con su copia hasta que se reescriba sin `@motor/*`, fuera de alcance de este hito
- [x] **`RunnerDePartida.geometriaAsentamientos()`** — `zonas` (`computeTodasLasZonas`), `zonasFusionadas`
  (`computeZonasFusionadasPorFaccion`) y `trazadoPorAsentamiento` de TODOS los asentamientos, memoizadas por
  identidad de referencia de `estado.asentamientos` (a diferencia de `cachePrecios`, **sin TTL**: es pura, no
  hay ninguna razón de diseño para que "se sienta" desactualizada un rato — memorizar por `===` es tan preciso
  como un TTL de cero milisegundos)
- [x] **`EstadoAdmin`** (`session/estado.ts`) trae `zonas`/`zonasFusionadas`/`trazadoPorAsentamiento` sin
  filtrar — el admin observa toda la partida (doc 5). Se fusionan en `admin.ts`, no en `vistaAdminDeEstado`:
  mismo criterio impuro que `preciosReferencia`, la función pura no puede calcularlos
- [x] **`ProyeccionJugador`** (`session/proyecciones/jugador.ts`) los trae **filtrados a la Facción propia**:
  `proyectarParaJugador` gana un tercer parámetro (`geometria: GeometriaAsentamientos`, calculado por el
  llamador HTTP) y reutiliza el mismo `esPropio`/`faccionId` que ya filtra caravanas/acuerdos/órdenes/eventos
  — cero geometría de un rival, mismo criterio conservador que el resto de Slice 1 ("mejor no ver nada del
  rival que exponer un nivel de detalle que nadie ha decidido que sea seguro")
- [x] **`GeometriaAsentamientos` vive en `session/estado.ts`, no en `server/runnerDePartida.ts`** — aunque es
  `RunnerDePartida` quien la calcula y cachea, `session/proyecciones/jugador.ts` necesita el TIPO para
  filtrarla, y `session/` no puede importar de `server/` (test de arquitectura). `RunnerDePartida` importa el
  tipo de `session/estado.ts` en vez de declararlo él mismo
- [x] 5 tests nuevos (`server/__tests__/runnerDePartida.test.ts`: vacío sin asentamientos, memoización por
  referencia, recálculo tras fundar, referencia intacta si el comando no toca asentamientos;
  `server/__tests__/api.test.ts`: una jugadora ve solo su zona/trazado, el admin ve los de las dos Facciones).
  617/617 en total, `tsc` limpio. Verificado en vivo contra el servidor real con dos jugadoras y un admin
- [x] **Reconciliación de doc 3/doc 4/doc 9** (ver la nota en el hito C10 del roadmap): el "grupo (b)" de este
  documento (más abajo) quedó disuelto por el doc 9, escrito el mismo día pero después — corregido ahí, no
  solo aquí

### C11b. Terreno indibujable sin `worldgen/` — resuelto sin rasterizar (2026-08-26)

El planteamiento original (doc 3, C11b) daba dos salidas: el servidor rasteriza (PNG o tiles, dependencia
nueva de *encoding*) o los evaluadores de `worldgen/` se publican como librería (la dependencia a eliminar).
Al preguntarle al usuario cuál de las dos, la respuesta fue una tercera que ya estaba en el doc 9 y no se
había conectado con C11b: el terreno es **T2a** ("el terreno lo ven todos", entrada no privilegiada) — no
hace falta que el SERVIDOR lo calcule para nadie, un cliente lo recalcula solo desde los parámetros públicos
que ya sirve C11a. Cero rasterizado, cero dependencia nueva.

- [x] **[`cliente-jugador/`](../../cliente-jugador/)** — proyecto nuevo, hermano de `cliente/`, con su propio
  `package.json`/`tsconfig.json`/`vite.config.ts`. Sin ningún alias `@motor/*` ni `paths` hacia `../src`: es
  la prueba de que el criterio de cierre de la Fase C es alcanzable, no solo una intención
- [x] **`src/terreno/`** — copia deliberada (no import) de la parte de `worldgen/` que evalúa por punto:
  `ruido.ts` (`evaluarRuido`/`evaluarRuidoParcial`, SIN `generarCampoRuido` — eso consume RNG, es generación,
  se queda en el servidor), `elevacion.ts`, `fertilidad.ts`, `biomas.ts`, `rios.ts` (solo
  `distanciaARioMasCercano` + `distanciaASegmento`, no la generación de ríos), `config.ts` (solo los umbrales
  que usa la evaluación: `ELEVACION`, `ELEVACION_SUAVIZADO`, `BIOMA`). Ver `cliente-jugador/src/terreno/README.md`
  para la tabla completa de qué se copió y qué no, y la disciplina de mantenimiento frente a `WORLDGEN_VERSION`
- [x] **Limitación documentada, no resuelta**: sin soporte de `region` (`worldgen/regiones.ts`, ~300 líneas de
  guías geográficas autoradas, no portadas) — una partida creada con `region` diverge del terreno real donde
  pesa la guía. `elevacion.ts` avisa por consola (una vez) si detecta `campo.region`. El caso sin región
  ("mundo libre", el default de la mayoría de partidas) se reproduce completo, borde incluido
- [x] **`render.ts`** pinta bioma (paleta propia, `BiomaTipo` no lleva color) + ríos a canvas, muestreando cada
  8 unidades de mapa (2000×2000 a 1:1 son 4M muestras, inviable por frame) — 250×250 muestras, aceptable
- [x] **Verificado en vivo, no solo por inspección de código**: la misma partida (`gameId: 'local'`, seed 42,
  sin región) renderizada a la vez en `cliente/` (motor real, `@motor/*`) y `cliente-jugador/` (evaluador
  duplicado) — capturas comparadas, la geografía coincide: mismos lagos, mismas montañas, mismos ríos, en la
  misma posición y forma. Es la prueba de que la copia es bit a bit correcta para el caso sin región, no una
  suposición
- [x] `tsc --noEmit` limpio en el proyecto nuevo; el backend (`src/`) no se tocó, sigue en 617/617

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
