# 02 — Contrato de comunicación servidor-cliente (v1)

**Estado:** ACEPTADO como diseño — los endpoints nuevos de batalla no existen todavía en `src/server/`.
Revisado y corregido tras `Docs/Coordinacion/propuestas/REVISION_CONTRATOS_CODEX_2026-09-11.md` (R01, R02,
R03, R06, R07, R08 y las correcciones menores de esa revisión).  
**Fecha:** 2026-09-11  
**Depende de:** `01_Modelo_de_datos_compartido.md` (mismo directorio) para las formas de `Heroe`,
`Escuadron`, `Batalla`, `BattleTicket`, `BattleResult`, `BattleServerAssignment`.

Este documento no reinventa convenciones: describe cómo se comunican Unity/Conquest y BronzeAge reutilizando
lo que la API HTTP/WS de BronzeAge ya hace hoy (`src/server/`), y solo añade lo nuevo que exige el ciclo de
batalla (BA-001).

## 1. Dos canales de comunicación, dos dueños

```text
Cliente Unity
  ├── HTTP/WS estratégico ───────► BronzeAgeFase0        (este documento)
  │                                 estado persistente y autoridad estratégica
  └── protocolo tiempo real ─────► Servidor de batalla Unity   (propiedad de Conquest, fuera de alcance)
                                    autoridad táctica de la partida real
                                              │
                                              └── BattleResult firmado ─────► BronzeAgeFase0 (§3)
```

BronzeAge nunca es parte del protocolo de tiempo real de la partida táctica — solo emite el `BattleTicket`
de entrada y recibe el `BattleResult` de salida, ambos por HTTP (§3).

## 2. Convenciones ya vigentes (reutilizadas sin cambios)

- **Prefijo `/v1`** en toda la API.
- **Login:** `POST /sesiones` con `Authorization: <esquema-proveedor> <credencial>` (ej. `dev ana`,
  `clave nick:contraseña`) → `{usuarioId, sesionId, expiraEn}`. El cliente guarda `sesionId` y lo presenta
  en cada petición posterior como `Authorization: sesion <sesionId>`.
- **Alta de cuenta local:** `POST /registro` (`nick`+`clave`, más `codigo` si la instancia lo exige).
- **Whoami:** `GET /sesiones/actual?gameId=...` → rol y `jugadorId` en esa partida.
- **Resolución de actor:** `Sesion -> Usuario -> Membresia -> jugadorId` (y desde ahí `Heroe`, cuando
  exista — doc 01 §1). Un comando nunca confía en un id que el cuerpo de la petición afirme tener.
- **Comandos de partida:** `POST /jugador/partidas/:gameId/comandos` (y su equivalente `/admin/...` con más
  privilegio), body `{tipo, params, idempotencyKey?}`. Éxito de TRANSPORTE (`HTTP 200`) no es lo mismo que
  éxito de DOMINIO: la respuesta trae `{..., resultado: {ok, ...}}`, y `ok: false` es el dominio diciendo
  que no (sin recursos, plaza ocupada...) — sigue siendo `HTTP 200`. Solo son error de transporte: `400`
  (cuerpo con forma inválida), `401` (sin sesión), `403` (autenticado pero sin permiso), `409` (fallo al
  PERSISTIR el resultado del comando). **Corrección de Codex (minor, R-final):** un `409` no significa "se
  aplicó en memoria y se perdió" — `server/runnerDePartida.ts` revierte al snapshot anterior si guardar
  falla, así que un `409` es un comando que NO llegó a tener efecto ninguno, ni en memoria ni en disco. Un
  cliente Unity debe tratar un `409` como "no ocurrió", nunca como "ocurrió pero no se sabe si se guardó".
- **Tiempo real (con BronzeAge, no con el servidor de batalla):** un único WebSocket por jugador con
  canales suscribibles. Cliente → `{"accion":"suscribir"|"desuscribir","canal":"..."}`. Servidor →
  `{"tipo":"suscrito"|"desuscrito"}` | `{"tipo":"error",...}` | `{"tipo":"evento","canal":...,"evento":...}`.
  Autenticación en el handshake (`preValidation`), antes de aceptar la conexión.

Ninguna de estas convenciones cambia para dar soporte a `Heroe`/`Escuadron`/`Batalla` — los comandos nuevos
de dominio (crear/gestionar `Heroe`, loadouts, escuadras) usan el mismo `POST .../comandos` con un `tipo`
nuevo en `REGISTRO_COMANDOS`, sujeto a la misma matriz de autorización que ya existe
(`Docs/Arquitectura/5_Contratos_Identidad_Permisos.md`).

## 3. Extensión para el ciclo de batalla (BA-001, nuevo)

**Esta sección se reescribió tras R01/R02/R03/R06 de `REVISION_CONTRATOS_CODEX_2026-09-11.md`** — faltaba
el mensaje de inicio, faltaba cómo el orquestador de Conquest recibe el ticket y cómo un participante
recupera su asignación si no estaba conectado cuando llegó el aviso por WS, y el canal colectivo no podía
entregar un secreto por destinatario. Corregido abajo.

### 3.1 Comandos de jugador (mismo mecanismo del §2)

`abrirBatalla`, `cancelarBatalla` — comandos normales de `POST /jugador/partidas/:gameId/comandos` (o
`/admin/...`), autorizados por la matriz existente. Equivalen a lo que hoy hacen
`iniciarAsedio`/`combateCampoAbierto`/`interceptarCaravana`: el lobby/convocatoria previo no pasa por aquí
(vive en `Ejercito`, sin persistencia nueva — doc 01 §15).

### 3.2 Entrega y recuperación del `BattleTicket` (Conquest ← BronzeAge) — corrección R02

Decir "BronzeAge publica el ticket" no bastaba sin una ruta que lo entregue de forma recuperable al
orquestador de Conquest (que puede no estar escuchando en el instante exacto en que se crea, o puede
reiniciar):

```text
GET /v1/batallas/pendientes        lista batallas en 'convocando' sin BattleServerAssignment todavía —
                                   el orquestador de Conquest hace polling; idempotente, sin efectos
GET /v1/batallas/:battleId/ticket    el BattleTicket vigente (huellaTicket/ticketRevision actuales) —
                                     recuperable en cualquier momento por el orquestador, no solo la
                                     primera vez
```

Ambos con la credencial servidor-a-servidor del §3.3 — nunca la de un jugador. Si BronzeAge reinicia entre
crear la `Batalla` y que el orquestador la recoja, `GET /pendientes` sigue devolviéndola: no depende de
ningún aviso en memoria que se pierda al reiniciar.

### 3.3 Endpoints servidor-a-servidor (Conquest → BronzeAge, NO de cliente jugador)

```text
POST /v1/batallas/:battleId/asignacion    Conquest reporta BattleServerAssignment
POST /v1/batallas/:battleId/inicio        Conquest confirma que la partida real EMPEZÓ — nuevo (R01)
POST /v1/batallas/:battleId/resultado     Conquest reporta BattleResult
```

**`/inicio` es la transición que faltaba (R01):** recibir una asignación no significa que la partida ya
esté jugándose — puede tardar en arrancar, o no llegar a arrancar. Este mensaje mueve `asignada -> en_curso`
y lleva `intentoAsignacionId` (doc 01 §15): solo el servidor que de verdad recibió ESA asignación puede
confirmar que arrancó. Sin este mensaje, un `BattleResult` llegaría exigiendo que la batalla estuviera ya
`en_curso` sin que nada la hubiera puesto ahí — el defecto que señaló R01.

Ninguno de los tres acepta la credencial de sesión de un jugador. Autenticación con un esquema nuevo,
`Authorization: batalla-servidor <token>`, verificado contra una lista de servidores autorizados en la
configuración del proceso — mismo patrón que `ADMINISTRADORES`/`ORIGENES_PERMITIDOS` en `server/index.ts`
(opt-in explícito: sin declarar la lista, ningún servidor de batalla puede reportar nada). Esta es la
separación de credenciales que exige la corrección de Codex (doc 01 §15): la credencial servidor-a-servidor
**nunca** es la misma que el token de entrada por participante del §3.4 — un token de jugador jamás puede
registrar una asignación, confirmar inicio ni firmar un resultado, y la credencial de servidor jamás se
reenvía a un cliente.

`battleId` es globalmente único (doc 01 §15, UUID) — estas rutas no necesitan `gameId` para resolver sin
ambigüedad (corrección R03).

`POST .../asignacion` e `POST .../inicio` deben además validar que `ticketRevision` en el cuerpo coincide
con la vigente en la `Batalla` — si `ticketRevision` cambió (sustitución de participante antes de empezar),
una asignación/inicio de la revisión vieja se rechaza con `409`.

**Dos orquestadores con la misma batalla (R02/R03, añadido 2026-09-13).** `GET /pendientes` puede devolver
la misma batalla a dos orquestadores a la vez; quien decide es el `POST .../asignacion`. BronzeAge acepta la
primera asignación válida para esa `battleId` y `ticketRevision` y rechaza las demás con `409`
(`batalla.ya_asignada`). Repetir el POST con el mismo `intentoAsignacionId` es idempotente: devuelve `200`
con la asignación ya aceptada. No hace falta un paso previo de reserva: todas las mutaciones de una partida
pasan por la misma cola serial del runner, así que dos asignaciones nunca se aplican a la vez. Como estas
rutas no llevan `gameId`, BronzeAge mantiene un índice `battleId → gameId` para encontrar la partida.

**Reintentos del servidor de batalla.** Conquest reintenta `asignacion`, `inicio` y `resultado`, espaciando
cada vez más los intentos, hasta recibir una respuesta definitiva: 2xx, o un 4xx que no sea transitorio. Es
lo que permite que una caída de BronzeAge no pierda un resultado. La tabla completa de transiciones, con los
vencimientos y lo que pasa en cada caída, está en doc 01 §15.

`POST .../resultado` sigue el flujo de idempotencia del doc 01 §16: mismo `resultId`+hash → `200` con el
resultado ya aplicado; mismo `resultId` con hash distinto → `409` (auditado). Solo se acepta si la
`Batalla` está `en_curso` (nunca `cancelada`/`aplicada`/`fallida`/`convocando`/`asignada`). Antes de
aplicar, valida en orden (checklist ampliado — corrección R03/R04/R05):

1. `ticketRevision` del payload coincide con la vigente;
2. todos los `heroeId`/`squadId`/objetivos pertenecen al `BattleTicket` congelado de esa revisión;
3. sin IDs duplicados ni entidades nuevas;
4. **completitud:** hay una entrada en `porEscuadra` por CADA `squadId` de las reservas del ticket, incluso
   las nunca desplegadas (doc 01 §15, corrección R05) — faltar una es rechazo, no omisión tolerada;
5. `supervivientesAlCierre + muertos == efectivosAutorizados` por escuadra (doc 01 §15) — cerrado sobre lo
   AUTORIZADO, no sobre `desplegados` (que es solo informativo);
6. ninguna cifra negativa ni por encima de lo autorizado;
7. ganador/razón compatibles con `BattleRules` del ticket;
8. fechas/duración coherentes;
9. `schemaVersion`/versión de build/balance autorizada;
10. la credencial que firma pertenece al `intentoAsignacionId` activo de esta `Batalla`;
11. `xpGanada` de cada héroe y escuadra es un entero ≥ 0 y no supera el tope por batalla de `BattleRules`
    si lo hay (la XP la calcula Unity desde 2026-09-13, doc 01 §15);
12. `botin` de cada héroe (doc 01 §15): solo en héroes con `participo: true`; cada `itemDefinitionId` existe
    en `versionCatalogoObjetos` del ticket; cantidades enteras > 0 y monedas enteras ≥ 0, por debajo del tope
    de `BattleRules` si lo hay; los objetos no superan `casillasInventarioLibres` del `HeroSnapshot`; cada
    `itemInstanceId` es nuevo en la partida.

Cualquier fallo de 1-12 responde `409` con el detalle — nunca se aplica una consecuencia parcial (mismo
principio del §2: o se aplica entero, o no ocurrió nada).

### 3.4 Lectura y recuperación de la propia asignación (jugador) — corrección R02/R06

**Corrección de Codex: el canal WS colectivo NO puede entregar un secreto por destinatario (R06).**
`hub.difundir` (`src/server/difusion/hub.ts`) serializa un evento UNA vez y lo manda igual a todos los
suscriptores del canal — perfecto para estado público, inservible para repartir un token distinto por
persona. El token de entrada nunca viaja por el canal colectivo de batalla.

```text
GET /jugador/partidas/:gameId/batallas/:battleId/asignacion
```

Autenticado igual que cualquier ruta de jugador (`Authorization: sesion <id>` → `heroeId` resuelto de la
membresía). Devuelve el token de ESE `heroeId` si la `Batalla` ya tiene `BattleServerAssignment` y ese
`heroeId` es participante — `404`/cuerpo vacío si todavía no hay asignación. **Recuperable**: un jugador que
no estaba conectado cuando se emitió el evento WS lo pide aquí sin depender de haberlo visto pasar en
tiempo real.

**Responsabilidad de verificación (resuelve la ambigüedad que señaló R06):** BronzeAge NUNCA verifica la
validez criptográfica del token — solo custodia el que Conquest le entregó en `BattleServerAssignment` y
controla QUIÉN puede leerlo (el `heroeId` correcto, autenticado por sesión). La verificación del token al
conectarse al servidor de batalla real es, siempre, responsabilidad de Conquest.

### 3.5 Canal de tiempo real — solo estado PÚBLICO de batalla

Canal WS `batalla/<battleId>`, mismo protocolo del §2. Lleva ÚNICAMENTE eventos sin secretos: `asignada`
(avisa que ya hay instancia — el cliente entonces llama a §3.4 para el token), `en_curso`, `aplicada`,
`cancelada`, `fallida`. Nunca lleva `tokensParticipante` ni ninguna credencial — eso es exclusivamente
§3.4.

## 4. Extensión de lectura estratégica (Unity como reemplazo de Vite) — nuevo, corrección R08

**Corrección de Codex: este documento solo enumeraba login y comandos de batalla — Unity no puede
reemplazar al cliente Vite con solo las entidades nuevas (`Heroe`/`Escuadron`/`Batalla`).** El cliente
estratégico ya consume una secuencia completa que este documento debe incluir explícitamente (rutas reales
de `src/server/rutas/jugador.ts`, ya consumidas por `BronzeAgeClient/src/apiCliente.ts`):

```text
POST /jugador/partidas/:gameId/membresia    unirse a una partida (crea/recupera Membresia+jugadorId)
GET  /jugador/partidas/:gameId               proyección filtrada del estado (con Heroe/heroeId cuando exista)
GET  /jugador/partidas/:gameId/eventos?desde=<version>
                                             eventos con version > desde, filtrados como la proyección
                                             (cursor ya existente, Fase C13)
GET  /jugador/partidas/:gameId/mapa/:mapaId   recursos de mapa (worldgen, doc 01 §11/§18)
```

**Reconexión (ya funciona así hoy).** El cliente guarda el `version` de la última proyección o evento que
recibió. Al reconectar el WebSocket, vuelve a suscribirse a sus canales (las suscripciones no sobreviven a
un cierre) y pide `GET .../eventos?desde=<ese version>` para recuperar lo que se perdió; si el hueco es
grande, le basta con pedir la proyección entera otra vez. El WebSocket solo avisa: nunca es la única copia de
un dato.

Estas rutas NO cambian de forma para dar soporte a `Heroe` — solo su contenido crece (`jugadorId` sigue
existiendo en la respuesta; `heroeId` se añade cuando el modelo de héroe esté implementado). Unity debe
implementar este flujo completo (membresía → proyección → mapa → eventos con cursor de reconexión), no solo
los endpoints de batalla, antes de poder sustituir a Vite en cualquier partida real.

**Visibilidad — corrección de permiso, no solo de formato (R08):** la proyección de jugador
(`session/proyecciones/jugador.ts`) filtra el interior detallado de un asentamiento por
`asentamientosPropios` (residencia/ciudadanía), NO por "el héroe está físicamente ahí en este instante". Ver
doc 01 §19 para el detalle — este documento no amplía ese permiso al llevarlo a Unity: el DTO que sirve
Unity respeta la misma regla de acceso que ya sirve a Vite hoy.

### 4.1 Qué añade el modelo de héroe a la proyección de jugador (R08, añadido 2026-09-13)

La proyección actual (`proyectarParaJugador`, `session/proyecciones/jugador.ts`) ya trae, entre otros:
`gameId`, `instante`, `version`, `jugadorId`, `faccionId`, `mapaId`, `estadoMapa`, `facciones`,
`asentamientos` (solo el interior del asentamiento propio en el que está), `asentamientosAvistados`,
`asentamientosConocidos`, caravanas y ejércitos propios y avistados, `acuerdos`, `ordenes`, `relaciones`,
`titulos`, `caminos`, `campamentosBandidos`, `historial`, `zonas`, `zonasFusionadas` y
`trazadoPorAsentamiento`. Esa forma se mantiene. El modelo de héroe añade:

```text
heroe                  el héroe propio completo (doc 01 §12), con:
  escuadrones[]        todas sus escuadras, con contenedor, enGuarnicion y reservaBatalla
  loadouts[]           cada uno con su liderazgoTotal, DERIVADO al servir (no se persiste)
  cupoGuarnicion       DERIVADO: cupo en el asentamiento donde reside (0 si es huérfano)
batallas[]             batallas en las que participa, solo estado público (el token va por §3.4)
```

De los héroes ajenos que el jugador puede ver (en columnas y ejércitos avistados, o en su mismo
asentamiento), la proyección trae solo su parte pública (decisión del usuario, 2026-09-13; canon Doc
5.16.7):

```text
HeroePublico
  heroeId
  displayName
  classDefinitionId
  nivel
  heridoHasta?           si está herido, hasta cuándo
  escuadrasQueLleva[]    solo las que lleva consigo: { tropaId, cantidad, nivel }
  equipamiento           por hueco, la definición del objeto que lleva puesto (no su inventario)
```

Nada más del héroe ajeno viaja al cliente: ni experiencia, puntos, atributos, perks o Liderazgo, ni su
residencia, los escuadrones de su campamento, sus loadouts, su inventario o sus monedas, ni género, avatar o
si es humano o bot.

### 4.2 Comandos del héroe (nuevos, mismo mecanismo del §2)

| Comando | Parámetros | Devuelve (`resultado.datos`) | Rechazos de dominio |
|---|---|---|---|
| `crearHeroe` | `displayName`, `classDefinitionId`, `genero`, `avatar` | `{ heroeId }` | ya tiene héroe en esta partida; clase inexistente |
| `repartirPuntos` | `atributos?` (atributo → puntos), `perks?` (ids) | — | sin puntos suficientes en esa bolsa; perk no disponible |
| `equipar` | `slot`, `itemInstanceId` de un objeto de su inventario, o `null` para desequipar | — | objeto que no es suyo; hueco no válido; el objeto no va en ese hueco; sin casilla libre para lo que sale del hueco |
| `guardarLoadout` | `loadoutId?`, `displayName`, `squadIds[]`, `perksSeleccionados[]`, `activo?` | `{ loadoutId, liderazgoTotal }` | escuadra que no es suya; supera su Liderazgo |
| `borrarLoadout` | `loadoutId` | — | no existe |
| `asignarGuarnicion` | `squadId` | — | no reside aquí; escuadra fuera de su campamento; supera el cupo |
| `retirarGuarnicion` | `squadId` | — | no está en guarnición |

- Una `Membresia` sin héroe no puede hacer nada más en la partida hasta crearlo. Crear el héroe es un
  comando aparte de `POST .../membresia` porque necesita datos del jugador (nombre, clase, aspecto).
- `equipar` saca el objeto del inventario y lo pone en el hueco; lo que ocupaba el hueco vuelve a una casilla
  libre (doc 01 §12.1). Qué va en cada hueco lo dice el catálogo de objetos de Conquest.
- Trasladar el campamento es el `cambiarResidencia` que ya existe (Doc 2.5): no hace falta un comando nuevo.
- Salir con un loadout reutiliza `salirAlMundo` y `movilizarEjercito`, que pasan a aceptar un `loadoutId`
  además de la lista de escuadras.
- Como el resto de comandos de `/jugador/*`, todos devuelven en la misma respuesta la proyección propia
  actualizada, y los rechazos de dominio salen como `resultado.ok: false` con su `codigoError`.

## 5. Formato de error y versión

- Todo DTO de contrato (`BattleTicket`, `BattleResult`, proyecciones de `Heroe`/`Escuadron`) lleva
  `schemaVersion`. Una versión no reconocida se rechaza con `409` y cuerpo
  `{error, schemaVersionEsperada}` — no es un error de forma (`400`), es una incompatibilidad semántica que
  el emisor debe corregir actualizando su cliente/build, no reintentando.
- **Corrección de Codex (minor):** ese cuerpo NO es simplemente "reutilizar `ERROR_RESPUESTA`" — el
  `ERROR_RESPUESTA` de `server/rutas/esquemas.ts` hoy solo declara `error`, y Fastify recorta del cuerpo
  cualquier propiedad no declarada en el schema de respuesta al serializar (la cabecera de ese archivo ya lo
  advierte). Hace falta un schema NUEVO, `ERROR_VERSION_INCOMPATIBLE` (`{error, schemaVersionEsperada}`,
  ambas `required`), declarado explícitamente en las rutas de batalla — no basta con la intención de
  "reutilizar" sin extender el schema real.

## 6. Fixtures y schemas formales

Este documento es la narrativa del contrato; la fuente verificable que Conquest debe poder deserializar en
C# vive en código, ubicación acordada en BA-004:

```text
src/contratos/v1/
  *.schema.json       JSON Schema versionado (heroe, escuadron, loadout, battleTicket, battleResult,
                       battleServerAssignment)
  fixtures/*.json      fixtures dorados, uno por entidad de contrato
  dto.ts                tipos TS derivados, consumidos por server/ al serializar
```

Cada schema publicado necesita al menos un fixture válido y una prueba que lo lea en ambos lados (regla §5.8
del modelo de cooperación) — pendiente de escribir junto con el código, no en este documento.

## 7. Pendiente de definir al implementar

- Formato exacto del token de entrada por participante (JWT firmado vs credencial opaca resuelta contra
  BronzeAge) — decisión de infraestructura, no bloquea el contrato de dominio de este documento. Su
  `expiraEn` sí está decidido: tiempo real UTC, nunca `Instante` de mundo (doc 01 §0/§15, corrección R07).
- Prefijo de ruta exacto de `/v1/batallas/*` — aquí propuesto sin prefijo `/admin`/`/jugador` porque no es
  ninguno de los dos roles (es servidor-a-servidor), análogo a cómo `/sesiones` tampoco lleva prefijo de
  superficie.
- Rotación/revocación de la credencial servidor-a-servidor si una instancia de Conquest se compromete —
  fuera de alcance de este documento, es política operativa.
- Verificabilidad del worldgen híbrido (fixtures de paridad TS/C# vs. datos canónicos servidos) — doc 01
  §18, todavía sin decidir cuál de las dos vías se adopta.
