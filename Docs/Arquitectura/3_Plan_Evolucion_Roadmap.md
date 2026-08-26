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

- [ ] B1. `GameSession` (o `GameApplicationService`) que reemplaza las responsabilidades de casos de uso de `GameStore`
- [ ] B2. Proceso backend Node.js + repositorio de partida
- [ ] B3. Persistencia de estado, tick, RNG, IDs, configuración y eventos (snapshots como estrategia inicial)
- [ ] B4. API administrativa (HTTP) sobre esa partida
- [ ] B5. La interfaz actual (`main.ts`) migrada a cliente remoto de esa API, en vez de llamar a `GameStore` local

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

- [x] C0. Cliente de navegador extraído a `cliente/`; el repo queda como backend puro (sin Vite ni capas `app`/`ui`/`main`/`lab`)
- [x] C1. Usuarios, sesiones y membresías implementados (2026-08-25), con la autenticación tras un puerto intercambiable: sustituir el proveedor de desarrollo por uno real es escribir un adaptador y darlo de alta, sin tocar nada de lo que se apoya en él
- [x] C2. Autorización de comandos por actor / facción / asentamiento / cargo (2026-08-25): matriz con una fila por comando, exhaustividad garantizada en compilación. El esquema de `params` por comando y la `idempotencyKey` siguen pendientes (C6 y reconexión de C5)
- [x] C3. Superficies separadas: `/admin/*` y `/jugador/*` con requisitos de rol distintos — completada 2026-08-25. Cierra de paso el agujero que quedaba de B4: crear partida, tick y estado ya no son endpoints abiertos. `/jugador/*` sin lectura de estado a propósito, hasta que existan las proyecciones de C4
- [ ] C4. Proyecciones de estado por audiencia — **Slice 1 completado 2026-08-26** (jugador ve su Facción completa, las demás solo metadatos públicos, `GET /jugador/partidas/:gameId`). Slice 2 pendiente: `ConocimientoJugador` y "último conocido" necesitan un radio de visualización (balance de juego) no definido en ningún doc de este repo
- [x] C5. WebSocket único con canales, suscripciones autorizadas y reconexión sin duplicar comandos — completada 2026-08-26. Difusión de eventos de dominio (no de comandos: el cliente los ejecuta por HTTP igual que antes)
- [ ] C6. Contrato publicable: CORS, versionado de API y OpenAPI generado desde los esquemas de Fastify
- [ ] C7. Balance versionado por partida/temporada (deja de ser módulo global mutable)

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

- [ ] Antes de iniciar la Fase D: una partida persistente corre en un backend dedicado, con varios jugadores + un admin conectados, comandos procesados en serie, y el servidor puede reiniciarse sin alterar la secuencia de ticks ni el RNG.
- [ ] Al cerrar la Fase D: la misma infraestructura opera en tiempo real total, sin que el motor dependa de un paso global fijo, y sin que API, persistencia o frontends traten el tick como unidad temporal principal.

## Registro de cierre de fases

_(completar con fecha y commit al cerrar cada fase)_

- Fase A: 2026-08-25 (A1–A6 completas; commit pendiente — el usuario gestiona los commits de esta sesión)
- Fase B: —
- Fase C: —
- Fase D: —
- Fase E: —
