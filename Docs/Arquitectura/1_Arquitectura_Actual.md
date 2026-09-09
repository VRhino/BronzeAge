# Arquitectura actual del proyecto

| | |
|---|---|
| **Versión** | 2.0 |
| **Actualizado** | 2026-09-05 |
| **Verificado contra** | commit `652a0aa` — 951 tests en 95 archivos, todos en verde |

> **Por qué existe este campo.** La v1.0 se escribió el 2026-08-26 y para el 2026-09-05 había derivado en
> ocho puntos concretos (número de comandos, número de tests, tamaño de `constants.ts`, estado de la niebla
> de guerra, rutas retiradas, un directorio que ya vivía en otro repositorio). Ninguno se detectó leyendo el
> documento: aparecieron al medir el código. Cuando este documento se toque, **subir la versión y la fecha**
> y anotar contra qué commit se verificó — un doc de arquitectura sin marca de verificación no se distingue
> de uno correcto hasta que alguien se apoya en él y falla.
>
> Fuente de verdad para el estado de cada hito: [3_Plan_Evolucion_Roadmap.md](3_Plan_Evolucion_Roadmap.md).

### Historial

| Versión | Fecha | Cambio |
|---|---|---|
| 2.0 | 2026-09-05 | Reconciliación con el código medido: comandos 30 → 42, tests 640/80 → 951/95, `constants.ts` 39 tablas/~1240 líneas → 51/1786, niebla de guerra de "pendiente" a Pasos 1-3 y 5 hechos, `cliente-jugador/` movido a otro repositorio (`2dfe9e7`), retirada la mención a `exportar-unity`, y `domain/` documentado como tres archivos. |
| 1.0 | 2026-08-26 | Reescritura completa para describir el backend real, tras varias sesiones en las que el documento seguía describiendo el prototipo de un solo `GameStore` en el navegador (Fase 0/A) mientras el código avanzaba hasta la Fase C. |

## Propósito y alcance

El proyecto es el backend multijugador de Bronze Age Collapse: un proceso Node.js (Fastify) que gobierna una
partida —mundo, facciones, asentamientos, economía, población, construcción, política, comercio, diplomacia y
combate— y la expone por red bajo un contrato HTTP versionado (`/v1`) más un canal WebSocket. No es un
prototipo de una sola pestaña: varios clientes externos, cada uno en su propio proceso y su propia identidad,
se conectan a la misma instancia de partida.

Este repositorio es **solo servidor** desde la Fase C0. Queda aquí un único cliente, `cliente/` (proyecto
aparte con su propio `package.json`), que es la herramienta de administración y depuración y **todavía no
cumple el criterio de cierre de la Fase C**: sigue importando el motor de este repo por un alias (`@motor/*`)
en vez de hablar solo por red. Medido el 2026-09-05: **30 sentencias de import sobre 20 módulos distintos**
del motor — 11 de ellos de `engine/`, más `session/`, `domain/`, `worldgen/`, `world/` y `constants`. La
dependencia no es simbólica: alcanza el corazón de las reglas, no solo los tipos. Ver `cliente/README.md`.

El cliente de JUGADOR ya no vive aquí: `cliente-jugador/` se movió a su propio repositorio
(**`BronzeAgeClient`**) en el commit `2dfe9e7`. Era el boilerplate del hito C11b —la prueba de que se puede
pintar el mundo sin una línea del motor— y su marcha es precisamente lo que la Fase C perseguía, no una
pérdida. Cualquier referencia a un directorio `cliente-jugador/` dentro de este repo es histórica.

Cuánto de esta superficie tiene hoy consumidor está medido en `docs/Analisis_Brecha_Backend.md` de ese
repositorio (2026-09-06): consume 5 de los 9 endpoints y **6 de los 42 comandos**, y no usa todavía el
WebSocket, el cursor de eventos ni `GET /v1/balance`. Es brecha de interfaz, no de backend — la matriz de
`autorizacion.ts` ya admite al rol `jugador` en los 42 comandos.

Por el lado del BACKEND, la Fase C está **completa** (C0–C13). C4 dio la proyección por audiencia
(`proyectarParaJugador`: un jugador nunca recibe el estado completo), y sobre esa frontera se construyó
después la niebla de guerra, que **ya no es trabajo pendiente**: ver "Niebla de guerra" más abajo.

## Vista global

```text
INFRAESTRUCTURA — cómo se sirve (sustituible sin tocar el negocio)
  src/server/                 Proceso Node: Fastify, persistencia en disco, RunnerDePartida
    server/identidad/         Adaptadores de los puertos de `acceso`: proveedor de
                              identidad, repositorio, parseo de cabeceras HTTP
          |
          v
NEGOCIO — qué debe pasar (estable aunque cambie todo lo de arriba)
  src/session/                Aplicación de partida: GameSession, comandos y su
                              autorización. Síncrona y sin E/S
  src/acceso/                 Dominio de acceso: Usuario, Sesion, Rol, Membresia y
                              los puertos que los sirven. Sin dependencias
          |
          v
  src/engine/                 Reglas y casos de cálculo de la simulación
  src/world/                  Fachada y consultas del mapa mutable
  src/worldgen/               Generación determinista del mundo
          |
          v
  src/domain/types.ts         Tipos de datos del dominio de juego
  src/constants.ts            Catálogos y parámetros de balance
```

La dirección de dependencias la congela `src/__tests__/arquitectura.test.ts`, que falla si algún import
apunta "hacia arriba". Dos invariantes tienen además su propio test en lenguaje de negocio:

- **Nada fuera de `server/` importa de `server/`.** Cambiar Fastify, el almacenamiento o el proveedor de
  identidad no debe arrastrar reglas de juego consigo.
- **`acceso/` no importa nada.** Un `Usuario` existe fuera de cualquier partida y su autenticación no sabe
  de HTTP; por eso se prueba con dobles, sin levantar infraestructura.

`session/` es el único punto que ve los dos dominios —juego y acceso— porque la autorización de comandos lo
exige: qué rol técnico tiene el actor Y qué relación de juego guarda con la entidad objetivo.

**951 tests en 95 archivos** cubren las seis capas (medido 2026-09-05, `npm run test:run`). Reparto por capa:
`engine/` 48 archivos, `session/` 22, `server/` 14, `world/` 5, `acceso/` 3, `__tests__/` 2 (los de frontera),
`worldgen/` 1. Ese último número es el punto más fino de la red: `worldgen/` es la capa con la promesa más
fuerte —semilla + `WORLDGEN_VERSION` reproducen el mapa exactamente— y la que menos test tiene.

## Capas y responsabilidades

### Dominio: `src/domain/`

Tres archivos, 904 líneas, y es el punto de mayor fan-in de todo el repo: **413 aristas de import entran
aquí** desde el resto de capas (medido sobre el grafo del código, 2026-09-05).

- `types.ts` — contratos de datos centrales: facciones, asentamientos, edificios, población, escuadrones,
  caravanas, mercado, relaciones políticas, recursos, nodos de mapa, ejércitos y geometría básica. También
  vive aquí `WorldConfig`, porque es entidad de dominio y `worldgen/` depende de `domain/`, nunca al revés.
- `tiempo.ts` — `Instante`, `Duracion` y las conversiones del modelo temporal (ver
  [10_Modelo_Temporal.md](10_Modelo_Temporal.md)).
- `eventos.ts` — la forma de los eventos de dominio (código estable + payload tipado, hito A5).

Las entidades se identifican por cadenas (`id`, `faccionId`, `asentamientoId`, `jugadorId`) — son identidades
del dominio de *juego*, distintas del `Usuario` autenticado que vive en `acceso/` (un mismo `Usuario` puede
tener un `Jugador` por partida).

### Configuración: `src/constants.ts`

51 tablas exportadas, 1786 líneas (medido 2026-09-05; `BALANCE_VERSION` va por 8): recursos, edificios,
economía, población, construcción, combate, política, mundo, murallas, visión y balance. **Servida completa y sin autenticar en `GET /v1/balance`** desde el hito **C7** (2026-08-26), con
`BALANCE_VERSION` estampada en cada partida al crearla. Sigue siendo **global al proceso** — la parte
"versionado por partida/temporada, con overrides reales" del hito queda deliberadamente sin construir, sin un
consumidor que la necesite todavía. Clasificación completa de qué puede viajar a un cliente y qué no:
[9_Reglas_vs_Simulacion.md](9_Reglas_vs_Simulacion.md).

### Generación de mundo: `src/worldgen/`

Genera un `MapaGenerado` a partir de una configuración y una semilla: elevación, fertilidad, biomas, ríos,
bosques, nodos de recurso y regiones. RNG propio, reproducible si se conserva la versión del algoritmo
(`WORLDGEN_VERSION`, hoy **15**) y la semilla. (Chokepoints existieron entre v7 y v14; eliminados por completo
en v15, 2026-08-26 — ver doc 9.)

El mundo generado es inmutable durante la partida salvo sus recursos agotables, cuyo consumo se mantiene
separado en `EstadoMapa`. Por ser función pura de la seed, se sirve como **asset cacheable** en vez de viajar
en cada respuesta — hito **C11a**, completado: `GET .../partidas/:gameId/mapa/:mapaId` con
`Cache-Control: immutable`. `elevacion`/`fertilidad` son *parámetros de ruido*, no rásteres — indibujable sin
código de evaluación — pero ese código es T2a (doc 9: "el terreno lo ven todos"), así que **C11b** se resolvió
sin rasterizar nada en el servidor: el cliente de jugador lleva su propia copia del evaluador y recalcula.

> **Deuda conocida, anotada el 2026-09-05.** Esa "propia copia" es hoy una **segunda implementación** del
> mismo cálculo, y desde que el cliente de jugador se mudó a otro repositorio (`2dfe9e7`) las dos ya no
> comparten ni siquiera árbol de fuentes. La promesa de `WORLDGEN_VERSION` es que semilla y versión
> reproducen el mapa **exactamente**; con dos implementaciones esa promesa depende de que nadie toque una sin
> tocar la otra, y nada lo verifica automáticamente. No es urgente —el terreno es T2a, lo ven todos— pero es
> el punto donde una divergencia silenciosa haría más daño.

### Mundo y consultas espaciales: `src/world/`

`MapaGenerado` y `EstadoMapa` son datos serializables. La clase `Mapa` es una fachada en memoria que construye
índices y ofrece consultas espaciales, stock de nodos, extracción y regeneración. También incluye geometría y
rutas. (El export de terreno a Unity Terrain, `exportUnity.ts`, se retiró: era un experimento de herramienta
de administración —nunca parte del juego— que no llegó a usarse y dejaba un test lento e intermitente
lastrando la suite.)

`world/poligonos.ts` resuelve la unión de siluetas que se solapan. Sus dos consumidores son de presentación:
`Mapa.contornosBosques()` y `engine/zones.ts` (fusión de zonas de influencia por facción).

### Motor: `src/engine/`

**32 módulos, 11.064 líneas** — el 44 % del código de producción del repo. Reglas por subsistema: fundación,
expansión, zonas, construcción, trazado urbano, murallas, población, mantenimiento, almacenamiento, mercado,
comercio, caravanas, caminos, movimiento, facciones, pertenencia, cargos, liderazgo, diplomacia, ligas, fusión,
combate, tropas, ejércitos, bandidos, reputación, títulos, y —desde 2026-09-04— exploración y memoria (las dos
mitades de la niebla de guerra). Libre de dependencias de `session/`, `server/` o de cualquier capa de
presentación.

`avanzarSimulacion(estado, mapa, tickActual)` (`engine/simulation.ts`) es el orquestador del tick: lo procesa
de forma ordenada y devuelve el nuevo estado de los subsistemas junto con eventos de dominio estructurados
(código + payload tipado, no texto — hito A5). Hay aleatoriedad de simulación en población, combate y
bandidos vía un RNG inyectado (no `Math.random()` suelto — A3); su estado forma parte del snapshot persistido
(ver "Persistencia" más abajo).

Clasificación completa de qué es regla pura (puede recalcularse en un cliente sin motor) y qué es simulación o
entrada privilegiada (solo servidor): [9_Reglas_vs_Simulacion.md](9_Reglas_vs_Simulacion.md).

### Dominio de acceso: `src/acceso/`

Entidades externas al motor, sin ninguna dependencia (ni siquiera de `domain/`): `Usuario`, `Sesion`, `Rol`,
`Jugador`, `Partida`, `Membresia` (`acceso/tipos.ts`), el servicio de autenticación por sesión
(`servicioAutenticacion.ts`), el registro de proveedores de identidad intercambiable
(`proveedorIdentidad.ts`) y la política de qué puede hacer cada rol técnico (`rolesDePartida.ts`:
`rolEnPartida`, `puedeAdministrar`, `puedeJugar`, `puedeCrearPartida`, `puedeDescartarPartida`).

Una `Membresia` es **una por (usuarioId, gameId)** — un Usuario no puede unirse dos veces a la misma partida
(409 en el intento). `esAdministradorGlobal` es un flag **ortogonal**, no una Membresia: un administrador
técnico no es por eso un jugador dentro de la partida (doc 5, "Diferencia entre rol técnico y cargo de
juego").

### Aplicación de partida: `src/session/`

`GameSession` (`gameSession.ts`) reemplaza lo que hacía `GameStore` en el prototipo: es el dueño del
`GameState` de una partida, deliberadamente **síncrona y sin E/S** (ninguna llamada a disco, red o reloj sin
que se lo pasen por parámetro) — así es fácil de probar y la async vive en la capa de encima (`server/`).

- `session/comandos/` — los **42** comandos de juego (`registro.ts`, contados el 2026-09-05), y `autorizacion.ts`: una matriz con una fila
  por comando (rol técnico mínimo + condición de dominio), exhaustividad garantizada en compilación. El actor
  manda una intención `{tipo, params}`; nunca ejecuta motor directamente.
- `session/proyecciones/jugador.ts` — compone lo que un jugador puede ver de la partida (Fase C4: su Facción
  completa, las demás solo metadatos públicos). Es la frontera de seguridad — un jugador nunca recibe el
  estado completo. Aquí es donde se aplica además la **niebla de guerra** (ver sección propia más abajo):
  lo ajeno viaja redactado (`asentamientosAvistados`, `ejercitosAvistados`), lo del mundo se filtra por lo
  visto ahora (`campamentosAvistados`) o por lo explorado (`caminosConocidos`), y `seVeAhora` se **deriva**
  en cada proyección en vez de guardarse.
- `session/canales.ts` — qué canal de WebSocket puede suscribir cada actor.
- `session/__tests__/memoriaNiebla.test.ts` — cubre los tres estados de visibilidad sobre la proyección.
- `session/npcGobernanza.ts` — automatización de facciones NPC tras el tick, separada del motor puro.
- `session/estado.ts` — las vistas de estado completo (`vistaAdminDeEstado`) que consume la superficie admin.

### Servidor: `src/server/`

El único punto async del backend. `api.ts` es la raíz de composición: monta Fastify, CORS, WebSocket, OpenAPI,
y registra tres superficies bajo `/v1` (Fase C6 — versionado por prefijo de ruta, sin alias sin versión):

- **`/v1/sesiones`** — login (proveedor de desarrollo hoy, `Authorization: dev <sujeto>`) y whoami. La puerta
  a las otras dos.
- **`/v1/admin/*`** (`rutas/admin.ts`) — gobierno de la partida como objeto: crear/reabrir, avanzar tick, leer
  estado completo. Exige `administrador_global` o una `Membresia` de administración (`administrador_partida`/
  `moderador`) en esa partida — política en `acceso/rolesDePartida.ts`.
- **`/v1/jugador/*`** (`rutas/jugador.ts`) — unirse (crea la `Membresia`), leer la proyección propia, ejecutar
  comandos. Un administrador **no** pasa este filtro: tener acceso técnico no da autoridad de jugador (doc 5).
- **`/v1/.../tiempo-real`** (`rutas/tiempoReal.ts`, Fase C5) — WebSocket con canales suscribibles, autorizados
  por `session/canales.ts`; difunde eventos de dominio, nunca ejecuta comandos por este canal.
- **`GET /v1/openapi.json`** — el contrato publicado, sin autenticar a propósito, para que otros repos generen
  su cliente.

Piezas de soporte:

- `server/identidad/` — adaptadores de los puertos de `acceso`: proveedor de desarrollo, repositorio en
  memoria, parseo de cabecera `Authorization`, y el directorio de administradores globales
  (`ADMINISTRADORES=proveedor:sujetoId` por variable de entorno — vacío por defecto, sin él nadie administra).
- `server/persistenciaPartida.ts` — snapshot de partida a disco: `PartidaExportada` completa (estado, tick,
  eventos, mapa/yacimientos, IDs y **el estado del RNG**), escritura atómica (`.tmp` + `rename`, nunca un
  archivo a medias), con comprobación de versión como red de seguridad contra dos procesos escribiendo el
  mismo `gameId`.
- `server/runnerDePartida.ts` — la pieza entre `GameSession` y el proceso real: una **cola serial** por
  partida (dos llamadas concurrentes se aplican en orden de llegada, nunca intercaladas), el ciclo
  "aplicar → persistir → confirmar" (si falla la escritura, `GameSession` vuelve atrás), idempotencia de
  comandos por `actor:idempotencyKey` (reconexión sin duplicar acciones, Fase C5), caché con TTL de un minuto
  real de `preciosReferencia()` (regla de entrada privilegiada, C10), y un scheduler opcional de ticks
  automáticos.
- `server/registroDePartidas.ts` — qué partidas están abiertas en este proceso.
- `server/difusion/hub.ts` — registro de conexiones WebSocket y envío a las suscritas a un canal; la decisión
  de quién puede suscribirse a qué vive en `session/canales.ts`, no aquí.
- `server/openapi.ts` — configuración de `@fastify/swagger` para el contrato publicado.

## Niebla de guerra

Estaba anotada como mecánica pendiente hasta la v1.0 de este documento; se implementó entre el 2026-09-03 y el
2026-09-05. Las **decisiones y el plan de los 6 pasos** viven en
[`Consideraciones/Niebla_De_Guerra_Definicion.md`](../../Consideraciones/Niebla_De_Guerra_Definicion.md) y las
**reglas de juego** en Doc 5.12.7; aquí solo se registra dónde se apoya en la arquitectura.

Sale de una sola regla —*lo que alcanzas a ver este tick queda grabado*— y produce tres estados:

| Estado | Cómo se representa |
|---|---|
| 1. Nunca visto | La celda **no está** en `exploracion` |
| 2. Lo vi antes | Está en `exploracion`, y de una plaza queda su última `FichaConocida` |
| 3. Lo estoy viendo | **No se guarda**: se deriva en cada proyección (`seVeAhora`) |

Que el estado 3 no se persista es deliberado y es la decisión de diseño que sostiene el resto: guardar lo
derivable es la clase de error que este proyecto lleva evitando desde el principio.

Dónde vive:

- `engine/exploracion.ts` — lo explorado **como área**: una rejilla de un bit por celda serializada en
  hexadecimal. Sobre el mundo de 2000 con celdas de 25 son 6.400 celdas = 800 bytes por Facción, frente a los
  ~45 KB que costaría una lista de pares `"col,row"`. Responde en O(1) y tiene tamaño máximo fijo.
- `engine/memoria.ts` — lo que cada Facción **recuerda**, con `conocidoEn` para que la información vieja se
  delate sola ("última información: hace 3 horas") en vez de caducar a un plazo que nadie ha medido.
- `session/proyecciones/jugador.ts` — donde se aplica al proyectar.

Es **por Facción y no por jugador**, por coherencia antes que por coste: la proyección ya enseña la Facción
propia completa a cualquiera de sus ciudadanos, así que un miembro que no supiera lo que otro ya exploró sería
incoherente con todo lo demás. Que además sea unas cinco veces más barato de guardar es un extra.

Una partida guardada antes de esta mecánica **no necesita migración**: una Facción sin registro vale cadena
vacía, que es exactamente "nada explorado".

**Pendiente:** Paso 4 (visión compartida por alianza, en vivo y solo mientras la alianza esté activa) y
Paso 6 (calibración de márgenes contra la vista de ejército).

## Estado de partida y ciclo de un tick

```text
Comando de jugador o admin (HTTP)
  -> RunnerDePartida.ejecutar(manejador, params, actor, idempotencyKey?)
  -> encolar en la cola serial de esa partida
  -> GameSession.ejecutar(...) (síncrono: valida, autoriza, muta estado, genera eventos)
  -> persistir snapshot en disco (o revertir GameSession si falla la escritura)
  -> resultado devuelto al cliente HTTP

POST /admin/.../tick  (o el scheduler opcional de RunnerDePartida)
  -> RunnerDePartida.avanzarTick()
  -> misma cola serial: GameSession.avanzarTick() -> avanzarAutoComercio() -> avanzarFaccionesNpc()
  -> UN solo persist para las tres (un fallo a mitad no deja tick aplicado sin NPC resuelto)
```

Los ticks siguen siendo la unidad de simulación (Fase D no ha empezado): se avanzan a mano vía
`POST /admin/.../tick`, o solos si el despliegue configura `INTERVALO_TICK_MS` (**C12**, opt-in — sin
configurarlo, ninguna partida avanza sola, ni siquiera en los tests). `GET /admin/partidas` descubre qué
partidas existen en disco, incluidas las que nadie ha reabierto todavía en este proceso.

## Persistencia, historial y observabilidad

- Cada partida es un snapshot JSON en disco (`server/persistenciaPartida.ts`), no una base de datos —
  suficiente para el volumen actual; el doc 4 registra por qué no hace falta SQLite todavía (la cola serial ya
  elimina la concurrencia de escritura).
- El snapshot incluye estado completo, tick, RNG, IDs, configuración/semilla del mundo y eventos de dominio —
  no solo el estado "de superficie".
- Los eventos de dominio son estructurados (código estable + payload tipado, A5), no mensajes de log en
  texto. Cada uno lleva la `version` de partida en la que se emitió (**C13**), y
  `GET .../eventos?desde=<version>` sirve solo los nuevos — un cliente que escucha por WebSocket ya no
  necesita releer el histórico completo para ponerse al día. Las lecturas de estado completo siguen trayendo
  `eventosDominio` entero, sin cursor todavía (crecen sin techo dentro de una partida larga: 2 → 41 → 102 KB
  entre los ticks 0 y 200 con 4 facciones) — migrar esas lecturas exige un cliente real usando el cursor
  primero, que hoy no existe.
- No hay historial de línea de tiempo por tick en el servidor (lo que hacía `GameStore` en el prototipo, para
  depuración) — esa herramienta de desarrollo, si se necesita, vive del lado de un cliente de depuración, no
  del backend de producción.

## Comunicación, identidad y permisos

- HTTP versionado (`/v1`) + WebSocket, con tres superficies separadas por rol (arriba). CORS desactivado por
  defecto (`ORIGENES_PERMITIDOS`, vacío = ningún origen cruzado pasa).
- `Usuario`, `Sesion`, `Jugador`, `Membresia` están implementados (C1), con la autenticación tras un puerto
  intercambiable: sustituir el proveedor de desarrollo por uno real es un adaptador nuevo, sin tocar lo que se
  apoya en él.
- Autorización de comandos por actor/facción/asentamiento/cargo, vía la matriz de `session/comandos/autorizacion.ts`
  (C2) — exhaustiva en compilación, no una lista que se pueda olvidar actualizar.
- **Defecto corregido (hito C8, 2026-08-26):** `rolEnPartida` cortocircuitaba a `administrador_global` para
  cualquier actor con `esAdministradorGlobal`, sin comprobar si además tenía una `Membresia` real en esa
  partida — así que `alternarFaccionNpc` (la única acción que la matriz permite a un admin,
  `['jugador', 'administrador_partida']`) devolvía 403 siempre. Se invirtió el orden: la `Membresia` manda
  sobre `esAdministradorGlobal`, no al revés. Ver doc 5 y doc 3 hito C8.
- **Resuelto (hito C9, 2026-08-26):** `ESQUEMA_EJECUTAR_COMANDO` tiene ahora un esquema por comando
  (`session/comandos/esquemas.ts`, `oneOf` discriminado por `tipo`) — un `params` malformado responde 400
  antes de tocar el manejador, en vez de 409 tras un `TypeError` sin capturar.
- La proyección de jugador (C4) da la Facción propia completa y del resto solo metadatos públicos, y sobre
  esa misma frontera se aplica la niebla de guerra (Pasos 1-3 y 5 hechos; ver sección propia).

## Fortalezas para una futura evolución

- Dirección de dependencias congelada por test (`arquitectura.test.ts`); `acceso/` sin dependencias y
  `session/` síncrona y sin E/S, lo que hace ambas capas triviales de probar con dobles.
- 951 tests en 95 archivos cubren motor, sesión, acceso y servidor (medido 2026-09-05).
- El mundo generado tiene semilla y versión, y se sirve como asset inmutable cacheado (C11a) en vez de viajar
  en cada respuesta.
- El snapshot de partida persiste el estado del RNG: una partida recargada continúa siendo determinista, no
  solo su estado de superficie.
- Escritura de snapshot atómica; cola serial + idempotencia de comandos hacen segura la reconexión de
  cliente sin duplicar acciones.
- Contrato publicado (`/v1/openapi.json`) para que un cliente externo genere su propio cliente tipado.
- Autorización de comandos exhaustiva en compilación (matriz, no lista suelta).
- Esquema de `params` exhaustivo en compilación por comando (**C9**): un `oneOf` publicado en el OpenAPI, no
  solo una validación interna — un cliente externo puede generar formularios correctos sin adivinar la forma.
- Geometría por frame servida y filtrada por audiencia (**C10**): zonas de influencia, fusión por Facción y
  trazado urbano ya no exigen que el cliente importe `engine/zones`/`engine/trazado` para dibujar el mapa — el
  admin las recibe de todos los asentamientos, un jugador solo de los suyos.
- Existió y existe un cliente real sin ninguna línea del motor (**C11b**) que pinta el terreno — prueba en
  vivo de que un cliente puede dibujar el mundo hablando solo por red. Desde `2dfe9e7` vive en su propio
  repositorio, que es exactamente donde la Fase C quería que acabara.
- Partidas descubribles (**C12**, `GET /admin/partidas`) y exportables (`GET .../partidas/:gameId/exportar`)
  por HTTP — ya no hace falta el navegador para ninguna de las dos. (La ruta hermana `exportar-unity` se
  retiró junto con `world/exportUnity.ts`; verificado el 2026-09-05, no queda rastro de ninguna de las dos
  en `src/`.)
- Cursor incremental de eventos (**C13**, `GET .../eventos?desde=`), filtrado por audiencia en la superficie
  de jugador: un cliente puede ponerse al día sin releer el histórico completo.

## Limitaciones actuales

- Balance (`constants.ts`) servido (**C7**), pero global al proceso — sin overrides por partida/temporada.
- Fuente de ticks opt-in (**C12**): sin configurar `INTERVALO_TICK_MS`, ninguna partida avanza sola. El
  intervalo, si se configura, es un placeholder de ritmo de juego sin decisión de balance tomada.
- Las lecturas de estado completo (`EstadoAdmin`/`ProyeccionJugador`) siguen trayendo `eventosDominio` entero,
  sin el cursor de **C13** — crecen sin techo dentro de una partida larga. Migrarlas exige un cliente real que
  ya use el cursor, que hoy no existe. El WebSocket sigue difundiendo eventos en bruto, no deltas de estado
  aplicables — la única reacción de un cliente sin motor a un evento es releer, aunque ahora puede releer solo
  lo nuevo en vez del estado completo.
- Niebla de guerra: **implementada** salvo dos piezas — el Paso 4 (visión compartida por alianza, en vivo y
  solo mientras la alianza esté activa) y el Paso 6 (calibración de los márgenes de visión). Detalle y estado
  en la sección propia más abajo.
- El terreno se evalúa en dos sitios: aquí y en el cliente de jugador, que ahora está en otro repositorio.
  Nada verifica automáticamente que sigan de acuerdo (ver la nota de deuda en "Generación de mundo").
- El cliente jugable completo (UI de comandos, etc.) es trabajo de un repo de interfaz aparte — fuera del
  alcance de este repo, que es solo servidor. Aquí solo queda `cliente/`, la herramienta de admin/dev, que
  todavía importa el motor por `@motor/*` (30 imports sobre 20 módulos, 11 de ellos de `engine/`) y por eso
  sigue sin cerrar el criterio de la Fase C.

## Decisión de evolución adoptada

La conversión temporal completa no se realiza como primer paso. La etapa actual construye el backend
multijugador sobre el motor de ticks existente: servidor, persistencia, comunicación, identidad y permisos
primero (Fases B y C), tiempo real después (Fase D).

Durante esta etapa:

- Cada partida es una instancia backend dedicada (`RunnerDePartida` + snapshot en disco).
- Todos los actores conectados a esa instancia comparten el mismo estado y tick.
- El servidor sigue siendo quien avanza y resuelve los ticks; ningún cliente ejecuta motor por autoridad.
- Los frontends (jugador, administración) son sustituibles sin trasladar la autoridad de la partida al
  navegador — la API es el producto, no el cliente. Construir esos frontends está fuera del alcance de este
  repo (solo servidor).

La conversión posterior será de ticks discretos a tiempo real total (Fase D). Desde el principio quedan
separados: el reloj del servidor, el tiempo de simulación, la unidad interna provisional del motor (`tick`),
el scheduler que decide cuándo avanzar, y la representación temporal que recibe cada frontend — ninguna API,
persistencia o DTO nuevo debe tratar el tick como contrato definitivo (doc 3, regla que gobierna todo el
roadmap).
