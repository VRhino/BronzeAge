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

## 7. Preguntas abiertas

- **`log` e `historialJugadores` son hoy parte de `GameState`** y se persisten en el archivo de guardado.
  En el diseño nuevo lo persistido deberían ser `EventoDominio[]` y el texto ser una proyección — pero eso
  depende del avance del marcador 0/13 de A5. Mientras tanto, `GameSession` puede conservarlos tal cual sin
  romper nada. Decidir si se migran al cerrar A5 o antes.
- **`faccionesNpcIds`** vive en `GameState` (capa de aplicación, no dominio, y está bien así). En el servidor
  pasa a ser configuración de partida — ¿la lleva `GameSession` o el runner? Se inclina a `GameSession`,
  porque afecta a lo que ocurre en el tick.
- **Mutación de `Mapa`** (`extraer`/`avanzarRegeneracion`, los 2 puntos identificados en el doc 6 §"puntos de
  fuga"): el doc 2 pide hacerlas explícitas en el resultado del tick. ¿Se aborda al crear `GameSession` o
  después? Afecta a si un snapshot puede escribirse de forma consistente con los eventos emitidos.
- **Multi-partida en un proceso**: `GameSession` recibe `gameId` y no es singleton (a diferencia del
  `gameStore` exportado hoy). Si un proceso aloja varias partidas o solo una es decisión del runner, no suya.
