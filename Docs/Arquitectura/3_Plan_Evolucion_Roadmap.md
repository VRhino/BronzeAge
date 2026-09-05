# Roadmap de evolución: backend multijugador y conversión temporal

> **Verificado contra el commit `652a0aa` (2026-09-05)** — repaso de las casillas de las cinco fases contra el
> código, no contra el documento anterior. Correcciones de esa pasada: las tres tareas derivadas del doc 6 §5
> que seguían en `[ ]` estando hechas (suscripciones, mapa como asset, cursor de eventos) y la sección nueva
> "Deuda arrastrada de fases cerradas", que rescata a la superficie el alcance que varios hitos `[x]`
> declararon NO cubierto en su propio texto. Misma convención que
> [1_Arquitectura_Actual.md](1_Arquitectura_Actual.md): al tocar este documento, anotar contra qué commit se
> verificó — un roadmap sin marca de verificación no se distingue de uno correcto hasta que alguien se apoya
> en él y falla.

Este documento es el mapa de alto nivel. Se marca un hito `[x]` solo cuando está
verificado (código, tests o documentación correspondiente), no cuando está "casi
listo". El desglose en tareas pequeñas y accionables vive en
[4_Plan_Evolucion_Tareas.md](4_Plan_Evolucion_Tareas.md); ese es el archivo que se
actualiza con más frecuencia. Este archivo se actualiza cuando se cierra un hito o
una fase completa.

**Alcance: solo arquitectura.** El trabajo posterior al 2026-08-30 (movimiento de ejércitos, murallas, niebla
de guerra) no aparece aquí porque son mecánicas de juego, no hitos de infraestructura: viven en
[`Docs/Game/`](../Game/) (canon de reglas) y en [`Consideraciones/`](../../Consideraciones/) (decisiones y
plan). Que este roadmap no se mueva durante semanas no significa que el repositorio esté parado.

Contexto y justificación de cada fase: ver
[2_Estudio_Evolucion_Backend_Multifrontend.md](2_Estudio_Evolucion_Backend_Multifrontend.md).
Decisiones de escala, conexiones y visibilidad: ver
[6_Sincronizacion_Visibilidad_y_Escala.md](6_Sincronizacion_Visibilidad_y_Escala.md).
Qué puede salir del servidor y qué no, módulo por módulo: ver
[9_Reglas_vs_Simulacion.md](9_Reglas_vs_Simulacion.md) — referencia transversal, la ejecutan C7, C10 y C11.

**Objetivo de escala (acordado 2026-08-24): mínimo 500 jugadores conectados
simultáneamente en una misma partida.** El cuello de botella es la CPU del tick, no la red ni la persistencia
— eso no ha cambiado desde agosto; lo que ha cambiado es la magnitud. **Medido el 2026-09-05: 500 jugadores
son ~70-100 asentamientos (no 500), y eso son 41-68 ms por tick** dentro de un intervalo de 60 000 ms. La
cifra de "~1,9 s" que este documento traía era una extrapolación de agosto a 500 ASENTAMIENTOS, corregida
dos veces desde entonces. Ver [documento 6](6_Sincronizacion_Visibilidad_y_Escala.md) §1 para las mediciones,
las tres optimizaciones que las mejoraron 6,9× —dejando el escalado en O(n^1.02), prácticamente lineal— y
lo que queda decidido y por decidir.

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
- [x] Cierre de huecos pequeños de Fase C — **2026-08-29**: (1) `Membresia`/`Sesion`/`Usuario` persistidos en disco (`server/persistenciaIdentidad.ts`, escritura atómica) — ya no se pierden al reiniciar el proceso, era el pendiente de C1/C3; (2) `GET`/`POST`/`DELETE /admin/partidas/:gameId/membresias` — otorgar/listar/revocar `moderador`/`observador`/`administrador_partida`, exige `administrador_partida`/`administrador_global`; (3) propiedad de escuadrones en combate (`comandaEscuadrones`) — un jugador solo comanda sus propias tropas en los 4 comandos de combate. 669/669 tests, verificado en vivo con servidor real. Detalle en [4_Plan_Evolucion_Tareas.md](4_Plan_Evolucion_Tareas.md#cierre-de-fase-c--huecos-pequeños-2026-08-29)
- [x] C4. Proyecciones de estado por audiencia — **completado 2026-08-26** (jugador ve su Facción completa, las demás solo metadatos públicos ya visibles en la ficción, `GET /jugador/partidas/:gameId`, `proyectarParaJugador`). Era "Slice 1"; el "Slice 2" (niebla de guerra: `ConocimientoJugador`, "último conocido", radio de visualización) NO es de arquitectura sino una **mecánica de juego con parámetros por definir** — movida a [`Docs/Mecanicas a desarrollar.md`](../Mecanicas%20a%20desarrollar.md) §12. El backend ya tiene la costura (mismo `proyectarParaJugador`) para añadir el filtrado cuando esos parámetros existan
- [x] C5. WebSocket único con canales, suscripciones autorizadas y reconexión sin duplicar comandos — completada 2026-08-26. Difusión de eventos de dominio (no de comandos: el cliente los ejecuta por HTTP igual que antes)
- [x] C6. Contrato publicable: CORS, versionado de API y OpenAPI generado desde los esquemas de Fastify — completada 2026-08-26. Todo bajo `/v1`; `GET /v1/openapi.json` sin autenticar; respuesta de comando autosuficiente en `/jugador/*` (incluye la proyección propia, ya no hace falta un `GET` aparte)
- [x] C7. Balance versionado y servido — **completada 2026-08-26**: `GET /v1/balance` (`server/rutas/balance.ts`), sin autenticar (regla pública, no estado de partida, doc 9 T1), las 39 tablas completas de `constants.ts` sin lista de exclusión (decisión del usuario: publicarlas todas). `BALANCE_VERSION` (`constants.ts`) se estampa en `PartidaExportada.balanceVersion` al exportar una partida, como registro de qué balance corría — sin rechazo al cargar si cambia, a diferencia de `worldgenVersion` (el balance no hace falta para reconstruir el snapshot, solo cambia qué reglas rigen desde ahora). **Alcance NO cubierto, a propósito**: "versionado por partida/temporada" con overrides reales — hoy sigue siendo un único valor de proceso, no hay mecanismo para que dos partidas corran versiones de balance distintas a la vez; el panel que mutaba `constants.ts` en caliente (`app/balanceConfig.ts`) se eliminó en Fase B sin reemplazo y sigue sin dueño. Absorbe el grupo "de tabla" de C10 (`capFundacion`, `cupoVivienda`, `slotsPoliticaDisponibles`, `nivelFaccionInfo`, `CATALOGOS`): con el balance servido, un cliente sin motor ya puede resolverlas por *lookup* sin viaje de red — pendiente de que exista ese cliente (C8-C13)
- [x] C8. Superficie y rol del cliente de administración — **completada 2026-08-26**. Replanteo: el administrador observa, no interactúa como jugador (decisión del usuario) — los 27 comandos de rol `jugador` no debían estar expuestos en el cliente de administración, no era un permiso que faltara. Interfaz corregida: paneles convertidos a solo consulta, ninguna acción de jugador queda en la UI. **Causa raíz corregida el mismo día**: `rolEnPartida` (`acceso/rolesDePartida.ts`) cortocircuitaba a `administrador_global` para cualquier actor con acceso técnico global, incluso con una `Membresia` `administrador_partida` real en esa partida (la que `otorgarAdministracion()` concede a quien la crea) — invertido el orden: la Membresia manda, `esAdministradorGlobal` es el *fallback* solo si no hay ninguna. `alternarFaccionNpc` —la única acción legítima de administrador (doc 5: "sin restricción si es admin")— ya no devuelve 403: verificado en vivo (servidor real, admin crea partida, jugador funda Facción, admin la cede al NPC — `resultado.ok: true`) y con dos regresiones nuevas en `server/__tests__/api.test.ts`. Dos tests preexistentes **codificaban el bug como comportamiento esperado** (`acceso/__tests__/rolesDePartida.test.ts`, `server/__tests__/api.test.ts`) — corregidos junto con el fix, no solo el código
- [x] C9. Contrato de comandos completo — **completada 2026-08-26**: `session/comandos/esquemas.ts`, un JSON Schema de `params` por cada uno de los 30 comandos (`Record<TipoComando, EsquemaJson>`, exhaustividad garantizada en compilación, mismo mecanismo que `MATRIZ_AUTORIZACION`). `ESQUEMA_EJECUTAR_COMANDO` (`server/rutas/comandos.ts`) pasa de `params: {}` a un `oneOf` con una rama cerrada por `tipo` — ajv rechaza con 400 antes de que el manejador vea el cuerpo, y el `oneOf` aparece en `GET /v1/openapi.json` (`@fastify/swagger` traduce `const` a `enum` de un valor, porque OpenAPI 3.0 no tiene `const`). **Solo valida FORMA, nunca reglas de dominio que ya tienen su propio código**: sin `minLength`/`minimum` en campos donde el dominio maneja el caso "vacío"/"cero" con semántica propia (`nombre` vacío en `crearFaccion` → `faccion.nombre_vacio`; vacío en `renombrarAsentamiento` → "usar el id"; vacío en `fusionar.nuevoNombre` → nombre por defecto; `cantidad<=0` en trueque/mercado → `TruequeInvalidoError`/`OrdenInvalidaError`) — un test que enviaba `nombre:''` esperando el rechazo de dominio lo detectó en el primer intento. Sí valida como `enum` cerrado los catálogos que el motor no comprueba antes de usar: `EdificioTipo`/`RecursoTipo`/`CargoTipo`/`CargoConstructor` (arrays exhaustivos nuevos en `domain/types.ts`/`construccion.ts`, mismo mecanismo `Record<Union,true>`) — un `EdificioTipo` inválido reventaba de verdad en `EDIFICIO_CATALOGO[tipo].costo` (`engine/construction.ts`), y un `direccion`/`tipo`/`origen` inválido se interpretaba SILENCIOSAMENTE como el otro valor del par (`direccion === 'arriba' ? ... : ...`) en vez de rechazarse — ambos, confirmados leyendo el código, no supuestos. **Efecto colateral necesario**: Fastify trae `coerceTypes: true` por defecto (pensado para query/params de URL, que siempre llegan como texto) — silenciaba el propio esquema convirtiendo `nombre: 123` en `"123"` en vez de rechazarlo; desactivado en `crearServidor` (`api.ts`). 6 tests nuevos, verificado en vivo. Arrastrado desde C2
- [x] C10. Consultas derivadas servidas por el backend — **completada 2026-08-26**. `calcularPrecioReferencia` hecha (`RunnerDePartida.preciosReferencia()`). Chokepoints y `evaluarViabilidadFundacion` (cliente) eliminados ese mismo día, no migrados — decisión del usuario. Las de tabla las absorbe C7. **Última pieza, geometría por frame (zonas, fusión, trazado)**: `RunnerDePartida.geometriaAsentamientos()`, memoizada por identidad de referencia de `estado.asentamientos` (sin TTL — a diferencia de precios, es pura, no hace falta un reloj que vigilar). `EstadoAdmin` la trae de TODOS los asentamientos (el admin observa toda la partida); `ProyeccionJugador` (`proyectarParaJugador`, que ahora recibe la geometría como parámetro) la filtra a **solo la Facción propia** — mismo criterio conservador que el resto de Slice 1, cero geometría de un rival. `trazadoParaAsentamiento` (nuevo, `engine/trazado.ts`) mueve a la capa de motor la orquestación que antes solo vivía en `cliente/src/app/gameStore.ts`. 5 tests nuevos, verificado en vivo (proyección de una jugadora solo trae su propio asentamiento; el admin ve los de las dos Facciones). **Nota de reconciliación**: [4_Plan_Evolucion_Tareas.md](4_Plan_Evolucion_Tareas.md) tenía además un "grupo (b)" de 9 consultas (`produccionInfo`, `mantenimientoInfo`, etc.) que este roadmap nunca contó como pendiente — correcto: [9_Reglas_vs_Simulacion.md](9_Reglas_vs_Simulacion.md), escrito el mismo día pero DESPUÉS, reclasifica esas 9 como T2a (entrada propia, el cliente las resuelve solas en cuanto tenga C7) — el grupo (b) de doc 4 quedó disuelto por esa reclasificación posterior y nunca se corrigió ahí; corregido ahora, ver la nota en doc 4
- [x] C11a. El mapa deja de ser estado — **completada 2026-08-26**: 125,4 KB idénticos byte a byte en los ticks 0, 50 y 200 (medido; el 78% de la proyección con solo 4 facciones), viajaban en cada acción desde C6. `GET .../partidas/:gameId/mapa/:mapaId` en ambas superficies (`Cache-Control: immutable`), `:mapaId` es cache-buster puro que el servidor ignora al servir. `ResumenPartida`, `EstadoAdmin` y `ProyeccionJugador` llevan `mapaId` en vez de `mapa`. Verificado en vivo: tras `tick`/comando, ninguna petición nueva a `/mapa/` — solo tras `regenerarMundo`, que cambia la seed. 605/605 tests, `tsc` limpio en los dos proyectos
- [x] C11b. Terreno indibujable sin `worldgen/` — **resuelto 2026-08-26, sin rasterizar** (decisión del usuario, tras plantear las opciones: rejilla de datos crudos, PNG con dependencia nueva, o esperar). Ninguna de las dos: `elevacion`/`fertilidad`/bioma son T2a en [9_Reglas_vs_Simulacion.md](9_Reglas_vs_Simulacion.md) ("el terreno lo ven todos", entrada no privilegiada) — el terreno **no necesita servirse ya calculado**, un cliente puede recalcularlo él mismo desde los parámetros públicos que ya sirve C11a, con su propia copia de las funciones puras de evaluación (no una librería de rasterización: cero dependencia nueva). Se creó **[`cliente-jugador/`](../../cliente-jugador/)** —boilerplate nuevo, hermano de `cliente/`, sin alias `@motor/*` de ningún tipo— con `src/terreno/` (copia de `elevacion.ts`/`fertilidad.ts`/`biomas.ts`/`ruido.ts`/`rios.ts` reducidos a la evaluación pura, sin RNG ni generación) y un `render.ts` que pinta bioma+ríos a canvas. **Verificado en vivo**: el mismo `gameId`/seed renderizado a la vez en `cliente/` (motor real) y `cliente-jugador/` (evaluador duplicado) produce la MISMA geografía — lagos, montañas y ríos coinciden en posición y forma. **Limitación documentada, no resuelta**: sin soporte de `region` (`worldgen/regiones.ts`, ~300 líneas no portadas) — una partida creada con `region` diverge donde pesa la guía geográfica; avisa por consola. **Riesgo de mantenimiento aceptado a conciencia**: si `WORLDGEN_VERSION` sube en el servidor y toca estas funciones/umbrales, `cliente-jugador/src/terreno/` necesita el mismo cambio a mano — sin comprobación automática todavía. **Ampliado 2026-08-27**: bosques fusionados (`poligonos.ts`+`bosques.ts`, mismo motor de fusión que el servidor usa para dibujar zonas de Facción, pero aplicado solo a bosques —dato público— nunca a zonas —entrada privilegiada—) — verificado idéntico byte a byte contra el servidor sobre los 170 bosques reales de una partida, no solo visualmente parecido
- [x] C12. Descubrimiento y operación — **completada 2026-08-26**:
  - **`GET /admin/partidas`** (`persistenciaPartida.ts` `listarPartidas`): lee el DIRECTORIO de snapshots, no `RegistroDePartidas` (que solo conoce lo abierto en este proceso) — una partida guardada antes de un reinicio sigue siendo descubrible. Exige `administrador_global`, mismo criterio que crear una partida
  - **Efecto colateral necesario, encontrado al escribir el test**: una partida recién creada, sin ningún comando aplicado todavía, no existía en disco (`guardarPartida` solo corría dentro de `ejecutar()`/`avanzarTick()`) — invisible para `listarPartidas`, y perdida del todo si el proceso moría antes del primer comando pese a que la creación ya había respondido 201. `RunnerDePartida.crearYPersistir` (nuevo) persiste antes de devolver el runner, usado por `cargarOCrear` y por `RegistroDePartidas.descartarYCrear` (que de paso pasó a ser async). Segundo efecto colateral: `descartarYCrear` reemplaza una partida que puede seguir en disco con `version` > 0 por una que empieza en 0 — la red de seguridad de `guardarPartida` contra escrituras concurrentes lo interpretaba como el conflicto que existe para detectar; `forzar: true` en `guardarPartida` distingue el reemplazo deliberado del bug real
  - **Fuente de ticks**: `RegistroDePartidas` acepta un `intervaloTickMs` opcional (constructor) — arranca los ticks automáticos al abrir/crear una partida (`iniciarTicksAutomaticos`, metrónomo simple; **D5 lo sustituyó por `iniciarRelojDeMundo`**, con catch‑up). **Opt-in, `undefined` por defecto**: sin configurarlo (como en absolutamente todos los tests existentes), ninguna partida avanza sola — mismo criterio que `ADMINISTRADORES`. Env var `INTERVALO_TICK_MS` en `server/index.ts`
  - **`GET /admin/partidas/:gameId/exportar`**: descarga el snapshot completo (`Content-Disposition: attachment`). Más simple de lo previsto: el formato v2 de `exportarSimulacion` (`GameStore`, retirado junto con `importarSimulacion`) no hacía falta reconstruirlo — el snapshot que YA se persiste tras cada comando (`PartidaExportada`) es el propio formato de exportación, así que `RunnerDePartida.exportar()` es un simple `sesion.exportar()`
  - **`GET /admin/partidas/:gameId/exportar-unity`**: mismo cálculo que ya existía en `world/exportUnity.ts` (`exportarParaUnityTerrain`, antes solo invocado desde el navegador vía `@motor/*`), ahora servido — heightmap/splatmap en base64 dentro de un JSON, sin inventar un protocolo multipart. Resolución configurable por query string, 400 (no 500) si no cumple 2^n+1
  - Tests nuevos en `persistenciaPartida.test.ts`, `runnerDePartida.test.ts`, `registroDePartidas.test.ts` (nuevo) y `api.test.ts`. Verificado en vivo: listar vacío → crear → aparece inmediatamente sin ningún comando; `INTERVALO_TICK_MS=200` avanza la partida sola; export y export-unity responden con datos reales
- [x] C13. Eficiencia de la sincronización — **completada 2026-08-26**: el objetivo corregido seguía siendo `eventosDominio`, que crece sin techo y viaja entero cada vez. `EventoDominioConVersion` (`session/estado.ts`): cada evento lleva la `version` de la partida en la que `exito()` lo estampó (único punto que sube `version`, así que único punto que puede estampar esto). `GET /admin|jugador/partidas/:gameId/eventos?desde=<version>` — cursor incremental filtrado por audiencia en la superficie de jugador (mismo criterio `esPropio` que la proyección, factorizado para no divergir). **Alcance NO cubierto, a propósito**: las lecturas de estado completo (`EstadoAdmin`/`ProyeccionJugador`) siguen trayendo `eventosDominio` entero — quitarlo habría roto el único cliente que existe hoy (`cliente/`, que todavía lo usa para su línea de tiempo de depuración) sin que exista un consumidor migrado al cursor que lo reemplace; queda como follow-up cuando lo haya. Tampoco se tocó que el WebSocket difunda `EventoDominio` en vez de deltas de estado — el cursor ya resuelve el problema práctico ("refetch completo" pasa a ser "pedir solo lo nuevo"), convertir eventos en parches aplicables es una pieza de diseño mayor, fuera de alcance de esta pasada

## Fase D — Conversión temporal total

Objetivo: pasar el motor de ticks discretos a tiempo de mundo con fechas, sin perder lo construido en B y C.

> **Completa (D1–D6 + cierre, 2026‑08‑29/30).** El `tick` ya no aparece en NINGÚN contrato externo — solo
> sobrevive como paso de integración interno del motor (`GameSessionState.tick`, del que se deriva el
> `instante`) y como diagnóstico admin. Los eventos, el log, los DTOs y las constantes son todo tiempo de
> mundo. Queda fuera: (a) la infra de comandos programados (aterriza con su primera mecánica de Fase 1+);
> (b) la **pasada de rebalanceo en tiempo**, que no es un hito de D — con 1 tick = 1 minuto real, valores
> como población +12 %/min compuesto necesitan reajuste sobre el laboratorio batch.

> **Re‑planteada 2026‑08‑29** tras revisión a fondo + paso por el consejo. El modelo temporal completo
> (relojes, determinismo, qué se conserva y qué se retira) vive en
> [10_Modelo_Temporal.md](10_Modelo_Temporal.md). Cambios respecto al plan original:
>
> - **Decisión del usuario**: mundo = tiempo real, **1 tick = 1 minuto real**, sin capa de aceleración
>   (modelo OGame). `instante = ÉPOCA + tick × 60 000 ms`, derivado del tick, nunca leído del reloj de pared.
> - **D3 original (todo a tasas/acumuladores) se cae**: solo hacía falta con paso variable. El paso es fijo.
> - **La "independencia del tamaño del paso" del doc 2 se retira**: medida inalcanzable (el RNG escala 1:1 con
>   los pasos, el crecimiento es compuesto) y no observable. Sustituida por "el paso es fijo, parte del
>   contrato de la partida".
> - **El scheduler sube de Fase E a D5** — es lo que el backlog de mecánicas (caravanas planificadas, asedios)
>   necesita. Absorbe E1.
> - **El rebalanceo en tiempo (antiguo D7) NO es un hito de D**: es una pasada dedicada DESPUÉS, apoyada en el
>   laboratorio batch — D1–D6 preservan valores y el juego sigue corriendo con el balance viejo.

- [x] D1. Reloj de mundo — **2026‑08‑29**: `SIMULACION` en `constants.ts` (`epocaInicial`/`duracionTickMs`, servido en `/v1/balance`, `BALANCE_VERSION` → 2), `instanteDeTick(tick)` puro y derivado (nunca almacenado, sin cambio de formato de snapshot), `GameSession.ejecutar` lo deriva del tick (`momento` eliminado de la firma), `RunnerDePartida` deja de inyectar `this.ahora()`. Cierra los dos bugs temporales en producción (doc 10 §7); regresión congelada. 677 tests, verificado en vivo
- [x] D2. Tipos `Instante`/`Duracion` (`domain/tiempo.ts`, branded) + los ~10 campos‑instante renombrados `*EnTick: number` → `*En: Instante` en `domain/types.ts`/`EstadoMapa`/`GameSessionState`. `ContextoSimulacion`/`ContextoComando` ganan `instante`. `ticksComoDuracion(n)` en `constants.ts` como puente hasta D6. Migración de snapshot v1→v2 (relación 1:1). 678 tests, verificado en vivo
- [x] D3. `Edificio.ticksRestantes: number` → `Edificio.completaEn?: Instante` (solo `en_construccion`). `avanzarConstruccion` recibe `instante` y compara `instante >= completaEn` en vez de decrementar. Migración de snapshot v2→v3 (`migrarSnapshot` encadena v1→v2→v3). Timing tick‑a‑tick idéntico (snapshot baseline sin cambios). De paso: `scripts/run-batch-sim.ts` al día con el modelo temporal, ids de edificio/campamento sin tick. Implementa "fechas, no contadores" del doc 6 §4. 679 tests, verificado con el laboratorio batch en vivo
- [x] D4. DTOs con instante de mundo: `ResumenPartida`/`ResumenPartidaEnDisco`/`EstadoAdmin`/`ProyeccionJugador` ganan `instante: Instante` (derivado como `mapaId`, no almacenado). `tick` sigue viajando, marcado provisional. Esquemas de respuesta actualizados (Fastify filtra por schema). Los campos‑instante de entidades (`completaEn`/`expiraEn`/`heridoHasta`) ya viajaban absolutos desde D2/D3; los eventos ya llevan `momento` ISO. 679 tests, verificado en vivo sobre las 4 superficies HTTP
- [~] D5. **Reloj de mundo + catch‑up hechos** (= el antiguo E1). `RunnerDePartida.iniciarRelojDeMundo(intervaloMs)` sustituye al metrónomo `iniciarTicksAutomaticos`: mantiene `estado.tick` sincronizado con el reloj de pared (referencia = `guardadoEn` del snapshot, anclada a `tickAlConstruir`), ejecuta EN RÁFAGA los ticks vencidos tras un reinicio por la cola serial, sin acumular jitter del temporizador. `cargarPartida` devuelve `{ sesion, guardadoEn }`; `RegistroDePartidas` recibe el reloj de pared inyectado y expone `cerrar()` (apagado limpio, `onClose` de Fastify). 683 tests, verificado en vivo por HTTP (crear en proceso A → reabrir en B 7 min después → catch‑up a tick 7). **Comandos programados a un `instante`: aplazados** — sus consumidores (asedios formales, caravanas planificadas) son Fase 1+ por decisión explícita ([doc Game 5 §"Asedios"](../Game/5_Sistema_Militar_y_Combate.md), [doc Game 3](../Game/3_Sistema_Economico_y_Comercio.md)); construir el almacén + despacho ahora sería infra especulativa sin consumidor. Aterriza con la primera mecánica que lo pida.
- [x] D6. Constantes de `constants.ts` renombradas de tick a minuto (`tiempoConstruccionMinutos`, `plazoMinutosPorDefecto`, `graciaMinutos`, `cooldownMinutos`, `respawnMinutos`, `duracionHeridoMinutos`, `duracionMinutosPorDefecto`, `horizonteMinutos*`, `*PorMinuto`), **sin tocar valores** (1 tick = 1 min). `ticksComoDuracion` eliminado — el motor usa `minutos()` de `domain/tiempo.ts`. `RelacionPolitica.tributo.cantidadPorTick` → `cantidadPorMinuto` (snapshot v3→v4). `BALANCE_VERSION` 2→3. 684 tests, `tsc` limpio
- [x] Cierre de Fase D. Se retira el `tick` provisional del contrato: `EventoDominio` solo `momento`; `EventoLogAdmin.tick`→`momento`; `ContextoSimulacion.tick` eliminado; `ResumenPartida`/`ProyeccionJugador`/`ResumenPartidaEnDisco` pierden `tick` (ya llevan `instante`). Migración de snapshot v4→v5. El `tick` solo queda como paso de integración interno del motor y diagnóstico admin. 685 tests, `tsc` limpio, verificado en vivo por HTTP
- [x] ~~Guard de autoridad temporal~~ — **hecho 2026‑08‑29**, antes de D1: `src/__tests__/autoridadTemporal.test.ts` congela que el núcleo puro no lee reloj ni aleatoriedad ambiental y `session` no lee el reloj de pared (doc 10 §4). De paso, `world/exportUnity.ts` deja de llamar a `new Date()` (el servidor le pasa `generadoEn`)

## Fase E — Operación persistente

Objetivo: infraestructura lista para operar de forma continua, no solo para
demostrar que funciona.

- [x] ~~E1. Scheduler temporal definitivo + recuperación de eventos vencidos tras reinicio~~ — **hecho en D5** (2026‑08‑29): `RunnerDePartida.iniciarRelojDeMundo` con catch‑up en ráfaga tras reinicio. Se movió a D5 porque el reloj de mundo y el catch‑up son parte de la conversión temporal, no operación posterior
- [x] E2. Auditoría, snapshots, backups y pruebas de restauración — **completada 2026-09-05**. `server/auditoria.ts` (JSONL append-only por partida, hermano del snapshot: aceptados Y rechazados, incluidos los 403 de autorización, que antes no dejaban ningún rastro; `GET /v1/admin/partidas/:gameId/auditoria` filtrable), `server/respaldos.ts` (copia fechada, y restauración que VERIFICA que el respaldo carga antes de sustituir el snapshot vigente — un respaldo corrupto falla sin tocar la partida buena), `server/mantenimiento.ts` (pasada periódica opt-in: respaldar + podar respaldos por cuenta + podar auditoría por edad) y `scripts/restaurar-partida.ts` como procedimiento ejecutable. **Corrige una premisa del plan**: no había pila de snapshots que podar — `guardarPartida` sobrescribe siempre el mismo archivo; lo que crecía sin techo era la auditoría. **Alcance NO cubierto, a propósito**: (a) la auditoría de cambios de BALANCE (doc 2 §8) sigue sin hacerse, bloqueada por la deuda de C7 — se pide sobre un balance versionado por partida que no existe; (b) restaurar exige el servidor parado, porque exponerlo por HTTP necesitaría que `RegistroDePartidas` supiera cerrar UNA partida y esa pieza no tiene consumidor todavía. 991 tests, `tsc` limpio, verificado en vivo por HTTP. Detalle en [4_Plan_Evolucion_Tareas.md](4_Plan_Evolucion_Tareas.md#e2--auditoría-snapshots-backups-y-restauración)
- [~] E3. Métricas y herramientas de moderación — **métricas completadas 2026-09-05**, moderación abierta. `server/metricas.ts` + `GET /v1/admin/metricas` (administrador global, no por partida): duración de tick, profundidad de cola, recuento de comandos por resultado con las cuatro causas separadas, conexiones, memoria y **tamaño de las ráfagas de catch-up**. Ensambla lo que ya llevan el runner, la auditoría de E2 y el hub, en vez de duplicar contadores. Incluyó la **re-medición de escala** que el doc 6 pedía desde el cierre de la Fase D (`scripts/medicion-escala.ts`) — ver la deuda arrastrada abajo, que esa medición resuelve una y reencuadra otra. Falta la **moderación**: E2 y esto sirven el diagnóstico, pero el acto (expulsar, silenciar, revertir) depende de decisiones de diseño sin tomar. 1003 tests, `tsc` limpio, verificado en vivo por HTTP incluida una ráfaga de catch-up real
- [ ] E4. Ciclos de servidor, Maravilla, legado NPC y temporadas

---

## Deuda arrastrada de fases cerradas

Ninguna de estas bloqueó el cierre de su fase, y todas están declaradas dentro del texto del hito que las
dejó fuera — **pero ahí no se ven**: un lector que recorra las casillas ve cinco fases en verde. Se listan
aquí para que dejen de depender de que alguien relea el párrafo correcto.

- [x] **Bloqueo de la cola serial** — **RESUELTO 2026‑09‑05**, y no por donde se buscaba. Llevaba abierto desde el 24 de agosto como decisión técnica (trocear la ráfaga, ceder la cola cada N ticks, un worker aparte). La re‑medición mostró primero que no es un tick suelto —111 ms a 100 asentamientos dentro de un intervalo de 60 000 ms— sino la **ráfaga de catch‑up** tras una caída: una sola entrada de cola, ~18,6 minutos sin que nadie pueda ejecutar un comando. Y al mirarla de cerca resultó que la pregunta de fondo era de diseño de juego, no de infraestructura: **si una caída consume tiempo de mundo**. Decisión del usuario: **no**. El reloj se ancla a "ahora" al arrancar, reabrir una partida la reanuda en su tick, y no queda ráfaga que pueda bloquear la cola. Ver doc 6 §1 y doc 10 §2.
- [x] **Re‑medir la escala tras la Fase D** — **hecha 2026‑09‑05** (`scripts/medicion-escala.ts`, doc 6 §1 reescrito), y de ella salió una optimización: el tick había engordado ~3× desde agosto, el perfilado señaló que la red de calles se rehacía entera cada tick por asentamiento sin haber cambiado nada, y memoizarla por contenido lo dejó **3,6× más rápido** (471 → 132 ms a 100 asentamientos) con el exponente cayendo de O(n^1.42) a **O(n^1.12)** — buena parte de lo superlineal era trabajo repetido, no simulación. Equivalencia demostrada con sellos SHA‑256 del estado completo a lo largo de 150 ticks, idénticos byte a byte.
- [ ] **Balance por partida/temporada** (alcance que C7 declaró NO cubierto) — `BALANCE_VERSION` se estampa en
  cada snapshot, pero sigue siendo un único valor de proceso: dos partidas no pueden correr balances
  distintos a la vez. El panel que lo mutaba en caliente (`app/balanceConfig.ts`) se eliminó en Fase B sin
  reemplazo y sigue sin dueño. Es el único `[~]` que queda en el backlog de riesgos del doc 4. **Segundo consumidor esperándola desde 2026-09-05**: la auditoría de cambios de balance que E2 dejó fuera por esto mismo.
- [x] **`eventosDominio` entero en las lecturas de estado** — **cerrado 2026‑09‑05**, el follow‑up que C13 dejó abierto. Medido antes de tocarlo: el campo **solo crece** y era el **87‑88 % de una lectura de estado** (264 KB de 303 KB en el tick 200), reenviando en cada lectura y en cada respuesta de comando un historial que el cliente ya tenía. C13 lo aplazó porque quitarlo habría roto al único cliente sin que hubiera ninguno migrado al cursor; se cierra **migrándolo**: `cliente/` mantiene ahora el historial él mismo, sembrado con `?desde=0` al abrir y extendido con `?desde=<version>` después. El filtro de propiedad —la parte de seguridad— no se toca: sigue en `eventosDominioParaJugador`, con sus tests reapuntados ahí. Verificado en el navegador con el cliente real: la consola arranca con sus 941 entradas y tras un refresco pasa a 945, sin duplicar ninguna.
- [ ] **Pasada de rebalanceo en tiempo** — explícitamente *no* es un hito de D (era el antiguo D7). Con
  1 tick = 1 minuto real, población a ~12 %/minuto compuesto duplica cada ~6 min reales. Esfuerzo dedicado
  apoyado en el laboratorio batch.
- [ ] **Comandos programados a un `instante`** (de D5) — aplazados a conciencia: sus consumidores (asedios
  formales, caravanas planificadas) son Fase 1+. Aterriza con la primera mecánica que lo pida, no antes;
  construirlo ahora sería infraestructura especulativa sin consumidor.

---

## Criterios de éxito

- [x] **Al cerrar la Fase C: el backend sirve TODO lo que un cliente sin motor necesita para jugar una partida completa** — sin que ese cliente importe el código del motor (`@motor/*`, ni `src/` de este repo de ninguna forma) ni reciba como entrada nada privilegiado (estado de otras facciones). Añadido 2026-08-26, corregido 2026-08-26 tras discutir el eje real: **no** es "cero líneas de dominio en el cliente" — [9_Reglas_vs_Simulacion.md](9_Reglas_vs_Simulacion.md) fija que las derivaciones T2a (puras, de entrada propia) se **reimplementan como código propio del cliente**, para no pagar un viaje de red por cada *tooltip*. El balance (T1) le llega como **datos servidos** (C7), no como módulo `constants` importado.

  **Cumplido 2026-08-26** (C7–C13): balance servido, geometría por frame, terreno recalculable con una copia
  propia de funciones puras, esquema de comandos, descubrimiento, exportación, cursor de eventos, WebSocket
  con canales. **Construir un cliente jugable completo (UI de comandos, etc.) es trabajo de interfaz en su
  propio repositorio — fuera del alcance de ESTE repo, que es solo servidor.** `cliente/` (herramienta de
  administración/desarrollo) y `cliente-jugador/` (boilerplate de terreno) viven aquí solo como referencia
  hasta la separación de repos; no son criterios de este plan.
- [x] Antes de iniciar la Fase D: una partida persistente corre en un backend dedicado, con varios jugadores + un admin conectados, comandos procesados en serie, y el servidor puede reiniciarse sin alterar la secuencia de ticks ni el RNG. **Cumplido 2026-08-29**: la persistencia de partida (B3) ya cubría estado/tick/RNG/eventos; el cierre de Fase C añadió la persistencia de identidad (usuarios/sesiones/membresías) que faltaba para que "el servidor puede reiniciarse" sea cierto de punta a punta, no solo para el estado de simulación.
- [x] Al cerrar la Fase D: la infraestructura opera en tiempo real (reloj de mundo + catch-up), y **ni el contrato hacia afuera —API, persistencia, eventos— ni el balance tratan el tick como unidad temporal**. Cumplido 2026-08-29/30 (D1–D6 + cierre): DTOs y eventos en `instante`/`momento`, constantes en minutos, snapshot en v5, `tick` retirado del contrato. Nota: "sin que el motor dependa de un paso global fijo" (P3 del plan original) **se retiró** — medido inalcanzable y no observable (doc 10 §1); el paso es fijo a propósito y forma parte del contrato de la partida.

## Registro de cierre de fases

_(completar con fecha y commit al cerrar cada fase)_

- Fase A: 2026-08-25 (A1–A6 completas; commit pendiente — el usuario gestiona los commits de esta sesión)
- Fase B: 2026-08-26 (marcada retroactivamente; B1–B5 completas sin fecha de cierre propia — ver nota arriba)
- Fase C: **completa** (2026-08-29). C0–C13 + huecos pequeños (identidad persistida, gestión de membresías, propiedad de escuadrones). C4 quedó completo: el "Slice 2" (niebla de guerra) no es de arquitectura sino una mecánica de juego con parámetros por definir — movida a `Mecanicas a desarrollar.md` §12. El cliente jugable completo es trabajo de un repo de interfaz aparte, fuera del alcance de este repo (solo servidor).
- Fase D: **estructuralmente completa** (2026-08-29/30). D1–D6 + cierre del contrato. Pendiente, sin bloquear el cierre: (a) comandos programados de D5 — aplazados a Fase 1+ con su primera mecánica; (b) rebalanceo en tiempo — pasada dedicada, no es un hito de D.
- Fase E: **en curso** (E1 hecho en D5; **E2 completa 2026-09-05**; **E3 a medias 2026-09-05** — métricas hechas, moderación abierta; E4 sin empezar). Ver además "Deuda arrastrada de fases cerradas" arriba.
