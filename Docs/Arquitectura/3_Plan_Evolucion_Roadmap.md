# Roadmap de evolución: backend multijugador y conversión temporal

Este documento es el mapa de alto nivel. Se marca un hito `[x]` solo cuando está
verificado (código, tests o documentación correspondiente), no cuando está "casi
listo". El desglose en tareas pequeñas y accionables vive en
[4_Plan_Evolucion_Tareas.md](4_Plan_Evolucion_Tareas.md); ese es el archivo que se
actualiza con más frecuencia. Este archivo se actualiza cuando se cierra un hito o
una fase completa.

Contexto y justificación de cada fase: ver
[2_Estudio_Evolucion_Backend_Multifrontend.md](2_Estudio_Evolucion_Backend_Multifrontend.md).
Decisiones de escala, conexiones y visibilidad: ver
[6_Sincronizacion_Visibilidad_y_Escala.md](6_Sincronizacion_Visibilidad_y_Escala.md).
Qué puede salir del servidor y qué no, módulo por módulo: ver
[9_Reglas_vs_Simulacion.md](9_Reglas_vs_Simulacion.md) — referencia transversal, la ejecutan C7, C10 y C11.

**Objetivo de escala (acordado 2026-08-24): mínimo 500 jugadores conectados
simultáneamente en una misma partida.** Condiciona Fases B y C — el cuello de
botella medido es la CPU del tick (~1.9 s a 500 asentamientos), no la red ni la
persistencia; ver documento 6 para las mediciones y sus consecuencias.

Orden de fases (no reordenar sin justificar por qué el nuevo orden reduce riesgo,
no solo por conveniencia):

```text
A. Núcleo reutilizable
B. Backend provisional con ticks
C. Multijugador sobre ticks
D. Conversión temporal total
E. Operación persistente
```

Regla que gobierna todo el roadmap: **el tick es un detalle provisional**. Ninguna
API, persistencia o DTO nuevo debe tratarlo como contrato definitivo. Ver sección
"Contratos que deben ser independientes de ticks desde el inicio" en el documento 2.

---

## Fase A — Núcleo reutilizable

Objetivo: dejar el motor y el estado en condiciones de ser gobernados por un
servidor, sin construir aún el servidor.

- [x] A1. Suite de pruebas estable (sin fallos preexistentes antes de empezar a tocar código de extracción)
- [x] A2. Motor (`engine/`) verificado sin dependencias de `app/`, `ui/` o `main.ts`
- [x] A3. RNG de simulación inyectable (sin `Math.random()` suelto en reglas) — el estado interno del RNG aún no es serializable/persistible, diferido a propósito a B3 (ver detalle en el doc de tareas)
- [x] A4. Generador de IDs desacoplado de `GameStore` (utilizable desde un contexto de partida)
- [x] A5. Eventos de dominio estructurados, separados de los mensajes de log en texto — completada 2026-08-25: los 13 subsistemas migrados, cada uno con su propio `codigo` estable y `payload` tipado (ya no `'legado'`). Ver [4_Plan_Evolucion_Tareas.md](4_Plan_Evolucion_Tareas.md#-marcador-de-progreso--migración-por-subsistema-13--13-migrados-100)
- [x] A6. Contratos mínimos documentados: `Usuario`, `Sesion`, `Jugador`, `Rol`, `Partida`, `Membresia` (solo diseño, sin implementar aún) — ver [5_Contratos_Identidad_Permisos.md](5_Contratos_Identidad_Permisos.md)

## Fase B — Backend provisional con ticks

Objetivo: una partida corriendo en un proceso Node.js dedicado, con persistencia,
gobernada por una única `GameSession`, todavía sin multijugador real.

> **Marcada `[x]` retroactivamente (2026-08-26).** Las cinco quedaron satisfechas como efecto colateral de
> implementar C1–C3 (2026-08-25) — nunca se marcaron en su momento porque el trabajo no se hizo *como* Fase B
> explícita, sino como la base que C necesitaba. No hay commit único de cierre que citar; la evidencia es el
> código listado en cada ítem, verificado el 2026-08-26.

- [x] B1. `GameSession` (o `GameApplicationService`) que reemplaza las responsabilidades de casos de uso de `GameStore` — `session/gameSession.ts`
- [x] B2. Proceso backend Node.js + repositorio de partida — `server/index.ts`, `server/api.ts` (Fastify), `server/registroDePartidas.ts`
- [x] B3. Persistencia de estado, tick, RNG, IDs, configuración y eventos (snapshots como estrategia inicial) — `server/persistenciaPartida.ts`, escritura atómica (`.tmp`+`rename`), incluye el estado del RNG en `PartidaExportada`
- [x] B4. API administrativa (HTTP) sobre esa partida — `server/rutas/admin.ts`
- [x] B5. La interfaz actual (`main.ts`) migrada a cliente remoto de esa API, en vez de llamar a `GameStore` local — `cliente/src/app/apiCliente.ts`

## Fase C — Multijugador sobre ticks (**solo servidor**)

Objetivo: dejar el backend listo para que varios jugadores y un administrador —cada uno desde
**su propio repositorio de cliente**— se conecten a la misma partida con autorización real.

> **Replanteada 2026-08-25.** Este repositorio pasa a ser **solo servidor**: el cliente de jugador vive en
> otro repositorio y el de administración ya existe fuera. Consecuencias sobre el plan original:
>
> - **Sale de alcance** "frontend de jugador" (era C5). El cliente de navegador que quedaba aquí se extrajo a
>   `cliente/` —proyecto aparte, con su `package.json`/`tsconfig`/`vite.config`— listo para inicializar su
>   propio repositorio. Ver `cliente/README.md`.
> - **Sube de prioridad** la proyección por audiencia: con un cliente externo, `GET /partidas/:gameId`
>   (estado completo, todas las facciones) deja de ser "sin proyección todavía" y pasa a ser una fuga. Las
>   proyecciones son frontera de seguridad, no refinamiento.
> - **Entra lo que no estaba**: CORS (hoy lo evita el proxy de Vite), versionado del contrato y publicación
>   del mismo como OpenAPI para que los otros repos generen su cliente.

> **Ampliada 2026-08-26 tras el diagnóstico de aislamiento del cliente.** C0–C6 dieron por buena una premisa
> falsa: que el cliente extraído podía moverse a su repositorio "repuntando el alias `@motor/*`". No puede.
> Ese alias existe porque el cliente **calcula en el navegador 26 consultas derivadas que el servidor no
> expone**, y porque el terreno que el servidor manda son *parámetros de ruido* inservibles sin el código de
> `worldgen/`. La decisión del usuario (2026-08-26) es que el cliente **no dependa del código del motor de
> ninguna forma**: no lo importa, no lo ejecuta. Eso convierte lo que era "una costura a repuntar" en seis
> hitos de servidor (C8–C13) y añade un criterio de cierre a la fase. **Matiz del mismo día, no contradicción**:
> esto no significa "cero derivaciones en el cliente" — [9_Reglas_vs_Simulacion.md](9_Reglas_vs_Simulacion.md)
> fija que las puras de entrada propia (T2a) se reimplementan como código del cliente a propósito, para evitar
> un viaje de red por *tooltip*. Diagnóstico completo, con la evidencia de cada hallazgo, en
> [4_Plan_Evolucion_Tareas.md](4_Plan_Evolucion_Tareas.md#diagnóstico-de-aislamiento-del-cliente-2026-08-26).
>
> Consecuencia inmediata ya aplicada: **`cliente/src/lab/` y `laboratorio.html` eliminados** (2026-08-26). El
> laboratorio visual del motor no era un cliente —ejecutaba el motor en el navegador, sin tocar la API— así
> que no podía aislarse por red ni por definición acompañar al cliente a otro repositorio. Queda en el
> historial de git por si se rescata como herramienta de desarrollo de ESTE repo.

- [x] C0. Cliente de navegador extraído a `cliente/`; el repo queda como backend puro (sin Vite ni capas `app`/`ui`/`main`/`lab`)
- [x] C1. Usuarios, sesiones y membresías implementados (2026-08-25), con la autenticación tras un puerto intercambiable: sustituir el proveedor de desarrollo por uno real es escribir un adaptador y darlo de alta, sin tocar nada de lo que se apoya en él
- [x] C2. Autorización de comandos por actor / facción / asentamiento / cargo (2026-08-25): matriz con una fila por comando, exhaustividad garantizada en compilación. El esquema de `params` por comando y la `idempotencyKey` siguen pendientes (C6 y reconexión de C5)
- [x] C3. Superficies separadas: `/admin/*` y `/jugador/*` con requisitos de rol distintos — completada 2026-08-25. Cierra de paso el agujero que quedaba de B4: crear partida, tick y estado ya no son endpoints abiertos. `/jugador/*` sin lectura de estado a propósito, hasta que existan las proyecciones de C4
- [ ] C4. Proyecciones de estado por audiencia — **Slice 1 completado 2026-08-26** (jugador ve su Facción completa, las demás solo metadatos públicos, `GET /jugador/partidas/:gameId`). Slice 2 pendiente: `ConocimientoJugador` y "último conocido" necesitan un radio de visualización (balance de juego) no definido en ningún doc de este repo
- [x] C5. WebSocket único con canales, suscripciones autorizadas y reconexión sin duplicar comandos — completada 2026-08-26. Difusión de eventos de dominio (no de comandos: el cliente los ejecuta por HTTP igual que antes)
- [x] C6. Contrato publicable: CORS, versionado de API y OpenAPI generado desde los esquemas de Fastify — completada 2026-08-26. Todo bajo `/v1`; `GET /v1/openapi.json` sin autenticar; respuesta de comando autosuficiente en `/jugador/*` (incluye la proyección propia, ya no hace falta un `GET` aparte)
- [x] C7. Balance versionado y servido — **completada 2026-08-26**: `GET /v1/balance` (`server/rutas/balance.ts`), sin autenticar (regla pública, no estado de partida, doc 9 T1), las 39 tablas completas de `constants.ts` sin lista de exclusión (decisión del usuario: publicarlas todas). `BALANCE_VERSION` (`constants.ts`) se estampa en `PartidaExportada.balanceVersion` al exportar una partida, como registro de qué balance corría — sin rechazo al cargar si cambia, a diferencia de `worldgenVersion` (el balance no hace falta para reconstruir el snapshot, solo cambia qué reglas rigen desde ahora). **Alcance NO cubierto, a propósito**: "versionado por partida/temporada" con overrides reales — hoy sigue siendo un único valor de proceso, no hay mecanismo para que dos partidas corran versiones de balance distintas a la vez; el panel que mutaba `constants.ts` en caliente (`app/balanceConfig.ts`) se eliminó en Fase B sin reemplazo y sigue sin dueño. Absorbe el grupo "de tabla" de C10 (`capFundacion`, `cupoVivienda`, `slotsPoliticaDisponibles`, `nivelFaccionInfo`, `CATALOGOS`): con el balance servido, un cliente sin motor ya puede resolverlas por *lookup* sin viaje de red — pendiente de que exista ese cliente (C8-C13)
- [x] C8. Superficie y rol del cliente de administración — **completada 2026-08-26**. Replanteo: el administrador observa, no interactúa como jugador (decisión del usuario) — los 27 comandos de rol `jugador` no debían estar expuestos en el cliente de administración, no era un permiso que faltara. Interfaz corregida: paneles convertidos a solo consulta, ninguna acción de jugador queda en la UI. **Causa raíz corregida el mismo día**: `rolEnPartida` (`acceso/rolesDePartida.ts`) cortocircuitaba a `administrador_global` para cualquier actor con acceso técnico global, incluso con una `Membresia` `administrador_partida` real en esa partida (la que `otorgarAdministracion()` concede a quien la crea) — invertido el orden: la Membresia manda, `esAdministradorGlobal` es el *fallback* solo si no hay ninguna. `alternarFaccionNpc` —la única acción legítima de administrador (doc 5: "sin restricción si es admin")— ya no devuelve 403: verificado en vivo (servidor real, admin crea partida, jugador funda Facción, admin la cede al NPC — `resultado.ok: true`) y con dos regresiones nuevas en `server/__tests__/api.test.ts`. Dos tests preexistentes **codificaban el bug como comportamiento esperado** (`acceso/__tests__/rolesDePartida.test.ts`, `server/__tests__/api.test.ts`) — corregidos junto con el fix, no solo el código
- [ ] C9. Contrato de comandos completo: esquema de `params` por cada uno de los 30 comandos. Hoy `ESQUEMA_EJECUTAR_COMANDO` declara `params: {}` — el contrato publicado tiene el agujero justo donde un cliente externo lo necesita, y un `params` malformado revienta dentro del manejador y sale como 409 en vez de 400. Arrastrado desde C2
- [ ] C10. Consultas derivadas servidas por el backend: la migración que [8_Triaje_Consultas.md](8_Triaje_Consultas.md) ya trió. **`calcularPrecioReferencia` hecha** (2026-08-26, `RunnerDePartida.preciosReferencia()`). **Chokepoints y `evaluarViabilidadFundacion` (cliente) eliminados** ese mismo día, no migrados — decisión del usuario: la mecánica de chokepoints no aportaba, y el aviso de *hover* de viabilidad se quita de la interfaz (la función se queda solo para el NPC de gobernanza). Queda **un grupo**: la **geometría por frame** (zonas, fusión, trazado), que viaja precalculada dentro de la proyección porque `render()` la pide en cada `mousemove`. Las de tabla las absorbe C7
- [x] C11a. El mapa deja de ser estado — **completada 2026-08-26**: 125,4 KB idénticos byte a byte en los ticks 0, 50 y 200 (medido; el 78% de la proyección con solo 4 facciones), viajaban en cada acción desde C6. `GET .../partidas/:gameId/mapa/:mapaId` en ambas superficies (`Cache-Control: immutable`), `:mapaId` es cache-buster puro que el servidor ignora al servir. `ResumenPartida`, `EstadoAdmin` y `ProyeccionJugador` llevan `mapaId` en vez de `mapa`. Verificado en vivo: tras `tick`/comando, ninguna petición nueva a `/mapa/` — solo tras `regenerarMundo`, que cambia la seed. 605/605 tests, `tsc` limpio en los dos proyectos
- [ ] C11b. Rasterizar el terreno — **descoped a propósito, sin implementar**: `elevacion`/`fertilidad` son parámetros de ruido, no rásteres, y el bioma no se guarda (se evalúa por píxel). El `MapaGenerado` de C11a sigue siendo indibujable sin `worldgen/`. Se difiere porque hoy no tiene consumidor real —el único cliente que existe (`cliente/`) sigue usando `@motor/*` y dibuja con la fachada `Mapa` en el navegador— y porque exige una decisión de dependencia nueva (librería de rasterización) que no hay razón para tomar antes de que exista un cliente sin motor real (bloqueado detrás de C8-C10 de todas formas)
- [ ] C12. Descubrimiento y operación: listar partidas (hoy el `gameId` llega fuera de banda), endpoints de exportación de administración (`exportarSimulacion`/`exportarMapaUnity`, hoy en el navegador) y una fuente de ticks (hoy solo `POST /admin/.../tick` a mano — E1 lo cierra del todo, pero sin algo el mundo no avanza)
- [ ] C13. Eficiencia de la sincronización — **objetivo corregido 2026-08-26**: el enemigo no era el mapa (sale por C11) sino `eventosDominio`, que crece sin techo y viaja entero cada vez (2,0 → 41,0 → 102,0 KB entre los ticks 0 y 200, con 4 facciones). Cursor `?desde=<version>`. Y el WebSocket difunde `EventoDominio`, que un cliente sin motor no puede aplicar a su estado cacheado — hoy la única reacción correcta a cualquier evento es refetch completo

## Fase D — Conversión temporal total

Objetivo: pasar el motor de ticks discretos a tiempo real total, sin perder lo
construido en B y C.

- [ ] D1. Reloj de simulación y fechas introducidos junto a la infraestructura de servidor (sin quitarla aún)
- [ ] D2. Construcción, políticas y cooldowns migrados a tiempo real
- [ ] D3. Producción, consumo, población, hambre y mantenimiento migrados a tasas/acumuladores
- [ ] D4. Caravanas, comercio, bandidos, NPC y combate migrados a eventos temporales
- [ ] D5. Migración explícita de partidas existentes de ticks a tiempo real
- [ ] D6. DTOs y frontends migrados de contadores (`ticksRestantes`) a fechas/duraciones/eventos
- [ ] D7. Balance recalibrado con simulaciones de referencia tras el cambio de modelo temporal

## Fase E — Operación persistente

Objetivo: infraestructura lista para operar de forma continua, no solo para
demostrar que funciona.

- [ ] E1. Scheduler temporal definitivo + recuperación de eventos vencidos tras reinicio
- [ ] E2. Auditoría, snapshots, backups y pruebas de restauración
- [ ] E3. Métricas (duración de tick/procesamiento, tamaño de cola, errores, clientes conectados) y herramientas de moderación
- [ ] E4. Ciclos de servidor, Maravilla, legado NPC y temporadas

---

## Criterios de éxito

- [ ] **Al cerrar la Fase C: un cliente en su propio repositorio, sin importar el código del motor (`@motor/*`, ni `src/` de este repo de ninguna forma) y sin ejecutar ninguna simulación (T3) ni ninguna derivación de entrada privilegiada, puede jugar una partida completa contra este backend.** Añadido 2026-08-26, corregido 2026-08-26 tras discutir el eje real: **no** es "cero líneas de dominio en el cliente" — [9_Reglas_vs_Simulacion.md](9_Reglas_vs_Simulacion.md) fija que las derivaciones T2a (puras, de entrada propia — lo que el jugador ya ve en su proyección) pueden y deben **reimplementarse como código propio del cliente**, para no pagar un viaje de red por cada *tooltip*; la divergencia de fórmula entre cliente y servidor se acepta a conciencia (doc 9, T2a). Lo que el criterio prohíbe es (a) importar el código del motor de este repo para ejecutarlo en el navegador, y (b) que el cliente calcule o reciba como entrada algo privilegiado (estado de otras facciones). El balance (T1) le llega como **datos servidos** (C7), no como módulo `constants` importado. Mientras `cliente/` necesite el alias `@motor/*` para compilar, la fase no está cerrada por mucho que C0–C6 estén marcados.
- [ ] Antes de iniciar la Fase D: una partida persistente corre en un backend dedicado, con varios jugadores + un admin conectados, comandos procesados en serie, y el servidor puede reiniciarse sin alterar la secuencia de ticks ni el RNG.
- [ ] Al cerrar la Fase D: la misma infraestructura opera en tiempo real total, sin que el motor dependa de un paso global fijo, y sin que API, persistencia o frontends traten el tick como unidad temporal principal.

## Registro de cierre de fases

_(completar con fecha y commit al cerrar cada fase)_

- Fase A: 2026-08-25 (A1–A6 completas; commit pendiente — el usuario gestiona los commits de esta sesión)
- Fase B: 2026-08-26 (marcada retroactivamente; B1–B5 completas sin fecha de cierre propia — ver nota arriba)
- Fase C: —
- Fase D: —
- Fase E: —
