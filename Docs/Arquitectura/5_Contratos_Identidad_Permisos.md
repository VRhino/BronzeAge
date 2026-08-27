# Contratos de identidad y permisos (diseño, Fase A6)

Es la Fase A6 del [roadmap](3_Plan_Evolucion_Roadmap.md): define las entidades externas al motor que la Fase B
(backend provisional) y la Fase C (multijugador) necesitan, y la matriz de autorización sobre la superficie de
comandos que `GameStore` ya expone hoy. Contexto y motivación: [2_Estudio_Evolucion_Backend_Multifrontend.md](2_Estudio_Evolucion_Backend_Multifrontend.md), sección "1. Definir contratos de partida, identidad y permisos".

> **Ya no es "solo documento" (2026-08-25/26).** Se escribió como diseño puro cuando nada de esto existía; hoy
> las entidades (`Usuario`, `Sesion`, `Jugador`, `Membresia`) y la matriz de autorización de comandos están
> **implementadas** (hitos C1/C2 del roadmap, `session/comandos/autorizacion.ts`). Las notas que dicen "hoy
> ninguna de estas condiciones se comprueba" (más abajo, junto a la matriz) describen el estado en el momento
> de escribir el diseño, no el estado actual — quedan como registro de la intención original, no como
> descripción vigente. Desajuste real entre este diseño y la implementación, encontrado en vivo y **corregido
> el 2026-08-26** (hito C8): la fila `alternarFaccionNpc` de abajo dice "sin restricción si es admin", pero
> `rolEnPartida` cortocircuitaba a `administrador_global` para cualquier actor con `esAdministradorGlobal` sin
> mirar su `Membresia` real — esa fila de la matriz solo admite `['jugador', 'administrador_partida']`, nunca
> `administrador_global`, así que el comando devolvía 403 siempre para un admin. Se invirtió el orden: la
> `Membresia` manda. Ver hito **C8** del roadmap.

## Punto de partida: qué existe hoy

Hoy no hay ninguna de estas entidades. Lo que existe es:

- `jugadorId`: una cadena libre que la UI decide y pasa directamente al motor (`GameStore.fundarAsentamiento(faccionId, posicion, numJugadores)` genera internamente ids como `jugador-<faccionId>-<n>`). No hay cuenta, sesión ni autenticación detrás.
- `Asentamiento.jugadoresFundadoresIds` / `Asentamiento.casasCompradas`: quién "reside" en un asentamiento — de ahí sale quién puede reclutar tropa o formar parte de la ciudadanía.
- `Asentamiento.cargos` (`CargosAsentamiento`): gobernador, tesorero, general, maestroObras, sacerdote — cargos LOCALES de un asentamiento, asignables a cualquier `jugadorId`.
- `Faccion.reyId` / `Faccion.embajadorId`: cargos a nivel de Facción.
- `Faccion.ciudadanosIds`: jugadores con ciudadanía en esa Facción.

Todo esto son **reglas de dominio del juego** (Doc 2, "cargos del juego... son reglas de dominio"). Este
documento no las reemplaza ni las toca — añade una capa de identidad/autorización TÉCNICA por delante de
ellas, que hoy no existe.

## Entidades nuevas

### Usuario

Cuenta autenticable, externa a una partida concreta.

```text
Usuario
  id            (uuid, estable)
  email o proveedor de identidad
  creadoEn
  ¿deshabilitado?   — para banear una cuenta sin borrar su historial
```

Un `Usuario` no pertenece a ninguna partida por sí solo — la pertenencia vive en `Membresia`.

### Sesion

Credenciales activas de un `Usuario` en un momento dado.

```text
Sesion
  id
  usuarioId
  emitidaEn
  expiraEn
  contexto de acceso   (ej. IP/user agent, para auditoría — no para autorización)
```

La API resuelve `Sesion -> Usuario` en cada request; nunca confía en un `usuarioId`/`jugadorId` que el
cliente afirme tener sin pasar por esto (Doc 2, principio 3: "actor autenticado").

### Jugador

Identidad DENTRO de una partida — sustituye al `jugadorId` de cadena libre actual por una entidad real, con
relación a `Usuario` y a `Faccion`.

```text
Jugador
  id             (mismo valor que hoy usa el motor como jugadorId — no cambia el dominio)
  gameId
  usuarioId
  faccionId      (a qué Facción pertenece en ESTA partida; null antes de unirse a una)
  creadoEn
```

Un `Usuario` puede tener como máximo un `Jugador` por partida (`gameId`) — es la partida la que aísla
identidades, no el usuario. El mismo `Usuario` puede jugar varias partidas simultáneas, cada una con su
propio `Jugador`.

### Rol

Roles TÉCNICOS de acceso — deliberadamente separados de los cargos de juego (gobernador, tesorero, general,
maestroObras, sacerdote, rey, embajador), que son dominio, no autorización de infraestructura.

```text
Rol =
  | 'jugador'              — un Usuario con un Jugador activo en la partida
  | 'administrador_partida' — gestiona ESA partida (equivalente a la consola de admin actual, GameStore)
  | 'moderador'            — subset de administrador_partida, sin acceso a balance/regeneración de mundo
  | 'administrador_global' — gestiona instancias/partidas, no un jugador de ninguna
  | 'observador'           — solo lectura, proyección pública
  | 'servicio_npc'         — el propio backend actuando como NPC de gobernanza (`avanzarNpcGobernanza`), no un Usuario humano
```

Un `Usuario` puede tener distintos roles en distintas partidas (jugador en una, administrador en otra). El
rol no sustituye los cargos de juego: un `administrador_partida` no puede reclutar tropa en un asentamiento
ajeno salvo que además sea su `Jugador` residente con el cargo correspondiente — separar ambos evita que
"tener acceso técnico" se confunda con "tener autoridad dentro del juego".

### Partida

Ya descrita en el documento 2 (sección "Alcance de la etapa provisional de backend"); se repite aquí solo el
subconjunto relevante para identidad/permisos.

```text
Partida
  gameId
  estado (creando | activa | pausada | finalizada)
  configuracion / semilla / versión de reglas y worldgen
  administradoresGlobalesConAcceso[]   — opcional, para instancias multi-tenant
```

### Membresia

La relación que de verdad se consulta en cada chequeo de autorización — une `Usuario`, `Jugador`, `Partida`,
`Facción` y `Rol` con vigencia.

```text
Membresia
  usuarioId
  gameId
  jugadorId       (null si el rol no requiere Jugador, ej. administrador_global)
  rol
  desde / hasta    (revocable sin borrar historial — "hasta" ausente = vigente)
```

> **`faccionId` retirado de `Membresia` y de `Jugador` (2026-08-25).** El diseño original los incluía. Al
> implementarlos se vio que duplicaban un hecho cuya única fuente de verdad es `Faccion.ciudadanosIds`, que
> el motor muta en `otorgarCiudadania`, `anexionar` y `fusionar`. Tras una anexión, los ciudadanos de la
> Facción absorbida pasan a la absorbente en el estado del juego, pero la copia de la capa de acceso seguiría
> apuntando a una Facción que ya no existe — y la autorización decidiría sobre datos fantasma. La Facción del
> actor se DERIVA del estado de partida en cada chequeo (`session/comandos/autorizacion.ts`). Un campo que no
> hay que creerse es peor que no tenerlo.

## Matriz de autorización por comando

Basada en la superficie real de comandos que `GameStore` expone hoy (`src/app/gameStore.ts`), no en una lista
abstracta — así la matriz es directamente el contrato que un futuro middleware de autorización debe cumplir
antes de invocar cada método equivalente en `GameSession` (Fase B2).

Columnas: rol técnico requerido como mínimo, más la condición de dominio (cargo/residencia) que además debe
cumplirse. "Facción propia" significa `Jugador.faccionId` del actor debe coincidir con la Facción objetivo;
"residente" significa `jugadorId` ∈ `jugadoresFundadoresIds ∪ casasCompradas` del asentamiento objetivo.

| Comando (`GameStore.*`) | Rol técnico mínimo | Condición de dominio adicional |
|---|---|---|
| `fundarAsentamiento`, `lanzarCaravanaFundacion`, `desarmarCaravanaFundacion` | jugador | Facción propia |
| `crearFaccion` | jugador | **RESUELTO 2026-08-27** (antes "abierto"): no ser ya ciudadano de ninguna Facción, y no haber abandonado una hace menos de `CIUDADANIA.cooldownCreacionFaccionDias` (7 días) — ambas son rechazo de DOMINIO dentro del propio comando (`faccion.ya_pertenece`/`faccion.cooldown_creacion`), no de esta matriz: cualquier `jugador` puede intentarlo, igual que nombre vacío/duplicado. Otorga ciudadanía inmediata a quien la crea |
| `unirseAFaccion` | jugador | no ser ya ciudadano de OTRA Facción (`faccion.ya_pertenece`); ya ciudadano de la misma es idempotente. Sin cooldown — solo `crearFaccion` lo tiene |
| `dejarFaccion` | jugador | ser ciudadano de alguna (`faccion.no_pertenece` si no); sin parámetros, solo puede dejar la PROPIA. Libera Rey/Embajador si los ocupaba; NO libera residencia ni cargos locales (limitación documentada en Doc 2.5) |
| `alternarFaccionNpc` | jugador (rey) o administrador_partida | Facción propia si es jugador; sin restricción si es admin |
| `asignarRey`, `asignarEmbajador` | jugador | Facción propia, cargo de rey vigente (o primera asignación) |
| `asignarCargoLocal` | jugador | residente del asentamiento, con el cargo que otorga ese poder según reglas de dominio ya existentes |
| `comprarCasa` | jugador | Facción propia del asentamiento objetivo (ciudadanía) |
| `activarPolitica`, `anadirEdificioManualmente`, `quitarDeCola`, `moverEnCola`, `mejorarEdificioAhora`, `pausarAutoConstruccion`, `reanudarAutoConstruccion`, `calibrarReservaManual` | jugador | residente + cargo exigido por el comando (`gobernador`/`maestroObras`, ya validado hoy por el motor) |
| `renombrarAsentamiento` | jugador | residente (o cargo específico — abierto, hoy no lo exige el motor) |
| `romperRelacion`, `rebelionVasallo`, `anexionar`, `fusionar` | jugador | Facción propia, cargo de rey/embajador según ya exige `engine/diplomacia.ts`/`faccion.ts` |
| `proponerRelacion` | jugador | Facción propia de `faccionAId` (quien propone), cargo de rey/embajador — **fila añadida 2026-08-25**: no estaba en la versión original de esta tabla, aunque el comando ya existía; implementada en `server/autorizacion/matriz.ts` por analogía con `romperRelacion` |
| `proponerTrueque` | jugador | residente de `asentamientoAId` (quien propone) — **fila añadida 2026-08-25**, mismo motivo que `proponerRelacion` |
| `colocarOrdenMercado`, `crearCaravana`, `reclutarTropa` | jugador | residente del asentamiento objetivo |
| `iniciarAsedio`, `combateCampoAbierto`, `interceptarCaravana`, `atacarCampamentoBandidos` | jugador | residente del asentamiento atacante, escuadrones propios del jugador o de otros residentes autorizados |
| `avanzarTick` | administrador_partida (etapa provisional) o scheduler del servidor | — (Doc 2: "el servidor sigue siendo quien avance y resuelva los ticks"; en producción no es un comando de cliente) |
| `regenerarMundo`, `importarSimulacion`, `exportarSimulacion`, `exportarMapaUnity` | administrador_partida | — |
| `getBalance`, `actualizarBalance`, `restaurarBalance` | administrador_partida | — (Doc 2, punto 8: balance debe pasar a versionado por partida/temporada, auditado) |
| Consultas de solo lectura (`getState`, `getZonas`, `getLigas`, `viabilidadFundacion`, `produccionInfo`, `manoObraInfo`, `nivelAsentamientoInfo`, etc.) | observador | Proyección según Doc 2 punto 7 — un `observador`/`jugador` NO debe recibir el mismo payload que `administrador_partida`; la proyección exacta por rol queda para cuando se diseñen los DTOs (Fase C) |

Notas sobre la matriz:

- Hoy **ninguna** de estas condiciones se comprueba — `GameStore` confía en los ids que le pasa la UI (Doc 1,
  sección "Comunicación, identidad y permisos"). Esta tabla es el contrato que un middleware de autorización
  de la Fase C debe implementar, no una descripción de lo que ya pasa.
- Los comandos de combate ya validan alguna condición de dominio dentro del motor (ej. `iniciarAsedio` exige
  `atacante.cargos.generalId`), pero eso es una regla de JUEGO (¿hay General nombrado?), no una comprobación
  de que el actor autenticado sea realmente ese jugador — son capas distintas y ambas hacen falta.
- `servicio_npc` no aparece en la tabla porque no pasa por esta autorización de cliente: es el propio backend
  invocando `avanzarNpcGobernanza` internamente, ya cubierto por el rol `administrador_partida`/scheduler.

## Diferencia entre rol técnico y cargo de juego

Confusión concreta a evitar (Doc 2 lo advierte explícitamente): un jugador que es **Gobernador** de un
asentamiento no tiene por eso ningún privilegio técnico — sigue siendo rol `jugador`. Un **administrador de
partida** no hereda ningún cargo de juego — no puede reclutar tropa en un asentamiento ajeno solo por tener
acceso técnico completo; para actuar como jugador necesitaría además un `Jugador`/`Membresia` propios con el
cargo correspondiente. Las dos jerarquías son ortogonales:

```text
Eje técnico (este documento):     jugador -> moderador -> administrador_partida -> administrador_global
Eje de juego (ya existe, dominio): residente -> {gobernador|tesorero|general|maestroObras|sacerdote} -> rey/embajador de Facción
```

## Preguntas abiertas

- ¿Un `Usuario` puede tener más de un `Jugador` en la misma partida si ya tiene Facción (ej. una segunda
  cuenta de personaje)? Se asume que no (1:1 Usuario↔Jugador por partida) hasta que se decida lo contrario.
- ¿`crearFaccion` debe limitarse a un Jugador sin Facción todavía, o cualquier Jugador puede fundar una
  Facción nueva y abandonar la anterior? Hoy el motor no lo restringe.
- La condición exacta de `renombrarAsentamiento` (¿cualquier residente o solo el gobernador?) no está definida
  por el motor actual — queda pendiente de decidir junto con el resto de comandos "menores" sin cargo exigido.
- Multi-tenancy (`administrador_global` gestionando varias `Partida`) es opcional para la Fase B/C mínima —
  una única instancia por partida (Doc 2, "Alcance de la etapa provisional de backend") no lo necesita todavía.
