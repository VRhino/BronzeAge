# BA-002 — Identidades persistentes compartidas

**Destino:** `BronzeAgeFase0`  
**Estado:** ACEPTADO (revisión 2026-09-11, corregido con la revisión de Codex del mismo día — ver abajo)  
**Prioridad:** Alta

## Problema

BronzeAge identifica al participante como `jugadorId`; Conquest mantiene una cuenta local con varios héroes,
busca héroes por `heroName` y mezcla IDs persistentes con IDs numéricos/entidades ECS. El nombre visible no
puede ser la llave entre procesos.

## Propuesta

Separar `userId`, `playerId`, `heroId`, `squadId`, `battleId`, `participantId` y `runtimeEntityId`. Todos son
opacos; nombres de héroe, escuadra, facción y asentamiento son editables. Los IDs de ejecución ECS no salen por
la API.

## Decisiones requeridas

- Uno o varios héroes por jugador del mundo.
- Pertenencia del héroe a cuenta global o partida.
- Pertenencia de escuadra al jugador o a un héroe.
- Cambio de héroe con columna desplegada.
- Identidad de una escuadra aniquilada y posteriormente repuesta.

## Criterios de aceptación futuros

- Ningún contrato usa el nombre como referencia.
- Un resultado enlaza sin ambigüedad dueño, héroe y escuadra persistentes.
- Se documentan cardinalidad, ciclo de vida y ámbito de cada ID.

## Decisiones del propietario (revisión 2026-09-11, corregida por decisión de producto del usuario)

**`Heroe` SÍ se crea en BronzeAge, con toda su estructura proveniente de Conquest** (clase, nivel, XP,
perks, equipo, avatar). No es una entidad que ya existiera —la revisión inicial de esta propuesta lo daba
por inexistente, correcto en ese momento— sino una decisión de producto tomada ahora: se añade.

**Consecuencia estructural, no solo de nombres:** hoy `Jugador` (`domain/types.ts`) es quien carga el estado
de partida — `liderazgoBase`, `ubicacion` (asentamiento/columna/desconectado), `plazasRecordadas`,
`exploracionPersonal` — y quien aparece como dueño en `Escuadron.jugadorId`, `Ejercito.participantes`/
`liderId`, `CargosAsentamiento.*Id`, `Faccion.reyId`/`embajadorId`/`ciudadanosIds`,
`Asentamiento.jugadoresFundadoresIds`/`casasCompradas`. **Todo ese estado y esas referencias pasan a
`Heroe`.**

**Corrección de Codex (2026-09-11), incorporada — no se crea una entidad `Jugador` nueva en `domain/`.** La
primera versión de esta propuesta decía que `Jugador` "queda como entidad de acceso" — eso duplicaría lo que
ya existe: `Membresia` (en `acceso/tipos.ts`) YA une `usuarioId` + `gameId` (+ `jugadorId` + rol + vigencia).
Crear otro objeto de dominio con esos mismos campos sería una segunda fuente de verdad de acceso. En su
lugar:

- `Usuario`, `Sesion` y `Membresia` siguen viviendo exclusivamente en `acceso/`, sin cambios.
- `Membresia.jugadorId` sigue siendo el identificador de participación dentro del mundo — no desaparece.
- `Heroe` vive en `domain/` y lleva `jugadorId` como dueño uno-a-uno (ese `jugadorId` es el mismo valor de
  `Membresia.jugadorId`, no una copia con otro significado).
- No hace falta persistir ningún `Jugador` dentro de `GameSessionState`: el actual `GameSessionState.jugadores`
  se reemplaza por `heroes` al hacer el corte de modelo.
- Los comandos resuelven la cadena autenticada `Sesion -> Usuario -> Membresia -> jugadorId -> Heroe`. Las
  reglas de juego reciben `heroeId`; auditoría y autorización conservan también `usuarioId`/`jugadorId`.

1. **Un solo héroe por jugador (cuenta) en cada mundo.** Confirmado. Cardinalidad: `Usuario` → como mucho una
   `Membresia`/`jugadorId` por `gameId` (ya así) → como mucho un `Heroe` por ese `jugadorId`/`gameId`,
   forzado por unicidad de `Heroe.jugadorId` dentro de la partida (no por una entidad `Jugador` nueva).
2. **El héroe pertenece a una partida (`gameId`), no es global.** Confirmado — mismo ámbito que `Jugador`
   hoy, nunca cuenta global. Progresión/equipo de un héroe no cruza de un mundo a otro.
3. **La escuadra pasa a pertenecer al héroe.** Confirmado — corrige la respuesta anterior de esta propuesta.
   `Escuadron.jugadorId` se convierte en `Escuadron.heroeId`. **Corrección de Codex incorporada:** el ámbito
   de `squadId` no es "dentro del contenedor donde vive" (confundía ubicación con identidad) — es único y
   estable dentro de `gameId` con independencia de dónde esté guardada la escuadra en cada momento
   (guarnición/ejército/escolta/reserva de batalla), con invariante de exclusividad: solo en UNO de esos
   contenedores a la vez. El invariante "un `Escuadron` por `tropaId` por héroe/asentamiento" también se
   revisa: no puede depender de que dos escuadras del mismo `tropaId` nunca coincidan en la misma lista —una
   escuadra que sale de un asentamiento y otra reclutada después del mismo `tropaId` podrían volver a
   coexistir. Queda como pendiente de implementación decidir el mecanismo exacto (fusionar, bloquear
   reclutamiento mientras la original esté fuera, o permitir varias instancias del mismo `tropaId`) — no es
   una decisión de identidad, es una regla de negocio a definir junto con el corte de modelo (BA-004).
4. **No hay cambio de héroe dentro de una partida**, justamente porque solo puede existir uno por mundo — no
   hace falta diseñar sustitución/reasignación. Confirmado.
5. **Identidad de escuadra aniquilada y repuesta** — respuesta original correcta, sin cambios: `Escuadron.id`
   persiste con `cantidad` en 0 y se rellena reclutando del mismo `tropaId`; solo cambia el nombre del campo
   dueño (`jugadorId` → `heroeId`).

**Sigue pendiente (trabajo de implementación, no de diseño):** migrar cada `jugadorId` de dominio listado
arriba a `heroeId`, y decidir el detalle de datos propios de Conquest (clase, perks, equipo, `displayName`)
que se añaden a `Heroe` — eso se detalla en BA-004.

