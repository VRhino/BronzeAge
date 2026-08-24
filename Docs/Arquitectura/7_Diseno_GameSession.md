# Diseño de `GameSession` (Fase B, tarea 1)

Diseño acordado antes de escribir código, según la tarea "Definir la forma de `GameSession`" de
[4_Plan_Evolucion_Tareas.md](4_Plan_Evolucion_Tareas.md). Nada de esto está implementado todavía.

`GameSession` es la pieza que sustituye a `GameStore` como dueña de una partida. El doc 2 la describe en una
línea ("Crear una capa de aplicación de servidor... que reciba una partida concreta y ejecute comandos"); este
documento la concreta a partir del inventario real de `src/app/gameStore.ts`.

## 1. Inventario: qué hace hoy `GameStore`

Analizado sobre el archivo real (~1570 líneas). Cuatro grupos, con destinos distintos:

| Grupo | Miembros | Destino |
|---|---|---|
| **Partida y motor** | `state`, `ids` (`GeneradorIds`), `rng`, `mapasPorEstado`, los ~35 comandos, `avanzarTick`, `avanzarFaccionesNpc`, `regenerarMundo`, `exportarSimulacion`, `importarSimulacion` | → **`GameSession`** |
| **Sesión local de navegador** | `listeners`, `subscribe`, `notify`, `historial`, `historialDesde`, `getSnapshot`, `clonarEstadoActual`, `zonasFusionadasCache` | → se queda en el **cliente**; desaparece del servidor |
| **Presentación** | `log`, `historialJugadores`, `registrar`, `registrarJugador`, y las ~15 consultas derivadas (`produccionInfo`, `manoObraInfo`, `nivelAsentamientoInfo`, `getLigas`…) | → **proyecciones/DTOs** (Fase C) |
| **Administración** | `getBalance`, `actualizarBalance`, `restaurarBalance`, `exportarMapaUnity` | → endpoints con **rol técnico** (Fase C) |

Dos hallazgos del inventario que cambian el diseño:

**(a) `GameStore` no usa ninguna API de navegador.** Cero `Blob`, `FileReader`, `document` o `window` — eso
vive en `main.ts`. `exportarSimulacion()` devuelve un `string` e `importarSimulacion(json)` recibe otro. La
extracción es bastante más limpia de lo que anticipaba el doc 2 ("Mantener fuera de esta capa el DOM,
`Blob`, `FileReader`…"): ya está fuera.

**(b) `notify()` clona el estado ENTERO en cada comando.** Hoy hace
`this.historial[tick] = this.clonarEstadoActual()`, que es un `structuredClone` de todas las entidades, y se
invoca desde **38 sitios** (cada comando, no solo el tick). A la escala objetivo (~2.4 MB de estado, ver
[doc 6](6_Sincronizacion_Visibilidad_y_Escala.md) §1) eso es insostenible: 500 jugadores emitiendo comandos
clonarían megabytes por comando. Es correcto como herramienta de depuración local de una partida de un solo
usuario; no puede sobrevivir al servidor. **`GameSession` no lleva historial ni clona nada.**

## 2. Qué es y qué no es `GameSession`

> **`GameSession` es la partida: estado + reglas + identidad de las entidades. Síncrona, sin E/S, sin red,
> sin reloj propio, sin cola.**

**Es responsable de:**

- Poseer el estado de UNA partida (`gameId`).
- Ejecutar comandos: validar contra el motor y aplicar el resultado.
- Ejecutar el tick y el turno del NPC de gobernanza.
- Poseer el contexto de partida: `GeneradorIds`, `RandomFn`, y el `tick`/`momento` que recibe.
- Traducir errores de dominio a **resultados estructurados** (nunca a texto de log).
- Emitir `EventoDominio[]` por cada operación.
- Serializarse y reconstruirse (`exportar()` / `importar()` sobre datos, no sobre archivos).
- Llevar una `version` que incrementa en cada mutación, para control de concurrencia optimista.

**NO es responsable de** (y esto es lo que la mantiene reutilizable):

| No hace | Quién lo hace |
|---|---|
| Leer el reloj | El llamador, vía `ContextoSimulacion.momento` (ya implementado) |
| Escribir en disco / base de datos | El runner de partida (§4) |
| Cola de comandos, orden, concurrencia | El runner de partida (§4) |
| HTTP, WebSocket, suscripciones | La capa de transporte (Fase C) |
| Autenticación y autorización | El middleware de API, según [doc 5](5_Contratos_Identidad_Permisos.md) |
| Notificar a la UI (`subscribe`/`notify`) | El cliente |
| Historial de depuración por tick | El cliente (hoy) / snapshots persistidos (Fase E) |
| Texto de log localizado | Las proyecciones, derivándolo de `EventoDominio` |

La prueba de que la frontera se respeta es la misma que ya tenemos automatizada: `GameSession` debe poder
ejercitarse en un test síncrono, igual que el motor, y `scripts/run-batch-sim.ts` debe seguir corriendo sin
tocar nada de infraestructura (ver [test de arquitectura](../../src/__tests__/arquitectura.test.ts)).

## 3. Contrato de comando

Hoy cada comando de `GameStore` sigue este patrón: llama al motor, atrapa el error de dominio
(hay **13 tipos** distintos: `FundacionInvalidaError`, `CaravanaInvalidaError`, `CombateInvalidoError`, …),
lo convierte a una frase en castellano y la mete en el log. El resultado se pierde: quien llamó no sabe si
funcionó.

Eso no puede ser el contrato con un cliente remoto — el doc 2 (punto 6) ya lo advierte: *"los logs
localizados son una presentación; no deben ser el único contrato entre servidor y frontend"*.

```ts
interface ResultadoComando {
  ok: boolean;
  /** Código estable de dominio en caso de rechazo (ej. 'fundacion.cap_alcanzado'), NUNCA texto localizado.
   *  Sale del tipo de error del motor, no de su mensaje. */
  codigoError?: string;
  /** Datos del rechazo para que el cliente componga su propio mensaje (ej. qué recurso faltó y cuánto).
   *  Cubre la petición pendiente de Notas_revision: "cuando el motor dé un mensaje de fondos insuficientes,
   *  que muestre lo que falta". */
  detalle?: Record<string, unknown>;
  /** Eventos producidos por este comando. */
  eventos: EventoDominio[];
  /** Versión de la partida tras aplicarlo — base del control de concurrencia optimista. */
  version: number;
}
```

Regla: **un comando rechazado no muta nada y no incrementa `version`.** Hoy eso se cumple por accidente
(el motor lanza antes de mutar); con el contrato explícito pasa a ser verificable.

El mapeo de los 13 tipos de error a códigos estables es trabajo mecánico pero hay que hacerlo una vez y
documentarlo — es parte de la superficie pública de la API.

## 4. La cola serial: decisión

**Decisión: la cola NO vive dentro de `GameSession`.**

El doc 2 (principio 5) exige procesar en serie por `gameId`, y el [doc 6](6_Sincronizacion_Visibilidad_y_Escala.md) §1
midió el problema que eso crea: un tick de ~1.9 s a 500 asentamientos son ~1.9 s sin atender comandos.

Se separa en dos piezas:

```text
RunnerDePartida            ← infraestructura: cola, scheduler, persistencia, difusión. Asíncrono.
      │  llama en serie
      ▼
GameSession                ← partida: estado + reglas. Síncrono, sin E/S.
      │
      ▼
engine/ world/ worldgen/   ← juego puro, ya existente
```

Por qué así y no con la cola dentro:

- **`GameSession` sigue siendo testeable síncronamente**, como el motor. Si llevara cola sería asíncrona y
  cada test tendría que esperar promesas para comprobar una regla de juego.
- **El problema del tick largo es del runner**, no de la partida. Las tres vías anotadas (comandos por lotes
  entre ticks, partir el tick en fases cedibles, moverlo a un worker) se pueden probar y cambiar **sin tocar
  una línea de `GameSession`**.
- Es la misma razón por la que el motor no conoce a `GameStore`: separar *qué se decide* de *cuándo y en qué
  orden se decide*.

El diseño del `RunnerDePartida` (y con él la resolución del tick bloqueante) queda como tarea aparte, después
de que `GameSession` exista. **No se diseña ahora** para no volver a mezclar dos problemas.

## 5. Dónde vive

**Capa nueva: `src/session/`.** No dentro de `app/`.

Motivo: `app/gameStore.ts` es la sesión de UNA pestaña de navegador (suscriptores, historial de depuración,
caché de render). `GameSession` tiene que poder instanciarse en un proceso Node sin nada de eso. Compartir
carpeta invita a que alguien vuelva a mezclarlos, y ya sabemos que la frontera solo aguanta si algo la vigila.

Reglas de capa a añadir en [arquitectura.test.ts](../../src/__tests__/arquitectura.test.ts) al crearla:

```ts
session: ['domain', 'worldgen', 'world', 'engine', 'constants'],   // NO 'app', NO 'ui'
app:     [...permitidasActuales, 'session'],                        // app pasa a consumir session
```

Con eso, un import accidental de `session → app` (o peor, `session → ui`) falla en la suite, igual que ya
falla `engine → app`.

## 6. Plan de migración sin romper la interfaz

El riesgo real no es escribir `GameSession`, es dejar la UI a medias durante la transición. El orden que lo
evita:

1. **Crear `session/gameSession.ts`** con el estado y los comandos, devolviendo `ResultadoComando`. Todavía
   nadie la usa.
2. **Convertir `GameStore` en adaptador delgado** sobre `GameSession`: mantiene `subscribe`/`notify`/
   `historial`/`log` (lo del navegador) y traduce `ResultadoComando` → entrada de log en texto. **La UI no
   cambia ni una línea** y la suite actual sigue siendo la red de seguridad.
3. **Mover comandos de a grupos**, no todos de golpe — la suite completa en verde entre grupo y grupo.
4. Cuando no quede lógica de partida en `GameStore`, lo que queda es un cliente local: exactamente la pieza
   que en Fase B5 se sustituye por un cliente de API remota.

Este orden mantiene la propiedad que ha funcionado en toda la Fase A: cada paso es verificable con los tests
que ya existen, y en ningún momento hay un estado intermedio roto.

## 7. Decisiones sobre las preguntas abiertas (resueltas 2026-08-24)

### 7.1 `log` e `historialJugadores` son datos de ADMINISTRACIÓN

Aclaración del usuario que cambia el encuadre: **estos dos campos no viajan nunca a los jugadores**. Se
quedan en el servidor; como mucho los consulta un administrador directamente. No forman parte de ninguna
proyección de jugador.

Consecuencias:

- **No hay urgencia de migrarlos a `EventoDominio`.** La regla del doc 2 ("los logs localizados no deben ser
  el único contrato entre servidor y frontend") aplica al contrato con el JUGADOR, y estos quedan fuera de él.
  Como superficie de administración, el texto ya formateado es un formato perfectamente razonable.
- **`GameSession` los conserva tal cual**, sin tocarlos. Cero riesgo, la interfaz actual sigue funcionando.
- **Pero se persiste `eventosDominio` en paralelo desde el principio**, no por el jugador sino por la
  auditoría y el replay de la Fase E, que sí necesitan datos estructurados.
- ⚠️ **Marcar explícitamente estos dos campos como "solo administración"** en el código y en los DTOs, para
  que nadie los incluya por descuido en una proyección de jugador — sería una fuga de información de otras
  facciones (el log global narra lo que pasa en todo el mundo).

Esto desacopla el cierre del marcador 0/13 de A5 de la Fase B: la migración de eventos avanza a su ritmo sin
bloquear nada.

### 7.2 `faccionesNpcIds` vive en `GameSession`

**Decidido: en `GameSession`**, y cambiarlo es un comando administrativo (rol técnico, ver
[doc 5](5_Contratos_Identidad_Permisos.md)).

Criterio: *si un dato cambia el resultado de un tick, es estado de partida* — si viviera en el runner y este
se reiniciara con otra configuración, la partida cambiaría de comportamiento sin que el snapshot lo reflejara,
rompiendo el principio 6 del doc 2 ("un reinicio no puede alterar la secuencia").

Matiz aportado por el usuario: **en una partida real este valor no cambia en caliente**. Una facción que se
declara IA lo es hasta que se destruye. Es configuración efectivamente inmutable tras la creación de la
partida — lo que simplifica el diseño: no hace falta prever recálculos ni invalidación de cachés al
cambiarla, y el comando administrativo que la modifica puede ser de uso excepcional (corrección/moderación),
no una palanca de juego.

### 7.3 Mutación de `Mapa`: se aborda en B3, no antes

**Decidido: posponer a B3 (persistencia), dejándolo escrito para que no se pierda.** Anotado como tarea
explícita en [4_Plan_Evolucion_Tareas.md](4_Plan_Evolucion_Tareas.md).

Motivo para no hacerlo ahora: crear `GameSession` ya es un refactor grande, y no necesita este arreglo para
existir. Meter los dos en el mismo paso mezclaría dos refactors del motor a la vez, justo lo que venimos
evitando.

Motivo para no dejarlo indefinidamente: es donde se vuelve un problema real. Al persistir tras cada tick se
guarda *estado + eventos*; si parte del estado (lo extraído de cada yacimiento) se mutó por un camino lateral,
puede guardarse un snapshot que no corresponde a los eventos emitidos. Y si un comando falla a mitad, el mapa
queda modificado aunque el resto del estado no — un rollback parcial silencioso.

**Vigilar que sigan siendo dos.** Hoy la superficie es exactamente `Mapa.extraer` (desde
`engine/construction.ts`) y `Mapa.avanzarRegeneracion` (desde `engine/simulation.ts`). Si aparecen más
mutaciones laterales antes de B3, el arreglo crece.

### 7.4 Una partida por proceso

**Decidido: cada proceso gestiona exactamente una partida.** Coincide con lo que ya decía el doc 2, y ahora
con una razón medida además de la de aislamiento: Node ejecuta JS en un solo hilo, así que dos partidas en un
proceso compartirían hilo y el tick de una dejaría a la otra sin atender comandos.

`GameSession` recibe `gameId` y **no es singleton** (a diferencia del `gameStore` exportado hoy). Eso mantiene
la decisión reversible sin reescribir nada.

Ver §8 para la cuestión del multihilo, que se derivó de esta decisión.

## 8. Multihilo: preparar, no construir

Pregunta derivada de §7.4: ¿conviene abordar ya una estrategia multihilo (`worker_threads`)?

**Decidido: no construirlo ahora; dejar el contrato preparado para poder hacerlo sin reescribir.**

### 8.1 El multihilo NO resuelve el problema de la cola serial

Distinción que hay que tener clara antes de decidir, porque es la que hace que la respuesta intuitiva sea
equivocada. El tick largo causa **dos** problemas distintos:

| | Problema | ¿Lo resuelve el multihilo? |
|---|---|---|
| **A** | Durante el tick no se APLICA ningún comando | **No.** La restricción es lógica, no de CPU: el estado solo admite un mutador a la vez. Mover el trabajo a otro hilo no cambia eso |
| **B** | Durante el tick el event loop está bloqueado: los heartbeats de WebSocket no responden, no se aceptan conexiones nuevas, un health check expira | **Sí.** Este es el problema que el multihilo sí ataca |

El problema B es el que de verdad duele a 500 jugadores: un event loop bloqueado 1.9 s puede hacer que los
clientes den la conexión por muerta. El A es inherente a tener un estado consistente y no se elimina, solo se
gestiona (encolar y confirmar recepción aunque la aplicación llegue después).

### 8.2 La magnitud real es menor de lo estimado

Corrección importante sobre las cifras del [doc 6](6_Sincronizacion_Visibilidad_y_Escala.md) §1: **500
jugadores no son 500 asentamientos**. Un asentamiento aloja `CIUDADANIA.casasBasePorAsentamiento` = 5
residentes (+2 por nivel adicional), así que 500 jugadores caben en ~70-100 asentamientos.

Sobre la curva medida (O(n^1.5)), eso da **~170-300 ms por tick**, no 1.9 s. Los 500 asentamientos son un
escenario de partida madura (las facciones expanden con el tiempo, ver `CAP_FUNDACION_POR_NIVEL`), no el
punto de partida.

Un bloqueo de ~200 ms es molesto pero perfectamente tolerable para heartbeats de WebSocket. **El multihilo no
hace falta para arrancar.**

### 8.3 Orden de soluciones cuando haga falta, de menor a mayor coste

1. **Cesión cooperativa del event loop** — ceder (`setImmediate`) cada N asentamientos dentro del bucle del
   tick. Mantiene un solo hilo, sin transferir estado. Es lo más barato y probablemente suficiente. Coste: el
   bucle del tick pasa a ser asíncrono, lo que contaminaría el motor con `async` — hay que decidir si se hace
   en el motor o exponiendo el tick por lotes que conduzca el runner.
2. **Worker que posee la partida entera** — el hilo principal queda como E/S pura (HTTP, WebSocket,
   persistencia) y el worker aloja `GameSession` de forma permanente. Los comandos son mensajes. Resuelve el
   problema B por completo y **sin transferir estado en cada tick** (el estado vive siempre dentro del
   worker). El problema A permanece idéntico, porque el worker sigue siendo serial.
3. **No viable**: repartir un mismo tick entre varios hilos. El estado es compartido y mutable; haría falta
   sincronización que costaría más de lo que ahorra.

### 8.4 Qué preparar ahora (coste cero)

- **`GameSession` síncrona y autocontenida** — ya es la decisión de §2. Es justo lo que la hace *movible* a un
  worker tal cual, sin reescribirla.
- **El contrato del `RunnerDePartida` nace asíncrono** (`async avanzarTick()`, `async ejecutar(comando)`)
  aunque hoy por dentro no espere nada. Así, migrar a cesión cooperativa o a un worker es un cambio *dentro*
  del runner, invisible para sus llamadores.
- **No introducir `worker_threads` todavía.** Añade serialización de mensajes, depuración más difícil y una
  forma nueva de fallo, a cambio de resolver un problema que a la escala inicial no se manifiesta.

Disparador para reconsiderarlo: cuando se mida un tick por encima de ~500 ms en una partida real, o cuando
aparezcan desconexiones de WebSocket atribuibles al bloqueo del event loop.
