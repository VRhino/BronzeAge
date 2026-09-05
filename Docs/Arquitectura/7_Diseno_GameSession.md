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

## 2.bis Forma de los comandos: funciones, no métodos (revisión 2026-08-24)

La primera implementación puso los comandos como **métodos** de `GameSession`. Se revisó antes de migrar el
resto y se cambió, por una razón medida:

| | Líneas | Comandos | Consultas |
|---|---|---|---|
| `GameStore` hoy | 1582 | 34 | 31 |
| `GameSession` con métodos, extrapolada a 34 comandos | ~1100+ | 34 | — |

Es decir: habría reproducido el mismo objeto-dios que el doc 2 critica de `GameStore`, cambiando de carpeta
pero no de problema. La forma adoptada es:

```text
session/
  estado.ts              GameSessionState + transformaciones PURAS (sin métodos que muten)
  comandos/
    tipos.ts             ContextoComando, ResultadoComando, TransicionComando, exito()/rechazo()
    fundarAsentamiento.ts    un archivo por comando
    crearFaccion.ts
    avanzarTick.ts
    avanzarFaccionesNpc.ts
  gameSession.ts         despachador + estado actual (139 líneas, y no crece al añadir comandos)
```

Tres decisiones concretas:

**(a) Un comando es una función pura** `(estado, mapa, ctx, params) -> { estado, resultado }`. Nunca muta el
estado que recibe; un rechazo devuelve *el mismo objeto*. Esto importa para la Fase B3: con transiciones
puras el runner puede hacer *aplicar → persistir → confirmar* y **descartar** el estado nuevo si la escritura
falla. Con mutación, un fallo de persistencia deja el estado ya modificado y sin vuelta atrás.

> ✅ **Resuelto el 2026-08-25** (§7.3). La mutación de `Mapa` era el único agujero de esa garantía: `extraer`
> modificaba `estadoMapa` fuera del valor de retorno. Desde el arreglo, la garantía es completa: **todo** lo
> que un comando cambia sale por su estado resultante.

**(b) `ContextoComando { momento, actor, rng, ids }`** en vez de parámetros sueltos. El `momento` no es un
dato del comando sino contexto de ejecución, y `actor` hará falta en **todos** los comandos cuando llegue la
autorización de Fase C. Queda simétrico con el `ContextoSimulacion` del motor, y por el mismo motivo: el
reloj y la aleatoriedad se inyectan, nunca se leen dentro.

**(c) `exito()` es el único sitio que incrementa `version`.** Así no puede existir un comando que mute el
estado y se olvide de versionarlo — que es precisamente lo que rompería el control de concurrencia optimista.

Beneficio adicional que no es de estilo: cada comando puede llevar **junto a su lógica** los metadatos de
autorización de su fila en la matriz del [doc 5](5_Contratos_Identidad_Permisos.md), en vez de en una tabla
paralela que se desincroniza en cuanto alguien añade un comando y olvida la tabla.

## 2.ter Lo que NO es un comando (aclaración del usuario, 2026-08-25)

Al llegar al último grupo de la migración se vio que tres de los "comandos" de `GameStore` no lo son. Lo
destapó una observación del usuario sobre `regenerarMundo`:

> *"Regenerar mundo es algo que se lanza desde fuera, antes de empezar una partida. Una vez empezada la
> partida no debería dispararse nunca, porque reinicia todo y se pierde todo lo que se tenía en esa partida."*

Eso no es un matiz de UI: es la diferencia entre **operar sobre una partida** y **decidir qué partida existe**.
`GameStore` las mezcla porque en una herramienta local de una sola pestaña solo hay una partida y un proceso,
así que "regenerar el mundo" y "jugar" caben en el mismo objeto. En un servidor no.

| Operación de `GameStore` | Qué es en realidad | Dónde vive |
|---|---|---|
| `regenerarMundo` | **Creación** de partida. Destruye la anterior por completo | `GameSession.crear()` — ya existe. Nunca un comando de una partida viva |
| `importarSimulacion` | **Reconstrucción** de partida desde datos | `GameSession.importar()` — ya existe. Tampoco es un comando |
| `actualizarBalance` / `restaurarBalance` | Mutación de **configuración global del proceso** | Ni comando ni partida — ver abajo |

Las dos primeras ya estaban resueltas sin darnos cuenta: son las factorías estáticas de `GameSession`. No hay
nada que migrar; lo que había que hacer era **dejar de considerarlas comandos**.

La consecuencia operativa importa: en el backend, "regenerar el mundo" de una partida en curso no es una
mutación sino **descartar esa partida y crear otra** — una operación destructiva de administración, sujeta a
rol técnico y a confirmación explícita (doc 5), no un botón más de la consola.

### El balance es peor: es estado global del proceso

`app/balanceConfig.ts` **muta en el sitio los objetos de `constants.ts`**, que son los mismos objetos que
importa cada módulo de `engine/`. Es decir: el balance no es estado de una partida, es estado **del proceso**,
compartido por todo lo que corra en él.

Con una partida por proceso (§7.4) el riesgo queda contenido hoy, pero sigue siendo exactamente lo que el
doc 2 marca como riesgo ("Balance global mutable → partidas afectadas entre sí") y lo que su punto 8 manda
convertir en **configuración versionada por partida, con auditoría**. Por eso estos dos no se migran como
comandos: hacerlo fingiría que son transiciones de estado de partida cuando no lo son, y consolidaría el
error. Quedan para la Fase C, junto al resto del trabajo de balance.

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
midió el problema que eso crea: mientras corre el tick no se atiende ningún comando.

> **Cifras al día (2026-09-05).** Este apartado y el §8 se escribieron sobre la medición de agosto, que
> extrapolaba **~1,9 s por tick a 500 asentamientos**. Esa cifra ya no vale por dos motivos independientes:
> 500 jugadores son ~70-100 asentamientos y no 500, y el tick se optimizó 4,3× en la Fase E3. Hoy son
> **70-111 ms**. **Ninguna decisión de este documento cambia** —salen todas reforzadas, que es justo por lo
> que conviene actualizar los números en vez de dejarlos: un argumento que se apoya en una cifra falsa es
> frágil aunque su conclusión sea correcta.

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

> **✅ Implementado el 2026-08-25** — `src/server/runnerDePartida.ts`. Cola serial (encadenando promesas, sin
> mutex: JS ya es de un solo hilo) + ciclo "aplicar -> persistir -> confirmar" del §2(a) sobre
> `persistenciaPartida.ts` + scheduler de ticks automáticos. La RESOLUCIÓN del tick bloqueante sigue sin
> construirse a propósito (§8.2: no hace falta a la escala de arranque) — lo que sí quedó resuelto es la
> preparación de coste cero del §8.4: la interfaz ya es `async`, así que la solución que haga falta el día
> que haga falta se implementa dentro del runner sin cambiar quién lo llama. Detalle en
> [4_Plan_Evolucion_Tareas.md](4_Plan_Evolucion_Tareas.md).

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
2. **Mover los 34 comandos a `session/comandos/`, de a grupos** — la suite completa en verde entre grupo y
   grupo. `GameStore` sigue intacto y funcionando mientras tanto.
3. **Convertir `GameStore` en adaptador delgado** sobre `GameSession`: mantiene `subscribe`/`notify`/
   `historial`/`log` (lo del navegador) y traduce `ResultadoComando` → entrada de log en texto. **La UI no
   cambia ni una línea** y la suite actual sigue siendo la red de seguridad.
4. Cuando no quede lógica de partida en `GameStore`, lo que queda es un cliente local: exactamente la pieza
   que en Fase B5 se sustituye por un cliente de API remota.

> **Corrección del orden (2026-08-24).** Los pasos 2 y 3 estaban al revés en la primera versión de este
> documento. Convertir `GameStore` en adaptador ANTES de migrar los comandos es imposible: `GameStore` tiene
> **70 mutaciones directas** de `this.state`, así que mientras quede un solo comando sin migrar habría dos
> fuentes de verdad —el estado de `GameSession` y el de `GameStore`— divergiendo desde el primer comando que
> se ejecutara. El paso 3 solo puede hacerse cuando ya no queda ninguna, y entonces es un cambio mecánico y
> atómico en vez de una convivencia frágil.

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

> El RNG de partida rompía exactamente este mismo principio hasta el 2026-08-25: `importar` lo reiniciaba
> desde la seed del mundo en cada carga, así que un reinicio SÍ alteraba la secuencia — no en el sentido de
> "la partida diverge de lo que debería ser" (no hay un debería ser externo, el servidor es autoridad única),
> sino en que dos reinicios reales consumían la misma racha de números, y un snapshot ya no bastaba para
> reconstruir el incidente que motivó guardarlo. Resuelto persistiendo el contador interno del RNG
> (`PartidaExportada.estadoRng`, `worldgen/rng.ts`) — ver la tarea en
> [4_Plan_Evolucion_Tareas.md](4_Plan_Evolucion_Tareas.md).

Matiz aportado por el usuario: **en una partida real este valor no cambia en caliente**. Una facción que se
declara IA lo es hasta que se destruye. Es configuración efectivamente inmutable tras la creación de la
partida — lo que simplifica el diseño: no hace falta prever recálculos ni invalidación de cachés al
cambiarla, y el comando administrativo que la modifica puede ser de uso excepcional (corrección/moderación),
no una palanca de juego.

### 7.3 Mutación de `Mapa`: resuelta (2026-08-25)

**Decidido en su día: posponer a B3 (persistencia), dejándolo escrito para que no se pierda.** Se abordó al
empezar B3, antes de escribir una sola línea de guardado, que era el orden acordado.

**El problema.** `Mapa` era una fachada sobre `estadoMapa` que *aliaseaba* el objeto de la partida: `extraer`
(desde `engine/construction.ts`) y `avanzarRegeneracion` (desde `engine/simulation.ts`) escribían dentro. Al
persistir tras cada tick se guarda *estado + eventos*, así que descartar el estado nuevo por un fallo de
escritura no revertía los yacimientos ya vaciados — snapshot y memoria divergían en silencio, y el snapshot
podía no corresponder a los eventos emitidos.

**La solución, en una frase: la mutación deja de ser un efecto lateral y pasa a ser un valor.**

| Antes | Ahora |
| --- | --- |
| `Mapa` aliasa el `EstadoMapa` de la partida | `Mapa` lo **copia** al construirse; expone `estadoActual()` |
| El tick escribe el mapa por dentro | El tick lo devuelve: `ResultadoTick.estadoMapa` |
| Una fachada para toda la vida de la sesión | Una fachada **por comando**, creada en `GameSession.ejecutar` |
| Olvidarse de propagar = corrupción silenciosa | Olvidarse = excepción en `ejecutar` |

La fachada por comando es lo que cierra el agujero de raíz, no solo en el tick: lo que un comando escriba y no
devuelva se va con la fachada al terminar. Y la comprobación de `ejecutar` (comando **aceptado** + fachada
tocada + `estadoMapa` sin cambiar de referencia → excepción) convierte el olvido en un fallo ruidoso. Un
comando **rechazado** no la dispara: devuelve el estado intacto a propósito, y ahí descartar es lo correcto.

**Coste y cómo se pagó.** Instanciar `Mapa` pasó de una vez por partida a una vez por comando. Sus índices
(ids, rejilla espacial, orden de generación) son función del `MapaGenerado`, que es inmutable, así que se
cachean por mundo en un `WeakMap` — la misma técnica que ya usaban los contornos de bosque. Lo que queda por
comando es copiar dos registros de números.

**La superficie sigue siendo dos.** `Mapa.extraer` y `Mapa.avanzarRegeneracion`, verificado al cerrar. Ya no
hay que vigilarlo con la misma urgencia —una mutación lateral nueva se pierde en vez de corromper, y si el
comando la acepta salta la excepción— pero sigue siendo la superficie a mirar si algo del mapa no cuadra.

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

El problema B es el que de verdad duele a 500 jugadores: un event loop bloqueado durante segundos puede
hacer que los clientes den la conexión por muerta. (Con las cifras de 2026-09-05 —111 ms— no llega a
plantearse; sí lo hace durante una ráfaga de catch-up, ver doc 6 §1.) El A es inherente a tener un estado consistente y no se elimina, solo se
gestiona (encolar y confirmar recepción aunque la aplicación llegue después).

### 8.2 La magnitud real es menor de lo estimado

Corrección importante sobre las cifras del [doc 6](6_Sincronizacion_Visibilidad_y_Escala.md) §1: **500
jugadores no son 500 asentamientos**. Un asentamiento aloja `CIUDADANIA.casasBasePorAsentamiento` = 5
residentes (+2 por nivel adicional), así que 500 jugadores caben en ~70-100 asentamientos.

Sobre la curva medida entonces (O(n^1.5)) eso daba **~170-300 ms por tick**, no 1,9 s; re-medido el
2026-09-05 y tras optimizar, **70-111 ms**. Los 500 asentamientos son un
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

### 8.5 ¿Y en el modelo de tiempo real? (pregunta del usuario)

Objeción legítima: el multihilo brilla cuando se puede **delegar trabajo en paralelo**. Que no encaje con el
modelo de ticks no significa que no encaje con el de tiempo real. Se analizó con mediciones.

**Medición: reparto del coste del tick** (100 asentamientos vivos, 60 ticks de calentamiento, media de 25).

> **Cifras de agosto de 2026.** El ABSOLUTO ya no vale: el tick completo a 100 asentamientos son **110,7 ms**
> medidos el 2026-09-05, tras engordar con trazado urbano/murallas/ejércitos/niebla y adelgazar después 4,3×
> con las optimizaciones de la Fase E3 (doc 6 §1). Lo que **sí se sostiene, y es lo que esta tabla existe para
> argumentar, es el REPARTO**: re-perfilado el 2026-09-05, la geometría de zonas —la parte paralelizable— es
> el 3,3 % del tick, del mismo orden que el 2,0 % de aquí. La conclusión de abajo no se mueve.

| Fase | Coste | % del tick | ¿Paralelizable? |
|---|---|---|---|
| Tick completo | 78.5 ms | 100% | |
| Zonas de influencia (geometría pura) | 1.6 ms | 2.0% | Sí |
| Reclamos de fuentes (lectura pura) | ~0 ms | 0.0% | Sí |
| **Resto: mutación de estado compartido** | **76.9 ms** | **98.0%** | **No** |
| Una ruta de pathfinding (por comando) | 0.1 ms | — | Sí, pero es despreciable |

**El 98% del tick es mutación de estado compartido.** Por la ley de Amdahl, paralelizar el 2% puro daría una
mejora máxima de ×1.02 — irrelevante. Y el pathfinding, que sería el candidato clásico a delegar, cuesta
0.1 ms por ruta: no es cuello de botella ni de lejos.

**Pero el usuario tiene razón en el fondo, y el motivo es estructural:**

> **El tick es una barrera global.** Hoy TODOS los asentamientos deben llegar al tick N antes de que
> cualquiera avance al N+1. Aunque fueran perfectamente independientes, esa sincronización obligatoria en
> cada frontera de tick impide repartirlos entre hilos.

En tiempo real esa barrera **desaparece**. El asentamiento A puede avanzar hasta T+5 s mientras B sigue en
T+1 s, siempre que no interactúen. Y entonces sí aparece estructura paralelizable, porque el acoplamiento
real del dominio es acotado:

- Cada asentamiento extrae de **sus propios nodos** (`reclamosDeFuentes` ya adjudica un extractor por
  yacimiento) — no compiten una vez asignados.
- Comparte estado con **su facción** (XP, nivel, reputación), no con las demás.
- Se acopla con sus **vecinos espaciales** (las zonas se recortan entre sí) y con sus **socios comerciales**.

Eso es un grafo particionable: por facción o por región, con las interacciones entre particiones como
mensajes. Es exactamente lo que hacen los MMO con *sharding* espacial. **En tiempo real, el paralelismo pasa
de imposible a posible.**

**Sin embargo, seguirá siendo innecesario — por el ritmo deliberado del juego.**

El coste no es por segundo de reloj, es por avance de simulación. A 100 asentamientos, un tick cuesta
**110,7 ms** (medido 2026-09-05; la cifra de 78,5 ms que traía esta línea era una extrapolación de agosto).
Si un tick representa ~1 minuto de tiempo de juego —y desde la Fase D representa exactamente eso—, es
**0,18 % de un núcleo**. Incluso extrapolando a 500 asentamientos sigue estando por debajo del 2 %. La carga total es minúscula porque el juego es lento
**por diseño**.

Conclusión: en tiempo real el paralelismo se vuelve *arquitectónicamente viable*, pero *económicamente
injustificado* — un solo hilo sobra para la simulación. Si algún día deja de sobrar, la partición por
facción/región es la vía correcta, y llegar ahí no requiere ninguna decisión tomada hoy.

**Dónde sí conviene el multihilo, en cualquiera de los dos modelos** — trabajo genuinamente independiente y
pesado, ninguno en el camino caliente de la simulación:

- **Varias partidas a la vez** — ya resuelto por la decisión de un proceso por partida (§7.4).
- **Simulaciones de balance en batch** (`scripts/run-batch-sim.ts`) — trivialmente paralelas, son partidas
  separadas sin estado compartido. Aquí sí hay ganancia real e inmediata.

Corrección (aportada por el usuario): la generación de mundo NO es un candidato recurrente — se ejecuta
**una sola vez por partida**, al crearla, y nunca más durante su vida. Sigue siendo un candidato legítimo a
paralelizar (es pura y pesada), pero como evento puntual de creación, no como parte del ciclo de vida del
juego en marcha. Y la exportación a Unity **no es parte del juego**: es una herramienta de `worldgen/`
(exportar el terreno generado), no algo que la partida en curso necesite — no debería figurar junto a
trabajo del propio juego.
