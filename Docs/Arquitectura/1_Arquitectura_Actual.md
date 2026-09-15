# Arquitectura actual del proyecto

| | |
|---|---|
| **Versión** | 2.5 |
| **Actualizado** | 2026-09-15 |
| **Verificado contra** | `eef7fe6`, rama `heroe-dominio` (sin mergear a `main`) — 1309 tests en 118 archivos, todos en verde |

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
| 2.5 | 2026-09-15 | Reconciliación con la rama `heroe-dominio`: modelo de **Héroe** (dueño de todo en el juego, `heroes` en el estado, `crearHeroe`, `sinHeroe`) y estado Herido; **contrato con Conquest** (`src/contratos/v1/`, capa nueva en `arquitectura.test.ts`); **batallas de Unity**, fase 1 (`session/batallas.ts`, `server/rutas/batallas.ts`, opt-in `SERVIDORES_BATALLA`, candado de batalla en el registro de comandos). Snapshot v13 → **v18** sin migración. Comandos 69 → 76, rutas HTTP ~22 → 29, tests 1222/112 → 1309/118, `constants.ts` 60/2004 → 63/1955, `engine/` 34 → 36 módulos. `ajv` pasa a dependencia de producción. |
| 2.4 | 2026-09-10 | Reconciliación de documentación operativa con el árbol de trabajo: `npm run typecheck` limpio y 1222 tests en 112 archivos en verde. Se corrigen las referencias obsoletas de arranque (`INTERVALO_TICK_MS`, `POST .../tick`) y del cliente de administración (la niebla de guerra ya no es pendiente). |
| 2.3 | 2026-09-09 | Puerto **`AlmacenDeObjetos`** (`server/almacen/`): toda la persistencia salvo respaldos —snapshots, eventos, auditoría, identidad— pasa por `leer`/`escribir`/`anexar`/`listar` por clave, con un adaptador de disco (`enDisco.ts`) como único hoy. `persistenciaPartida`/`eventosDePartida`/`auditoria`/`persistenciaIdentidad` dejan de tocar `node:fs`; `crearServidor` acepta un `almacen` inyectado (disco por defecto). Prepara el cambio limpio de proveedor (object storage, SQLite/HTTP, Postgres) para desplegar en un free tier con disco efímero. `repositorioEnDisco.ts` → `repositorioPersistente.ts`. Tests 1203/110 → 1211/111. |
| 2.2 | 2026-09-09 | Identidad de jugador con contraseña para el playtest: proveedor `clave` (nick + contraseña, hash scrypt de stdlib, secreto en `identidad.json` — sin subir `FORMATO_IDENTIDAD_VERSION`, el archivo v1 se lee con `credencialesLocales: []`), endpoint `POST /v1/registro` con `CODIGO_REGISTRO` opcional. El proceso real monta `clave` + `dev` (`proveedoresDeProceso`); `dev` queda solo para el cliente de administración en local. Tests 1189/108 → 1203/110. |
| 2.1 | 2026-09-09 | Reconciliación con el código medido: comandos 42 → 69, tests 951/95 → 1189/108, `constants.ts` 51 tablas/1786 líneas → 60/2004, `engine/` 32 módulos/11.064 líneas → 34/13.392, niebla de guerra Paso 4 (visión compartida por alianza/vasallaje) de "pendiente" a hecho, Fase E documentada (E2 auditoría/respaldos/mantenimiento y E3 métricas — completas), rutas nuevas listadas (`/admin/metricas`, `/admin/.../auditoria`, `GET`/`POST`/`DELETE /admin/.../membresias`). Además, tres cambios de persistencia de esta misma fecha: **(a)** retirada la cadena de migraciones de snapshot (sin partidas anteriores al formato vigente); **(b)** formato **v13** — el snapshot deja de guardar el terreno (se regenera de la seed) y el historial de eventos (`<gameId>.eventos.jsonl`, append-only); de ~450 KB creciendo a ~9 KB plano; **(c)** borrado `session/estado.proyectarLog` (sin llamador de producción — la consola de admin que derivaba de él vive en el repo de cliente). |
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
repositorio (2026-09-06, cifra pendiente de re-medir): consumía entonces 5 de los 9 endpoints y una fracción
de los comandos, y no usaba todavía el WebSocket, el cursor de eventos ni `GET /v1/balance`. Es brecha de
interfaz, no de backend — la matriz de `autorizacion.ts` ya admite al rol `jugador` en **75 de los 76** comandos
(el que falta, `crearFaccionNpc`, es del admin).

Por el lado del BACKEND, la Fase C está **completa** (C0–C13) y la Fase D **estructuralmente completa**
(reloj de mundo + catch-up, contrato en `instante`/`momento`, `tick` retirado del contrato — solo queda como
paso de integración interno del motor). La **Fase E** está en curso: E1 y E2 hechas (auditoría, respaldos,
mantenimiento), E3 con las métricas hechas y la moderación abierta, E4 sin empezar (ver
[3_Plan_Evolucion_Roadmap.md](3_Plan_Evolucion_Roadmap.md)). C4 dio la proyección por audiencia
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

  src/contratos/v1/           Contrato con Conquest/Unity: schema JSON, DTO y fixtures.
                              Solo mira domain y constants; lo leen session y server
```

La dirección de dependencias la congela `src/__tests__/arquitectura.test.ts`, que falla si algún import
apunta "hacia arriba". Dos invariantes tienen además su propio test en lenguaje de negocio:

- **Nada fuera de `server/` importa de `server/`.** Cambiar Fastify, el almacenamiento o el proveedor de
  identidad no debe arrastrar reglas de juego consigo.
- **`acceso/` no importa nada.** Un `Usuario` existe fuera de cualquier partida y su autenticación no sabe
  de HTTP; por eso se prueba con dobles, sin levantar infraestructura.

`session/` es el único punto que ve los dos dominios —juego y acceso— porque la autorización de comandos lo
exige: qué rol técnico tiene el actor Y qué relación de juego guarda con la entidad objetivo.

**1309 tests en 118 archivos** cubren todas las capas con código (medido 2026-09-15, `npm run test:run`). Reparto
por capa: `engine/` 58 archivos, `session/` 25, `server/` 23, `world/` 5, `acceso/` 3, `__tests__/` 2 (los de
frontera), `worldgen/` 1, `contratos/` 1. Ese último número es el punto más fino de la red: `worldgen/` es la capa con la
promesa más fuerte —semilla + `WORLDGEN_VERSION` (hoy **15**) reproducen el mapa exactamente— y la que menos
test tiene.

## Capas y responsabilidades

### Dominio: `src/domain/`

Tres archivos, ~1.130 líneas (`types.ts` 998), y es el punto de mayor fan-in de todo el repo: **cientos de
aristas de import entran aquí** desde el resto de capas (413 medidas sobre el grafo del código el 2026-09-05).

- `types.ts` — contratos de datos centrales: facciones, asentamientos, edificios, población, escuadrones,
  caravanas, mercado, relaciones políticas, recursos, nodos de mapa, ejércitos, héroes (con sus escuadrones, loadouts, inventario y equipo) y geometría básica. También
  vive aquí `WorldConfig`, porque es entidad de dominio y `worldgen/` depende de `domain/`, nunca al revés.
- `tiempo.ts` — `Instante`, `Duracion` y las conversiones del modelo temporal (ver
  [10_Modelo_Temporal.md](10_Modelo_Temporal.md)).
- `eventos.ts` — la forma de los eventos de dominio (código estable + payload tipado, hito A5).

Las entidades se identifican por cadenas (`id`, `faccionId`, `asentamientoId`, `heroeId`) — son identidades
del dominio de *juego*, distintas del `Usuario` autenticado que vive en `acceso/`. Desde el modelo de Héroe
(2026-09-14) el dueño de todo en el juego es un `Heroe` —residencia, cargos, ciudadanía, escuadrones, mando de
columnas—, uno por jugador y partida: `Heroe.jugadorId` es el `jugadorId` de la `Membresia`, y `null` en los
héroes bot de las Facciones NPC. La cadena completa es `Sesion → Usuario → Membresia → jugadorId → Heroe`.

### Configuración: `src/constants.ts`

63 tablas exportadas, 1955 líneas (medido 2026-09-15; `BALANCE_VERSION` va por 11 y `LAYOUT_VERSION` por 2): recursos, edificios,
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

**36 módulos, 12.988 líneas** (sin tests, medido 2026-09-15) — el mayor bloque de código de producción del repo.
Reglas por subsistema: fundación, expansión, zonas, construcción, trazado urbano, murallas, población,
mantenimiento, almacenamiento, mercado, comercio, caravanas, caminos, movimiento, facciones, pertenencia, cargos,
liderazgo, diplomacia, ligas, fusión, combate, tropas, ejércitos, bandidos, reputación, títulos, exploración y
memoria (las dos mitades de la niebla de guerra) y —desde 2026-09-14— el héroe (`heroe.ts`: progresión, loadouts,
Herido) y la tropa (`tropa.ts`: las escuadras viven en su héroe, y el motor militar trabaja sobre vistas de
columna y de escolta con la tropa puesta). Libre de dependencias de `session/`, `server/` o de cualquier capa de
presentación.

`avanzarSimulacion(estado, mapa, contexto)` (`engine/simulation.ts`) es el orquestador del tick: lo procesa
de forma ordenada y devuelve el nuevo estado de los subsistemas junto con eventos de dominio estructurados
(código + payload tipado, no texto — hito A5). Con batallas de Unity activas no resuelve los combates donde entra
algún héroe humano: los devuelve en `combatesPorAbrir` para que la partida los abra (ver "Contrato con Conquest y
batallas de Unity"). Hay aleatoriedad de simulación en población, combate y
bandidos vía un RNG inyectado (no `Math.random()` suelto — A3); su estado forma parte del snapshot persistido
(ver "Persistencia" más abajo).

Clasificación completa de qué es regla pura (puede recalcularse en un cliente sin motor) y qué es simulación o
entrada privilegiada (solo servidor): [9_Reglas_vs_Simulacion.md](9_Reglas_vs_Simulacion.md).

### Dominio de acceso: `src/acceso/`

Entidades externas al motor, sin ninguna dependencia (ni siquiera de `domain/`): `Usuario`, `Sesion`,
`RolTecnico`, `IdentidadVinculada`, `CredencialLocal` y `Membresia` (`acceso/tipos.ts`; el `Jugador` del diseño no
existe como entidad: su papel lo cubren la `Membresia` y el `Heroe` del juego), el servicio de autenticación por sesión
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

- `session/comandos/` — los **76** comandos de juego (`registro.ts`, contados el 2026-09-15), y `autorizacion.ts`: una matriz con una fila
  por comando (rol técnico mínimo + condición de dominio), exhaustividad garantizada en compilación. El actor
  manda una intención `{tipo, params}`; nunca ejecuta motor directamente. El actor es el héroe de la membresía,
  y todos los comandos pasan por el candado de batalla (`comandos/batalla.ts`) salvo los dos que actúan sobre la
  propia batalla.
- `session/batallas.ts` — el ciclo de las batallas de Unity (ver "Contrato con Conquest y batallas de Unity").
- `session/proyecciones/jugador.ts` — compone lo que un jugador puede ver de la partida (Fase C4: su Facción
  completa, las demás solo metadatos públicos). Desde el modelo de Héroe trae además el héroe propio completo,
  la parte pública de los héroes ajenos (`heroesVisibles`), el nombre de los compañeros de Facción y las
  batallas que se ven (`batallas`). Es la frontera de seguridad — un jugador nunca recibe el
  estado completo. Aquí es donde se aplica además la **niebla de guerra** (ver sección propia más abajo):
  lo ajeno viaja redactado (`asentamientosAvistados`, `ejercitosAvistados`), lo del mundo se filtra por lo
  visto ahora (`campamentosAvistados`) o por lo explorado (`caminosConocidos`), y `seVeAhora` se **deriva**
  en cada proyección en vez de guardarse.
- `session/canales.ts` — qué canal de WebSocket puede suscribir cada actor.
- `session/__tests__/memoriaNiebla.test.ts` — cubre los tres estados de visibilidad sobre la proyección.
- `session/npcGobernanza.ts` — automatización de facciones NPC tras el tick, separada del motor puro. Una Facción
  NPC con algo en una batalla activa no gobierna mientras dura (`comandos/avanzarFaccionesNpc.ts`).
- `session/estado.ts` — las vistas de estado completo (`vistaAdminDeEstado`) que consume la superficie admin.

### Servidor: `src/server/`

El único punto async del backend, ~4.240 líneas (12 archivos de raíz + 10 de `rutas/` + 8 de `identidad/` + el
hub). `api.ts` es la raíz de composición: monta
Fastify, CORS, WebSocket, OpenAPI, y registra las superficies bajo `/v1` (Fase C6 — versionado por prefijo de
ruta, sin alias sin versión). En total 29 rutas HTTP:

- **`/v1/sesiones`** — `POST /sesiones` (login: `Authorization: <esquema> <credencial>` — hoy `clave
  <nick>:<contraseña>` para jugadores, `dev <sujeto>` para el cliente de administración en local),
  `POST /registro` (alta de cuenta local: `{nick, clave, codigo?}`, opcionalmente tras un `CODIGO_REGISTRO`),
  `GET /sesiones/actual` (whoami). La puerta a las otras dos.
- **`/v1/admin/*`** (`rutas/admin.ts`) — gobierno de la partida como objeto: `GET`/`POST /admin/partidas`
  (descubrir/crear), `POST .../:id/tick`, `GET .../:id` (estado completo), `.../eventos?desde=` (cursor C13),
  `.../exportar`, `.../mapa/:mapaId`, `POST .../:id/comandos`, `.../auditoria` (E2), `GET`/`POST`/`DELETE
  .../:id/membresias[/:usuarioId]`, y `GET /admin/metricas` (E3, por proceso — no por partida). Exige
  `administrador_global` o una `Membresia` de administración (`administrador_partida`/`moderador`) — política
  en `acceso/rolesDePartida.ts`.
- **`/v1/jugador/*`** (`rutas/jugador.ts`) — `POST .../:id/membresia` (unirse), `GET .../:id` (proyección
  propia + niebla), `.../eventos?desde=`, `.../mapa/:mapaId`, `POST .../:id/comandos`. Un administrador **no**
  pasa este filtro: tener acceso técnico no da autoridad de jugador (doc 5). Sin héroe, `GET .../:id` responde
  `sinHeroe: true` y el único comando posible es `crearHeroe`. `GET .../:id/batallas/:battleId/asignacion`
  (`rutas/batallas.ts`) entrega a cada jugador su token para entrar en una batalla, y solo el suyo.
- **`/v1/batallas/*`** (`rutas/batallas.ts`) — la superficie del servidor de batalla de Conquest:
  `GET /batallas/pendientes`, `.../:battleId/ticket` y `.../incorporaciones`; `POST .../asignacion`, `.../inicio` y
  `.../tokens`. Ni admin ni jugador: entra con `Authorization: batalla-servidor <token>` de un servidor declarado
  en `SERVIDORES_BATALLA`, y cada mensaje se valida contra `contratos.schema.json`. Sin esa variable no entra nadie.
- **`/v1/.../tiempo-real`** (`rutas/tiempoReal.ts`, Fase C5) — WebSocket con canales suscribibles, autorizados
  por `session/canales.ts`; difunde eventos de dominio en bruto, nunca ejecuta comandos por este canal.
- **`GET /v1/balance`** (C7) y **`GET /v1/openapi.json`** — reglas públicas y el contrato publicado, sin
  autenticar a propósito, para que otros repos generen su cliente.

Piezas de soporte:

- `server/almacen/` — **puerto `AlmacenDeObjetos`** (`leer`/`escribir`/`anexar`/`listar` por clave plana) y
  su único adaptador hoy, `enDisco.ts` (un archivo por clave, `.tmp` + `rename` para la escritura atómica).
  Es la ÚNICA frontera que sabe DÓNDE viven los bytes de la persistencia (snapshots, eventos, auditoría,
  identidad). Cambiar de proveedor —disco → object storage → base de datos, para desplegar en un free tier
  sin disco persistente— es escribir otro adaptador y elegirlo en `index.ts`; ningún módulo de persistencia
  se entera. `respaldos.ts` es la excepción: sigue siendo de disco a propósito (ver abajo).
- `server/persistenciaPartida.ts` — snapshot de partida (vía el almacén): estado, tick, IDs, `config`/semilla
  del mundo y **el estado del RNG**; comprobación de versión como red de seguridad contra dos procesos
  escribiendo el mismo `gameId`. Desde el formato **v13** NO guarda el terreno (se regenera de la seed al
  cargar) ni el historial de eventos (vive en `eventosDePartida.ts`) — ver "Persistencia" más abajo.
  `FORMATO_SNAPSHOT_VERSION` (hoy **18**) rechaza cualquier otro formato; ya no hay cadena de migraciones.
  También rechaza un `worldgenVersion` o un `layoutVersion` distintos de los de la build (ver "Persistencia").
- `server/eventosDePartida.ts` — el historial de `EventoDominio` de una partida en un JSONL append-only
  hermano del snapshot (`<gameId>.eventos.jsonl`), mismo patrón que `auditoria.ts`. `anexarEventos` añade una
  línea por evento; `leerEventos` lo devuelve más-nuevo-primero para que `cargarPartida` rehidrate
  `eventosDominio`.
- `server/persistenciaIdentidad.ts` — el dominio de acceso (usuarios/sesiones/membresías/credenciales
  locales) bajo la clave `identidad.json` del mismo almacén, reescrito entero en cada mutación. Solo
  `index.ts` lo cablea; `crearServidor` trae el repositorio **en memoria** por defecto.
- `server/runnerDePartida.ts` — la pieza entre `GameSession` y el proceso real: una **cola serial** por
  partida (dos llamadas concurrentes se aplican en orden de llegada, nunca intercaladas), el ciclo
  "aplicar → persistir → confirmar" (si falla la escritura, `GameSession` vuelve atrás), idempotencia de
  comandos por `actor:idempotencyKey` (reconexión sin duplicar acciones, Fase C5), el anexado del historial de
  eventos al JSONL tras cada guardado (`eventosDePartida.ts`), caché con TTL de un minuto real de
  `preciosReferencia()` (regla de entrada privilegiada, C10), caché de geometría por frame (C10), el **reloj
  de mundo** con catch-up (Fase D5) e instrumentación por partida (E3). Recibe las `OpcionesSesion` del proceso
  (hoy, si las batallas con humanos van a Unity) y las conserva al reconstruir la sesión tras un fallo de
  escritura.
- `server/registroDePartidas.ts` — qué partidas están abiertas en este proceso (un `Map`; `abrir` lanza si ya
  lo está). Un proceso = una partida activa de facto: nada shardea ni coordina varios procesos sobre el mismo
  directorio.
- `server/auditoria.ts` (E2) — log JSONL append-only por partida, hermano del snapshot: comandos aceptados **y
  rechazados**, con las cuatro causas de rechazo tipadas (`autorizacion`/`esquema`/`dominio`/`persistencia`).
  Poda por edad. `GET /admin/.../auditoria` la sirve filtrable.
- `server/respaldos.ts` (E2) — copias fechadas en `respaldos/` (snapshot + sus hermanos `.auditoria.jsonl` y
  `.eventos.jsonl`); `restaurar` comprueba que el respaldo **carga de verdad** antes de tocar el snapshot
  vigente. Exige el servidor parado (`scripts/restaurar-partida.ts`).
- `server/mantenimiento.ts` (E2) — tarea periódica **opt-in** (`MANTENIMIENTO_INTERVALO_MS`, apagada por
  defecto): respaldar + podar respaldos por cuenta + podar auditoría por edad.
- `server/metricas.ts` (E3) — **ensambla**, no mide: junta cola/tick/ráfagas del runner, recuentos de la
  auditoría y conexiones del hub en una instantánea. Pull-only (`GET /admin/metricas`), sin exportador.
- `server/difusion/hub.ts` — registro de conexiones WebSocket y envío a las suscritas a un canal; la decisión
  de quién puede suscribirse a qué vive en `session/canales.ts`, no aquí.
- `server/identidad/` — adaptadores de los puertos de `acceso`: proveedores de identidad (`proveedorClave` —
  cuentas locales nick + contraseña, hash scrypt de stdlib, secreto en el propio repositorio de identidad; y
  `proveedorDesarrollo` — sin verificar nada, solo para el cliente de administración en local), repositorios
  en memoria y en disco, parseo de cabecera `Authorization`, y el directorio de administradores globales
  (`ADMINISTRADORES=proveedor:sujetoId` por variable de entorno — vacío por defecto, sin él nadie administra), y
  los servidores de batalla (`servidoresDeBatalla.ts`, `SERVIDORES_BATALLA=servidorId:token`, comparados en tiempo
  constante).
  `proveedoresDeProceso` monta la lista real; `proveedoresPorDefecto` (solo `dev`) es el default de
  `crearServidor` para los tests.
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

El **Paso 4** (visión compartida por alianza y vasallaje, en vivo y solo mientras la relación esté activa)
está **hecho** (`f20d64e`). **Pendiente:** solo el Paso 6 (calibración de márgenes contra la vista de
ejército).

## Contrato con Conquest y batallas de Unity

Desde el 2026-09-10 este backend coopera con `Conquest_prototype`, el cliente y servidor de batalla en Unity de
Codex (protocolo y propuestas BA-*/CQ-* en `Docs/Coordinacion/`). De ahí salen dos piezas de arquitectura:

- **`src/contratos/v1/`** — la forma del cable con Conquest: `contratos.schema.json` (draft-07, una definición por
  mensaje), `dto.ts` (su espejo en TypeScript), `fixtures.ts` → `fixtures/*.json` y el catálogo de tropas generado
  de `constants.ts`. `contratos.test.ts` comprueba que cada fixture cumple su definición y es idéntico al
  publicado. Es una capa propia en `arquitectura.test.ts`: solo mira `domain` y `constants`, y la leen `session`
  (el ticket de una batalla se congela con esta forma) y `server` (valida lo que entra).
- **El ciclo de `Batalla`** (BA-001; doc 01 §15 y doc 02 §3 de `Docs/Coordinacion/`). Un combate con algún héroe
  humano no se resuelve en el tick: se abre una `Batalla` con su ticket congelado, se juega en Unity y vuelve un
  resultado.
  - `engine/` no sabe de batallas: con `ContextoSimulacion.batallas` devuelve en `combatesPorAbrir` los combates
    con humanos que no resuelve, y hace esperar a la puerta a quien llega a una plaza en batalla.
  - `session/batallas.ts` es el ciclo: abrir (ticket, reservas en `Escuadron.reservaBatalla`, bloqueo), unirse
    (incorporaciones), cancelar, vencer plazos, y lo que manda Conquest (asignación, inicio, tokens).
    `GameSessionState.batallas` lo persiste. Lo bloqueado se hace cumplir en dos sitios: el candado del registro
    de comandos, que mira los parámetros por su nombre, y el tick, que aparta de la simulación las columnas y
    caravanas en batalla.
  - `server/rutas/batallas.ts` es la superficie de Conquest y la del token del jugador (arriba).
  - **Opt-in.** Sin `SERVIDORES_BATALLA`, `OpcionesSesion.batallasEnUnity` queda apagado y todo se resuelve con
    números, como antes. Es configuración del proceso, no estado de partida: viaja de `RegistroDePartidas` a
    `RunnerDePartida` y `GameSession`, y de ahí a `ContextoComando`.
  - El `battleId` es un UUID determinista (el `gameId` resumido más el contador de ids), porque `session` no puede
    usar aleatoriedad global (`autoridadTemporal.test.ts`).

  Estado: fase 1 hecha (abrir, bloquear, unirse, cancelar, vencer y las rutas). Falta aplicar el `BattleResult`
  (fase 2) y el canal WS `batalla/<battleId>` (fase 3). Detalle vivo en `Docs/Mecanicas a desarrollar.md` §30.

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
     (avanzarTick cierra las batallas vencidas, aparta lo que está en una activa, simula, y abre como
      batallas los combates con humanos que el motor dejó sin resolver)
  -> UN solo persist para las tres (un fallo a mitad no deja tick aplicado sin NPC resuelto)
```

El tick es la unidad de integración interna del motor (retirado del contrato hacia afuera en el cierre de la
Fase D — lo que viaja es `instante`/`momento`). Se avanza a mano vía `POST /admin/.../tick`, o solo por el
**reloj de mundo** de `RunnerDePartida` (Fase D5) si el despliegue configura `INTERVALO_TICK_MS` (**C12**,
opt-in — sin configurarlo, ninguna partida avanza sola, ni siquiera en los tests). El reloj ancla su
referencia a "ahora" al abrir la partida: **el mundo no avanza mientras el servidor está caído**, y un atraso
mayor que `MAX_TICKS_POR_PASADA` (5) se descarta en vez de ejecutarse en ráfaga. `GET /admin/partidas`
descubre qué partidas existen en disco, incluidas las que nadie ha reabierto todavía en este proceso.

## Persistencia, historial y observabilidad

- Toda la persistencia pasa por el puerto **`AlmacenDeObjetos`** (`server/almacen/`): objetos de texto por
  clave plana, con un único adaptador hoy —`enDisco.ts`, un archivo por clave, `.tmp` + `rename`—. Es el
  punto por el que se cambia de proveedor sin tocar ningún módulo de persistencia (object storage, SQLite
  sobre HTTP, Postgres) — pensado para poder desplegar en un free tier con disco efímero. `respaldos.ts` es
  la excepción deliberada: copias fechadas de archivos, un concepto de sistema de ficheros; con un almacén
  remoto el respaldo lo hace el proveedor.
- Cada partida es un snapshot JSON (`server/persistenciaPartida.ts`), no una base de datos — suficiente para
  el volumen actual; el doc 4 registra por qué no hace falta SQLite todavía (la cola serial ya elimina la
  concurrencia de escritura). Se reescribe entero en cada comando aceptado, pero desde el formato **v13**
  (2026-09-09) es **plano**: ~9 KB da igual la edad de la partida, porque ya no contiene ni el terreno ni el
  historial (ver los dos puntos siguientes). Antes eran ~450 KB a 35 000 ticks y creciendo.
- **El mapa no se guarda: se regenera.** El snapshot conserva de `state.mapa` solo `{ version, config }`;
  `cargarPartida` reconstruye el `MapaGenerado` con `generarMapa(config)` (función pura de la seed). Es lo que
  el propio `MapaGenerado.version` decía que debía pasar — el rechazo por `worldgenVersion` ya garantizaba que
  se puede. Eran ~130 KB constantes en cada escritura.
- **El historial de eventos vive en `<gameId>.eventos.jsonl`**, append-only, hermano del snapshot — mismo
  patrón que `auditoria.ts` (cuyo comentario ya decía que meter un historial en el estado era "tropezar dos
  veces con la misma piedra"). `RunnerDePartida` anexa los eventos nuevos tras cada guardado (cortando por
  `version`, no por `ResultadoComando.eventos`, para cubrir el tick completo); `cargarPartida` rehidrata
  `GameSessionState.eventosDominio` desde el archivo, así que el cursor `?desde=` funciona igual tras un
  reinicio. Un fallo de escritura del JSONL no revierte el comando (el snapshot es la fuente de verdad, esto
  el historial derivado); el cursor no avanza y el siguiente guardado reintenta ese tramo. Las lecturas de
  estado completo (`EstadoAdmin`/`ProyeccionJugador`) ya no traían `eventosDominio` desde el follow-up de C13
  (2026-09-05).
- El snapshot incluye estado, tick, RNG, IDs y `config`/semilla del mundo — no el terreno, no el historial.
- **`FORMATO_SNAPSHOT_VERSION` es 18 y no hay cadena de migraciones.** La hubo (v1→v12) mientras había
  partidas de builds anteriores que arrastrar; se retiró el 2026-09-09 al no quedar ninguna. De v14 a v18
  (2026-09-14/15: modelo de Héroe, Herido, batallas de Unity) subió sin migración, por decisión del usuario: una
  partida de un formato anterior se descarta. `cargarPartida`
  acepta solo el formato vigente y rechaza el resto con `FormatoSnapshotNoSoportadoError` — mismo criterio que
  `persistenciaIdentidad.ts` desde el principio. La próxima mecánica que cambie la forma del snapshot sube el
  número y, si en ese momento existen partidas que preservar, vuelve a añadir su función de migración puntual.
- **Dos versiones de contenido también rechazan, sin migrar:** `worldgenVersion` (`WorldgenVersionNoCoincideError`
  — el mapa se regenera de la seed y otro algoritmo daría otro mapa) y, desde BA-005 (2026-09-13),
  `layoutVersion` (`LayoutVersionNoCoincideError`, `LAYOUT_VERSION` en `constants.ts`) — la huella de cada
  edificio se deriva del catálogo vigente, así que sus `posicion` guardadas no encajarían en otra geometría.
  `balanceVersion` en cambio solo se registra.
- El estado de acceso (usuarios/sesiones/membresías/credenciales locales) se persiste bajo la clave
  `identidad.json` del mismo almacén (`server/persistenciaIdentidad.ts`). El repositorio persistente solo lo
  cablea `index.ts`; el `crearServidor` por defecto usa el **en memoria** — un embebido que no lo sustituya
  pierde todo al reiniciar.
- Los eventos de dominio son estructurados (código estable + payload tipado, A5), no mensajes de log en
  texto. Cada uno lleva la `version` de partida en la que se emitió (**C13**), y
  `GET .../eventos?desde=<version>` sirve solo los nuevos — un cliente que escucha por WebSocket ya no
  necesita releer el histórico completo para ponerse al día.
- No hay historial de línea de tiempo por tick en el servidor (lo que hacía `GameStore` en el prototipo, para
  depuración) — esa herramienta de desarrollo, si se necesita, vive del lado de un cliente de depuración, no
  del backend de producción.
- **Auditoría** (E2, `server/auditoria.ts`): un JSONL append-only por partida registra todos los comandos —
  aceptados y rechazados, con la causa — que es lo que `eventosDominio` no cuenta (quién lo pidió, qué se
  intentó y falló). **Métricas** (E3, `GET /admin/metricas`): cola, duración de tick, ráfagas de catch-up,
  recuento de comandos por resultado, conexiones y memoria, ensambladas de quien ya lleva cada número.
- **Sin logging de request**: `Fastify({ logger: false })`. La auditoría cubre comandos; lecturas y errores
  generales no dejan rastro. Tampoco hay rate limiting ni límite de tamaño de body más allá del esquema.

## Comunicación, identidad y permisos

- HTTP versionado (`/v1`) + WebSocket, con tres superficies separadas por rol (arriba). CORS desactivado por
  defecto (`ORIGENES_PERMITIDOS`, vacío = ningún origen cruzado pasa).
- `Usuario`, `Sesion` y `Membresia` están implementados (C1), con la autenticación tras un puerto
  intercambiable: sustituir el proveedor de desarrollo por uno real es un adaptador nuevo, sin tocar lo que se
  apoya en él. El `Jugador` del diseño lo cubren la `Membresia` y el `Heroe` del juego (2026-09-14): una
  membresía sin héroe solo puede crearlo (`crearHeroe`).
- Una tercera credencial, de servidor a servidor: `batalla-servidor <token>` para Conquest (`/v1/batallas/*`),
  que nunca entra por las superficies de persona. El token con que cada jugador entra en una batalla solo lo
  recoge él; ni el estado que ve el admin lo lleva (`vistaAdminDeEstado` los vacía).
- Autorización de comandos por actor/facción/asentamiento/cargo, vía la matriz de `session/comandos/autorizacion.ts`
  (C2) — exhaustiva en compilación, no una lista que se pueda olvidar actualizar.
- **Defecto corregido (hito C8, 2026-08-26):** `rolEnPartida` cortocircuitaba a `administrador_global` para
  cualquier actor con `esAdministradorGlobal`, sin comprobar si además tenía una `Membresia` real en esa
  partida — así que `alternarFaccionNpc` (la única acción que la matriz permite a un admin,
  `['jugador', 'administrador_partida']`) devolvía 403 siempre. Se invirtió el orden: la `Membresia` manda
  sobre `esAdministradorGlobal`, no al revés. Ver doc 5 y doc 3 hito C8. (`alternarFaccionNpc` se retiró el
  2026-09-14: las Facciones NPC las crea el admin con `crearFaccionNpc`, y ninguna Facción de jugador pasa a la IA.)
- **Resuelto (hito C9, 2026-08-26):** `ESQUEMA_EJECUTAR_COMANDO` tiene ahora un esquema por comando
  (`session/comandos/esquemas.ts`, `oneOf` discriminado por `tipo`) — un `params` malformado responde 400
  antes de tocar el manejador, en vez de 409 tras un `TypeError` sin capturar.
- La proyección de jugador (C4) da la Facción propia completa y del resto solo metadatos públicos, y sobre
  esa misma frontera se aplica la niebla de guerra (implementada salvo el Paso 6; ver sección propia).

## Fortalezas para una futura evolución

- Dirección de dependencias congelada por test (`arquitectura.test.ts`); `acceso/` sin dependencias y
  `session/` síncrona y sin E/S, lo que hace ambas capas triviales de probar con dobles.
- 1309 tests en 118 archivos cubren motor, sesión, acceso, servidor y contrato (medido 2026-09-15).
- El contrato con Conquest es verificable y no solo declarado: cada mensaje tiene su fixture, validado contra el
  schema en la suite, y lo que llega por `/v1/batallas/*` se valida contra ese mismo schema.
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
- El WebSocket difunde eventos en bruto, no deltas de estado aplicables — la única reacción de un cliente sin
  motor a un evento es releer, aunque desde C13 puede releer solo lo nuevo (`?desde=`) en vez del estado
  completo.
- `RunnerDePartida` mantiene el historial de eventos entero en RAM (rehidratado al cargar). ~300 KB a 35 000
  ticks — nada hoy, pero crece sin techo; el disco ya no (formato v13, JSONL append-only). El día que la RAM
  importe, acotar la cola en memoria y leer lo antiguo del archivo para un `?desde=` profundo.
- Niebla de guerra: **implementada** salvo el Paso 6 (calibración de los márgenes de visión contra la vista de
  ejército). Detalle y estado en la sección propia más abajo.
- **Un proceso, una partida activa.** `RegistroDePartidas` es un `Map` en memoria; nada shardea ni coordina
  varios procesos sobre el mismo directorio. Sin historia de escalado horizontal.
- Identidad de jugador: `proveedorClave` (nick + contraseña, hash scrypt, sin verificación de email ni
  recuperación de contraseña ni rate-limiting en el login). Suficiente para el playtest; un IdP real sería
  otro adaptador. El proveedor `dev` sigue activo en el proceso para el cliente de administración local —
  acepta cualquier sujeto sin verificar, así que la superficie de admin **no debe exponerse en público** tal
  cual. El repositorio de identidad por defecto de `crearServidor` es el **en memoria** (el proceso real usa
  el de disco).
- Persistencia: hay puerto (`AlmacenDeObjetos`) pero **solo el adaptador de disco**. Un adaptador remoto
  (object storage, SQLite/HTTP, Postgres) está por escribir — es lo que haría falta para un free tier con
  disco efímero. `respaldos.ts` sigue siendo de disco por diseño.
- Respaldos y poda **apagados por defecto** (`MANTENIMIENTO_INTERVALO_MS`): un despliegue que lo olvide no
  tiene copias.
- Producción corre TypeScript vía `tsx` directo — no hay target de build para el servidor.
- El terreno se evalúa en dos sitios: aquí y en el cliente de jugador, que ahora está en otro repositorio.
  Nada verifica automáticamente que sigan de acuerdo (ver la nota de deuda en "Generación de mundo").
- El cliente jugable completo (UI de comandos, etc.) es trabajo de un repo de interfaz aparte — fuera del
  alcance de este repo, que es solo servidor. Aquí solo queda `cliente/`, la herramienta de admin/dev, que
  todavía importa el motor por `@motor/*` (30 imports sobre 20 módulos, 11 de ellos de `engine/`) y por eso
  sigue sin cerrar el criterio de la Fase C.
- Batallas de Unity a medias: sin `POST .../resultado` una batalla abierta solo puede cancelarse o vencer (fase 2),
  y las cerradas no se podan de `GameSessionState.batallas`. La IA de una Facción NPC con algo en batalla se pausa
  entera, no por plaza (`ponytail:`), y una batalla se busca recorriendo las partidas abiertas del proceso, sin
  índice `battleId → gameId`.
- El modelo de Héroe, el contrato con Conquest y las batallas viven en la rama `heroe-dominio`: `main` sigue en el
  modelo anterior hasta que se mergee.

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
