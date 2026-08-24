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
- [ ] **Pendiente, diferido a Fase B3 a propósito**: el estado interno del RNG (semilla consumida) no se persiste en `exportarSimulacion`/`importarSimulacion` — un import reinicia el RNG desde la seed del mundo en vez de restaurar el punto exacto de la secuencia ya consumida, así que dos partidas "idénticas" tras un import pueden divergir. Documentado en el comentario del campo `GameStore.rng`. Resolverlo pertenece al diseño de persistencia real (B3: "Persistir estado, tick, RNG, IDs...").

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
  - **Progreso de migración de comandos: 8 / 34** — `fundarAsentamiento`, `crearFaccion`, `asignarRey`, `asignarEmbajador`, `asignarCargoLocal`, `comprarCasa`, `activarPolitica`, `alternarFaccionNpc` (más las 2 operaciones de sistema `avanzarTick` y `avanzarFaccionesNpc`, que no son comandos de jugador)
  - **Bug latente corregido al migrar**: en `GameStore` estos comandos hacían `.find(...)!` sobre Facción/asentamiento; si la entidad no existía, el `!` dejaba pasar `undefined` y el motor reventaba con un `TypeError` en vez de rechazar. Los handlers comprueban antes y devuelven `faccion.no_existe` / `asentamiento.no_existe`. Cubierto por 3 tests que en la versión anterior habrían fallado con excepción
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
- [ ] **⚠️ B3 — Hacer explícitas las mutaciones de `Mapa` ANTES de implementar la persistencia** (decidido 2026-08-24 posponerlo hasta aquí, ver [7_Diseno_GameSession.md](7_Diseno_GameSession.md) §7.3 — NO se pierde: es prerrequisito del guardado, no algo opcional). Hoy `Mapa` es lo único no funcional puro del motor: `extraer` (desde `engine/construction.ts`) y `avanzarRegeneracion` (desde `engine/simulation.ts`) mutan por dentro en vez de devolver el mapa nuevo. Al persistir tras cada tick se guarda *estado + eventos*; si parte del estado se mutó por un camino lateral, puede guardarse un snapshot que no corresponde a los eventos emitidos, y un comando que falla a mitad deja el mapa modificado aunque el resto no (rollback parcial silencioso). **Vigilar que sigan siendo exactamente esos 2 puntos** — si aparecen más antes de B3, el arreglo crece
- [ ] Implementar guardado de: estado, tick, RNG, IDs, configuración, eventos, con versión de concurrencia
- [ ] Persistir `eventosDominio` en el snapshot desde el principio — no para el jugador (ver decisión §7.1 del doc 7: `log`/`historialJugadores` son de administración y no viajan a jugadores) sino para la auditoría y el replay de la Fase E
- [ ] Definir y documentar el esquema de snapshot/versión de formato de partida
- [ ] Exponer API HTTP administrativa mínima (crear partida, avanzar tick, consultar estado)
- [ ] Migrar `main.ts` para hablar con esa API en vez de con `GameStore` local (puede convivir temporalmente con un modo local para desarrollo)
- [ ] **Resolver el bloqueo de la cola serial por ticks largos** — un tick de ~1.9 s a 500 asentamientos son ~1.9 s sin procesar comandos de nadie (el doc 2 no lo contempla). Vías a evaluar: comandos por lotes entre ticks, partir el tick en fases cedibles, o mover el tick a un worker aparte del hilo que atiende comandos. **Decisión pendiente** — se aborda dentro del diseño del `RunnerDePartida` (ver [7_Diseno_GameSession.md](7_Diseno_GameSession.md) §4): al quedar la cola FUERA de `GameSession`, estas tres vías se pueden probar y cambiar sin tocar la lógica de partida

## Fase C — Multijugador sobre ticks

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
- [ ] RNG determinista con estado persistido (partidas reproducibles tras reinicio)
- [ ] DTOs/proyecciones por audiencia (nunca enviar `GameState` completo a un cliente no-admin)
- [ ] Snapshots y retención para el historial (nunca clones ilimitados en RAM)
- [ ] Balance versionado y ligado a partida/temporada (no global mutable)
- [ ] Frontends y endpoints de admin vs. jugador separados con roles técnicos distintos
