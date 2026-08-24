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
- [ ] A5. Eventos de dominio estructurados, separados de los mensajes de log en texto — ⚠️ **NO completada, solo la base**: tipo `EventoDominio` definido y `avanzarSimulacion` ya lo devuelve, pero todo evento sigue siendo texto libre bajo `codigo: 'legado'`. **Progreso real: 0/13 subsistemas migrados (0%)** — se aborda progresivamente, uno por tarea; este hito solo se marca `[x]` cuando el marcador de [4_Plan_Evolucion_Tareas.md](4_Plan_Evolucion_Tareas.md#-marcador-de-progreso--migración-por-subsistema-0--13-migrados-0) llegue a 13/13 (100%)
- [x] A6. Contratos mínimos documentados: `Usuario`, `Sesion`, `Jugador`, `Rol`, `Partida`, `Membresia` (solo diseño, sin implementar aún) — ver [5_Contratos_Identidad_Permisos.md](5_Contratos_Identidad_Permisos.md)

## Fase B — Backend provisional con ticks

Objetivo: una partida corriendo en un proceso Node.js dedicado, con persistencia,
gobernada por una única `GameSession`, todavía sin multijugador real.

- [ ] B1. `GameSession` (o `GameApplicationService`) que reemplaza las responsabilidades de casos de uso de `GameStore`
- [ ] B2. Proceso backend Node.js + repositorio de partida
- [ ] B3. Persistencia de estado, tick, RNG, IDs, configuración y eventos (snapshots como estrategia inicial)
- [ ] B4. API administrativa (HTTP) sobre esa partida
- [ ] B5. La interfaz actual (`main.ts`) migrada a cliente remoto de esa API, en vez de llamar a `GameStore` local

## Fase C — Multijugador sobre ticks

Objetivo: varios jugadores y un administrador conectados a la misma partida, con
autorización real.

- [ ] C1. Usuarios, membresías y roles implementados
- [ ] C2. Autorización de comandos por actor / facción / asentamiento / cargo
- [ ] C3. WebSocket o SSE para difusión de cambios, con reconexión e idempotencia de comandos
- [ ] C4. Proyecciones de estado por audiencia (jugador / facción / admin / observador)
- [ ] C5. Frontend de jugador independiente, con solo las acciones y datos autorizados
- [ ] C6. Balance versionado por partida/temporada (deja de ser módulo global mutable)

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

- Fase A: —
- Fase B: —
- Fase C: —
- Fase D: —
- Fase E: —
