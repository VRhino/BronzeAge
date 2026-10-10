# 03 — Tiempo real por segmentos de vista (contrato v1, extensión aditiva)

**Estado:** IMPLEMENTADO en el servidor (Backend, commits `2ab3b11..fe78215`, 2026-10-10, sin push): canales `admin/*`, `vista/mapa`,
`vista/heroe`, `vista/asentamiento/<id>`, con foto/delta/resync/revocado, coalescer, backpressure, métricas y schemas. Falta la Fase 4
(`desde`) y medir con 500 bots. Los fixtures de [`fixtures/tiempo-real/`](fixtures/tiempo-real/) los cumple el servidor. Las **desviaciones
del diseño inicial** están marcadas «(implementado así)» y resumidas en §12. Sustituye al sondeo del estado
completo cada 3-5 s que hacen hoy el cliente admin y el cliente jugador.
**Depende de:** `02_Contrato_Comunicacion_Servidor_Cliente.md` §2 (WS, `evento`, `tick`) y §4 (proyección), y de
`01_Modelo_de_datos_compartido.md` para las entidades. **Compatibilidad v1:** solo se AÑADEN canales, mensajes y
campos; nada de lo existente cambia de forma ni de significado (§8).

## 0. Resumen en 12 líneas

1. El cliente se suscribe, por el WebSocket que ya tiene, a **canales de vista** (`vista/mapa`, `vista/heroe`,
   `vista/asentamiento/<id>`; para el admin, `admin/partida`, `admin/asentamiento/<id>`, `admin/campamento/<id>`).
2. Al suscribirse recibe una **foto** con `version`; después **deltas encadenados** `desde → hasta`.
3. Un delta lista, por tipo de entidad e `id`, qué se **inserta/sustituye** (`upsert`), qué se **retoca** (`patch`) y
   qué **desaparece** (`remove`), más los escalares que cambian (`instante`…). El cliente toca solo eso.
4. Lo móvil manda su `ruta` una vez al salir y luego solo `progreso` + `avancePorTick` (propios) o
   `desplazamientoPorTick` (ajenos): el cliente interpola a 24-60 fps sin preguntar nada.
5. Hueco de versión, reconexión o socket lento ⇒ **foto de nuevo** (`resincronizar` / `resync`). Nunca hay reparación
   parcial.
6. El GET HTTP de la proyección y el del estado admin **siguen existiendo** como foto inicial y respaldo; la foto de
   un canal tiene los mismos campos con los mismos nombres que la proyección, así que el almacén del cliente es el mismo.
7. El diff se calcula **una vez por canal y flush**, no por suscriptor, cuando varios ven lo mismo; el mapa con niebla
   se agrupa por **clave de visibilidad** (Facción + aliados que comparten vista), recomendado frente al diff por
   suscriptor (§6.2).
8. Siempre se hace el diff de dos fotos **ya filtradas** para esa audiencia: lo que no se puede ver no entra nunca en
   ninguna de las dos (§7).
9. Backpressure: un socket lento no acumula cola; se salta y se le manda foto cuando se recupera, o se cierra (§6.3).
10. La autorización se evalúa al suscribirse y **se reevalúa en cada flush** (§7.2); si se pierde, `revocado`.
11. Los canales de EVENTOS (`mapa/general`, `asentamiento/<id>`, `batalla/<id>`, `heroe/<id>`) y el mensaje `tick`
    **se mantienen** sin cambios; `tick` se depreca cuando ambos clientes hayan migrado (§8).
12. Migración en 4 fases (§8.2); cada una es desplegable sola y deja el sondeo como respaldo.

## 1. Problema medido y objetivos

El cliente admin pide el estado completo (`GET /admin/partidas/:gameId`, `GameSessionState` sin `mapa`) y el cliente
jugador pide la proyección completa (`GET /jugador/partidas/:gameId`, ~50 campos, ver `session/proyecciones/jugador.ts`)
cada pocos segundos y repintan todo. Con N clientes el servidor construye y serializa N estados completos por ronda de
sondeo, aunque en un tick solo se muevan unas pocas columnas. Hoy el WS solo difunde EVENTOS y un `tick` sin datos que
manda volver a pedir la proyección entera por HTTP (02 §2).

Objetivos (medibles, §6.4): bytes por cliente y minuto ≥ 10× menos que el sondeo en régimen estacionario; coste de CPU
del servidor proporcional a lo que CAMBIA y al número de claves de visibilidad, no al número de clientes; movimiento
fluido de lo móvil sin más tráfico que un `progreso` por tick.

**No objetivos:** no es el protocolo de la batalla táctica (Unity, 02 §1); no cambia ningún comando (siguen por
`POST .../comandos`, que devuelve la proyección propia como hasta hoy); no cambia reglas de juego.

## 2. Catálogo de canales de vista

Los canales de vista se suscriben igual que los de eventos (`{"accion":"suscribir","canal":"..."}`) pero llevan
**estado**, no hechos. Conviven con los de eventos (§8.1): el evento cuenta lo que pasó (para la crónica, avisos,
animaciones); el delta mantiene el estado al día.

| Canal | Quién | Contiene | Clave de agrupación (§6.2) |
|---|---|---|---|
| `vista/mapa` | jugador con membresía (**implementado: clave = el héroe**, §6.2) | Lo del mundo que su Facción (y los que comparten su vista) ve bajo la niebla: posiciones/progreso de lo móvil, asentamientos avistados/conocidos, territorio, batallas visibles, campamentos y Aedas a la vista | *diseño:* `faccionId` + firma de aliados; *implementado:* el `heroeId` (§6.2) |
| `vista/heroe` | jugador, solo su héroe | Lo personal: `heroe`, `ejercitos` propios con su `ruta`, `caravanas` propias, `convocatorias`, informes, miradas, mercado de campamento, tecnología… | el `heroeId` (no se comparte) |
| `vista/asentamiento/<id>` | ciudadano de la Facción dueña **y presente dentro de la plaza** (misma regla que `asentamientos[0]` de la proyección; Doc 1.10.1: la ciudadanía habilita, la presencia ejerce) | El interior de esa plaza: escalares de `asentamiento`, `edificios` (con las obras: `estado: 'en_construccion'`/`'en_cola'` y `completaEn`), almacén, población, cargos, residentes, colas de reclutamiento, mercado local, muralla | el `asentamientoId` (todos los ciudadanos ven lo mismo) |
| `admin/partida` | administrador (`exigirAdministracion`) | La partida completa SIN niebla en versión **ligera** (§2.1) | la partida (un solo diff) |
| `admin/heroe/<id>` | administrador | Detalle de un héroe: `historial` y, si pesa, `almacenPersonal` | el `heroeId` |
| `admin/asentamiento/<id>` | administrador | El `Asentamiento` completo, sin niebla | el `asentamientoId` |
| `admin/campamento/<id>` | administrador | El campamento de mercenarios completo (mercado, residentes, fondos) | el `campamentoId` |

Qué va exactamente en cada bloque (nombres = los de `ProyeccionJugador` / `EstadoAdmin`, que son la fuente):

- **`vista/mapa`:** `ejercitosAvistados`, `caravanasAvistadas`, `asentamientosAvistados`, `asentamientosConocidos`,
  `aedasAvistados`, `batallas`, `campamentosBandidos`, `campamentosMercenarios`, `alijos`, `heroesVisibles`,
  `facciones`, `zonas`, `zonasFusionadas`, `caminos`, `territorioPorEjercito`, `exploracion`, `relaciones`, `titulos`,
  `nombresDeCompaneros`, `nombresDeDirigentes`, escalares `instante`, `version`, `mapaId`, `estadoMapa`.
- **`vista/heroe`:** `heroe`, `ejercitos`, `caravanas`, `convocatorias`, `miConvocatoriaId`, `informesDeInspeccion`,
  `miradasIntel`, `informesPlaza`, `tarifasIntel`, `propuestas*`, `acuerdos`, `ordenes`, `tecnologia`,
  `aedasResidentes`, `mercadoCampamento`, `escenaCampamento`, `historial`, `ascensoDeAsentamiento`,
  `produccionDeAsentamiento`, `preciosReferencia`.
- **`vista/asentamiento/<id>`:** los campos de `Asentamiento` que la proyección sirve para la plaza propia
  (`asentamientos[i]` con ese `id`), descompuestos así: escalares en la entidad `asentamiento`, y `edificios` como
  entidades con `id`. El trazado (`trazadoPorAsentamiento[id]`) viaja como bloque sustituible (cambia solo con
  `layoutVersion` o al construir).
- Lo estático (`mapa/:mapaId`, catálogos de `GET /balance`, `catalogoTropas`) **no** viaja nunca por estos canales.

### 2.1 Versión ligera de `admin/partida` (acordada con Cliente admin, 2026-10-10)

Las listas globales del admin (Registro/Militar, aviso de mantenimiento, mapa) derivan de TODOS los asentamientos, así que
quitar solo `edificios`/`almacen` no basta. Contenido:

- `asentamientos`: todos los escalares + `edificios` **solo de ámbito `'mapa'`** (el canvas dibuja murallas/torres) + un
  bloque nuevo **`resumen`** calculado en servidor con las funciones del motor:
  `{ edificiosActivos, edificiosEnCurso, almacenClave: { madera|piedra|trigo|oro: { cantidad, capacidad } },
  mantenimientoIncumplido, ocupada, poderMilitar, tieneFundicion, tieneGranFundicion }`. Sin `almacen` completo, sin
  `recintos`, sin `edificios` internos (van en `admin/asentamiento/<id>`). Así un tick no parchea el almacén de todas las
  plazas; solo cambia `resumen` si cambia algo de eso. (Alternativa descartada: edificios compactos + almacén completo.)
- `heroes`: todo salvo `historial` (con `escuadrones`, `ubicacion`, `desconectaEn`, `fuera`, `controlador`, `displayName`,
  `oroDeBotin`, `racionEn`, `almacenPersonal`). Detalle en `admin/heroe/<id>`.
- `campamentosMercenarios`: todo salvo `mercado` y `fondos` (van en `admin/campamento/<id>`); sí `residentesIds`, `edificios`,
  `poblacion`, `poblacionEn`, `bandidosEn`, `origen`, `creadoEn`, `eligieronComoInicial`.
- Completos y sin recortar: `ejercitos` (con `ruta`/`progreso`/`avancePorTick`), `caravanas` (con escolta), `facciones`
  (con sigilo), `relaciones`, `batallas`, `campamentosBandidos`, `aedas`, `titulos`, `tecnologia`, `red`/`caminos`,
  `estadoMapa` y los escalares (`mapaId`; `instante` y `version` van en la cabecera).

  Definición implementada de `resumen`: `mantenimientoIncumplido = medidorMantenimiento < MANTENIMIENTO.medidorInicial`; `poderMilitar` = suma del
  poder de las escuadras de los residentes en su campamento (no cuenta las que andan en columnas); el resto sale de `edificios` y `almacen`.

> Si un campo de la proyección no aparece arriba, pertenece a `vista/heroe` por defecto. El reparto fino es decisión
> de Backend (§10) siempre que cada campo viaje por UN solo canal de vista y la unión de los tres reproduzca la
> proyección HTTP (test de paridad, §9).

## 3. Protocolo

Mismo endpoint y autenticación que hoy (`/v1/jugador/partidas/:gameId/tiempo-real`, `?sesion=<id>` o cabecera). Para
el admin, endpoint hermano **`/v1/admin/partidas/:gameId/tiempo-real`** con el mismo protocolo, autorizado en
`preValidation` con `exigirAdministracion` (hoy el cliente admin no tiene WS de datos). Los canales `admin/*` solo se
aceptan en ese endpoint y los de jugador solo en el suyo.

### 3.1 Cliente → servidor

```json
{ "accion": "suscribir",       "canal": "vista/mapa" }
{ "accion": "suscribir",       "canal": "vista/mapa", "desde": 1042 }
{ "accion": "resincronizar",   "canal": "vista/mapa" }
{ "accion": "desuscribir",     "canal": "vista/mapa" }
```

- `suscribir` sobre un canal de vista responde `suscrito` y **a continuación** una `foto`. `desde` es opcional
  (optimización de la Fase 4): si el servidor aún conserva el encadenado desde esa versión lo reanuda sin foto; si no,
  manda foto. Un cliente v1 que no lo envía recibe siempre foto.
- `resincronizar` pide la foto de nuevo (el cliente detectó un hueco, o un fallo al aplicar). Rate limit: una foto por
  canal cada 2 s por socket; el exceso se responde `error` con `codigo: "demasiado_frecuente"`.

### 3.2 Servidor → cliente

```json
{ "tipo": "suscrito", "canal": "vista/mapa", "intervaloTickMs": 4000 }
{ "tipo": "foto",   "gameId": "g1", "canal": "vista/mapa", "version": 1042, "instante": 1789410720000, "datos": { … } }
{ "tipo": "delta",  "gameId": "g1", "canal": "vista/mapa", "desde": 1042, "hasta": 1043, "instante": 1789410724000, "cambios": [ … ], "escalares": { … } }
{ "tipo": "resync", "canal": "vista/mapa", "motivo": "cliente_lento" }
{ "tipo": "revocado", "canal": "vista/asentamiento/a7", "motivo": "ya_no_es_de_tu_faccion" }
{ "tipo": "error", "canal": "vista/xx", "error": "canal desconocido", "codigo": "canal_desconocido" }
```

- `suscrito` gana el campo opcional `intervaloTickMs` (real, ms entre ticks del reloj de mundo; `null` si la partida no
  tiene reloj y solo avanza con `POST .../tick`: entonces el cliente **no extrapola**). Aditivo: los clientes actuales
  lo ignoran.
- `foto.datos` es un objeto cuyas claves son las del canal (§2), con los **mismos nombres y tipos** que la proyección
  HTTP. `version` es `GameSessionState.version` (sube en cada comando aceptado y en cada tick, `session/gameSession.ts`,
  `session/comandos/tipos.ts`); `instante` es el tiempo de mundo de la foto.
- `delta.desde` es la `version` del mensaje anterior de ESE canal (foto o delta); `hasta` la de este. Un delta puede
  cubrir varias versiones (varios comandos coalescidos, §6.1). Un delta vacío (`cambios: []`) en cada tick actúa de
  **latido**: confirma que el cliente está al día y deja detectar un canal muerto sin mirar `tick`.
- `resync` avisa de que la cadena se corta por el servidor (socket lento, reevaluación de visibilidad, reinicio del
  runner); **va seguido de una `foto`** sin que el cliente tenga que pedirla. `motivo`: `cliente_lento` | `visibilidad`
  | `servidor_reiniciado` | `partida_reemplazada`.
- `revocado`: se perdió el derecho a ver ese canal; el servidor ya lo ha desuscrito. `motivo` es informativo.
- `error.codigo` (aditivo; `error` conserva su texto): `canal_desconocido` (el servidor no implementa ese canal: el
  cliente cae al sondeo — es el mecanismo de descubrimiento, no hay otra negociación), `no_autorizado`,
  `demasiado_frecuente`, `formato`. El `error: "no autorizado"` actual se mantiene.

### 3.3 Reglas del cliente

1. Guarda por canal `version` y el almacén de entidades. Al recibir `foto`: **sustituye** el almacén del canal.
2. Al recibir `delta`: si `desde === version` → aplica y `version = hasta`; si `hasta <= version` → lo ignora
   (duplicado); en cualquier otro caso (hueco) → manda `resincronizar` y descarta deltas hasta recibir la foto.
3. Reconexión del socket: las suscripciones no sobreviven (02 §4); el cliente vuelve a suscribirse a todo y recibe
   fotos nuevas. No hace falta `GET .../eventos` para el estado de los canales de vista, solo para los eventos
   (crónica, avisos) que quiera recuperar.
4. Un delta de `vista/mapa` y uno de `vista/heroe` con el mismo `hasta` describen el mismo instante pero llegan por
   separado: el cliente repinta por `requestAnimationFrame`, no por mensaje.
5. Mientras el canal esté sin foto (esperándola), la UI conserva lo último dibujado y no muestra datos nuevos a medias.
6. Si un canal responde `canal_desconocido`, ese canal vuelve al sondeo HTTP (§8.3) sin afectar a los demás.

## 4. Formato del delta

```json
{
  "tipo": "delta", "gameId": "g1", "canal": "vista/mapa",
  "desde": 1042, "hasta": 1043, "instante": 1789410724000,
  "cambios": [
    { "t": "ejercitosAvistados", "id": "e9",  "op": "upsert", "v": { "id": "e9", "tipo": "ejercito", "faccionId": "f2", "posicionActual": {"x": 410, "y": 233}, "participantes": 3, "heroeIds": ["h4","h5","h6"], "desplazamientoPorTick": {"x": 6.1, "y": -2.4} } },
    { "t": "ejercitosAvistados", "id": "e3",  "op": "patch",  "v": { "posicionActual": {"x": 388.5, "y": 240.2}, "desplazamientoPorTick": {"x": 5.9, "y": -2.2} } },
    { "t": "caravanasAvistadas", "id": "c2",  "op": "remove" },
    { "t": "exploracion",        "op": "set",    "v": { … } }
  ],
  "escalares": { "estadoMapa": "…" }
}
```

Operaciones:

| `op` | Significado | Aplica a |
|---|---|---|
| `upsert` | La entidad `t`/`id` pasa a ser exactamente `v` (inserta o sustituye entera) | colecciones con clave |
| `patch` | Fusión **superficial**: cada clave de `v` sustituye a la de la entidad (un `null` explícito borra la clave); las que no aparecen no cambian. Si la entidad no existe es un hueco: el cliente resincroniza | colecciones con clave |
| `remove` | La entidad desaparece (por destruida, por salir de vista o por perder el derecho a verla: el cliente no distingue ni debe) | colecciones con clave |
| `set` | Bloque sin clave (objeto o array sin `id`) sustituido entero | claves de primer nivel que no son colección |

`escalares` se fusiona sobre la raíz del canal (`instante`, `estadoMapa`, `miConvocatoriaId`…); `instante` y `version`
van además en la cabecera del mensaje. Orden dentro de `cambios`: se aplican en el orden dado; el servidor emite
primero los `remove`, luego `upsert`/`patch`, de modo que un id reutilizado no se borre tras reinsertarse.

**Qué colecciones tienen clave y cuál es** (todas llevan `id` salvo que se indique): `ejercitos`, `ejercitosAvistados`,
`caravanas`, `caravanasAvistadas`, `asentamientos` (admin), `asentamientosAvistados`, `asentamientosConocidos`
(`asentamientoId`), `aedasAvistados`, `aedasResidentes`, `batallas` (`battleId`), `campamentosBandidos`,
`campamentosMercenarios`, `alijos`, `heroesVisibles` (`heroeId`), `heroes` (admin), `facciones`, `zonas`,
`zonasFusionadas`, `caminos`, `convocatorias`, `informesDeInspeccion` (clave compuesta `objetivo.tipo+objetivo.id`,
servida como `id` sintético `"<tipo>:<id>"`), `miradasIntel`, `informesPlaza` (`asentamientoId`), `propuestas*`,
`acuerdos`, `ordenes`, `relaciones`, `titulos`, `edificios`, y `asentamiento` (en `vista/asentamiento/<id>` y
`admin/asentamiento/<id>`: una sola entidad cuyo `id` es el de la plaza). El diccionario `nombresDe*` y `territorioPorEjercito`
son mapas `id → valor`: viajan como colección (cada clave es un `id`, `v` el valor). Cualquier otro campo de primer
nivel es un bloque `set`. El servidor implementa **un diff genérico** sobre esta tabla (§6.1).

### 4.1 Lo móvil: ruta una vez, progreso después

Dos audiencias, dos formas. **Implementado así:** `avancePorTick` y `desplazamientoPorTick` viajan **solo por WS** (fotos y deltas), no en el GET HTTP; el servidor los calcula por
diferencia con la foto anterior, así que **no aparecen hasta el segundo tick** de una entidad (el cliente no extrapola hasta tenerlos). `instante` y
`version` van solo en la cabecera del mensaje, no en `datos` ni en `escalares`. Los ids sintéticos (`informesDeInspeccion`, `historial`) los fabrica el
servidor. Ausentes cuando no aplican.

**Propios** (`ejercitos`, `caravanas` en `vista/heroe`; en `admin/partida` todos): ya traen `ruta: Point[]`,
`progreso` (0‥1 sobre la **longitud** de la polilínea, `engine/movimiento.ts`), `posicionActual`, `estado`. Regla:

- **Al salir** (o al cambiar de rumbo, perseguir, regresar, recalcular ruta) el delta incluye `ruta` en un `upsert` o
  `patch`. **Mientras la ruta no cambia, no se vuelve a enviar** (el servidor compara la ruta con la del flush
  anterior; es la ruta de mayor peso del mensaje).
- **Cada tick** de algo en movimiento: `patch` con solo `progreso`, `posicionActual` y, nuevo, **`avancePorTick`**
  (fracción de la ruta que recorrió en el último tick; varía con el coste del terreno y los caminos). Un ejército
  `estacionado` lleva `avancePorTick: 0`.
- Opcional, informativo: `llegadaEstimada` (`Instante`), extrapolación lineal `ahora + (1 − progreso)/avancePorTick ·
  tick`. El cliente no interpola con ella; sirve para etiquetas («llega en 3 min»). Backend puede omitirla en la
  primera fase.

**Ajenos avistados** (`ejercitosAvistados`, `caravanasAvistadas`): no llevan `ruta` ni destino (sería filtrar intención,
02 §4.1); solo `posicionActual`, y se añade **`desplazamientoPorTick: Point`** (vector del último tick, en unidades de
mapa; `{0,0}` si está quieto). Es información que el cliente ya podía deducir de dos posiciones consecutivas, así que no
amplía lo que se ve. `teSigue` y el resto no cambian.

**Interpolación (cliente).** Con `intervaloTickMs = T`, `t0` el instante local de recepción del último delta:

```text
propio:  progresoVisto(t) = min(1, progreso + avancePorTick · min(1, (t − t0)/T))      → posición = puntoEnPolilínea(ruta, progresoVisto)
ajeno:   posiciónVista(t) = posicionActual + desplazamientoPorTick · min(1, (t − t0)/T)
```

Al llegar el siguiente delta se corrige hacia el valor real con un amortiguado corto (≈150 ms) para no dar saltos. Con
`T = null` no se extrapola. La extrapolación se capa a un tick: si el delta se retrasa, la entidad se detiene en vez de
adelantarse sin límite. El cliente no debe pedir nada al servidor entre deltas.

## 5. Ejemplos de contenido

Los mensajes completos y válidos están en [`fixtures/tiempo-real/`](fixtures/tiempo-real/):

| Fixture | Qué enseña |
|---|---|
| `suscribir.vista-mapa.json`, `suscrito.json` | alta de suscripción y su respuesta con `intervaloTickMs` |
| `foto.vista-mapa.json` | foto de mapa: lo móvil propio con `ruta`, ajenos con `desplazamientoPorTick`, asentamientos avistados |
| `delta.vista-mapa.json` | un tick: `patch` de progreso, `upsert` de un avistado nuevo, `remove` de uno que sale de vista, `escalares` |
| `delta.vista-heroe.salida-con-ruta.json` | el ejército propio sale: `patch` que trae la `ruta` por única vez |
| `delta.vista-heroe.progreso.json` | los ticks siguientes: solo `progreso`/`avancePorTick`/`posicionActual` |
| `foto.vista-asentamiento.json`, `delta.vista-asentamiento.json` | interior de plaza: una obra que termina (`en_construccion` → `activo`), almacén que cambia |
| `foto.admin-partida.json`, `delta.admin-partida.json` | partida sin niebla en versión ligera |
| `delta.latido.json` | delta vacío de tick |
| `resync.json`, `revocado.json`, `error.canal_desconocido.json` | los mensajes de control |

## 6. Coste en servidor

### 6.1 Cuándo y cómo se calcula

Hoy el hub serializa un evento una vez por canal (`HubDeDifusion.difundir`). El nuevo mecanismo añade un
**`DifusorDeVistas`** junto al hub (infraestructura de transporte en `server/difusion/`; la decisión de «qué ve quién»
sigue en `session/`, como `canales.ts`):

1. **Disparo.** Tras cada transición aceptada (comando, tick, admin `POST .../tick`) el runner avisa de la nueva
   `version`. El difusor marca «sucias» las claves con suscriptores y programa un **flush coalescido**: como mucho uno
   por canal-clave cada `COALESCER_MS` (propuesta 100 ms) y siempre uno por tick. Un comando seguido de un tick en esos
   100 ms genera un solo delta con `desde`→`hasta` multiversión.
2. **Proyección única por clave.** Por cada clave sucia se construye UNA vez la foto filtrada del segmento
   (`proyectarSegmento(estado, clave)`: la proyección actual, recortada a los campos de ese canal; reutiliza las
   funciones puras de `session/proyecciones/jugador.ts` y no duplica reglas de visibilidad).
3. **Diff genérico** contra la **foto cacheada** de esa clave (la del último flush): por colección con clave →
   `upsert`/`patch`/`remove`; por bloque → `set` si cambió (comparación estructural; las rutas por identidad primero).
   El `patch` de lo móvil se construye comparando solo los escalares móviles. Resultado: el objeto `cambios`.
4. **Serialización única por clave:** `JSON.stringify` del mensaje UNA vez y el mismo string a todos los suscriptores de
   la clave (como `difundir`). La foto cacheada pasa a ser la nueva; una suscripción nueva recibe esa misma foto
   (el `JSON.stringify` de la foto se cachea hasta el próximo flush) y la cadena continúa sin hueco.
5. **Sin suscriptores o sin cambios no se hace nada** (ni proyectar). Con cambios vacíos solo se emite el latido de tick.

### 6.2 Mapa con niebla por jugador: agrupar por clave de visibilidad o diff por suscriptor

Dos opciones:

- **A. Agrupar por clave** — un diff y una serialización por clave de visibilidad, el string compartido.
- **B. Diff por suscriptor sobre una foto cacheada** — una proyección filtrada por jugador y un diff por jugador; la foto
  cacheada ahorra rehacer la anterior, no la nueva.

**Recomendación de diseño: A, con dos capas. Implementado así: A degenerada (clave = héroe)**, ver «Resultado de la implementación» al final de este apartado. Los argumentos siguen valiendo para cuando se pueda agrupar.

Argumentos, comprobados contra el código de la proyección (`proyectarParaJugador`):

1. La niebla es de **Facción**, no de héroe: `asentamientosPropios`/`esPropio` salen de la Facción; la memoria se
   guarda `memoriaPorFaccion`; lo que ve un aliado se suma «EN VIVO» (`compartenVision`). Todos los ciudadanos de una
   Facción ven el mismo mapa. Lo que sí es de héroe (`ejercitos` propios con ruta, `teSigue`, `heroe`, informes) es
   pequeño y va en `vista/heroe`. Dividir en `vista/mapa` (clave de Facción) y `vista/heroe` (clave de héroe) hace que
   lo caro (el mapa) se comparta y lo personal (barato, derivado de unas pocas entidades propias) no.
2. El coste dominante no es el diff, es **construir la proyección**. A es O(claves) en construcciones y
   serializaciones; B es O(suscriptores). Con 5 jugadores de play-test en 1-2 Facciones da igual; con una partida de
   500 jugadores en pocas decenas de Facciones, A divide el coste entre el tamaño medio de la Facción.
3. A reutiliza el `JSON.stringify` (mismo patrón que `difundir`); B serializa N veces.
4. B solo gana cuando casi nadie comparte clave; en ese caso A degenera exactamente en B (clave = héroe, p. ej. los
   sin Facción) sin coste extra. No hay escenario donde B sea mejor y A peor, salvo la complejidad de calcular la clave.
5. Riesgo de A: que la **clave quede corta** (dos jugadores con clave igual pero visión distinta ⇒ fuga). Se mitiga
   definiendo la clave como `faccionId` + firma ordenada de las Facciones con las que `compartenVision` es cierto, y
   **comprobando en test** que la proyección completa de dos miembros de la misma clave coincide en los campos de
   `vista/mapa` (test de invariancia, §9). Si una regla futura hiciera la visión dependiente del héroe (p. ej. visión
   personal de exploración, ver `Heroe.exploracionPersonal`), esa parte pasa a `vista/heroe`; la regla de oro: **todo
   campo de `vista/mapa` debe ser función de la clave**.
6. La reevaluación de la clave (cambio de Facción, relación de visión rota o creada) mueve al suscriptor de grupo con
   `resync` + foto del grupo nuevo (§7.2): el coste de la transición es una foto, que es lo que costaría B igualmente.

**Resultado de la implementación (Backend, 2026-10-10).** `vista/mapa` agrupa por **héroe**, no por Facción: la proyección del
mapa mezcla entradas de héroe (`exploracionPersonal`, `plazasRecordadas`, la plaza pisada excluida de avistados, el almacén de una
plaza si su columna está a la puerta), y compartirlas por Facción las filtraría (rompe la invariante de que todo campo de `vista/mapa`
sea función de la clave). Una proyección por héroe cuesta ≈0,5 ms. Para poder agrupar por Facción habría que sacar esas entradas del
mapa hacia `vista/heroe`, y eso cambia **qué ve cada jugador**: es decisión de reglas, **del usuario** (pregunta abierta nº 6, §11). Mientras
tanto la compartición por clave queda en `vista/asentamiento/<id>` y `admin/*`.

Sobre `vista/asentamiento/<id>` y `admin/*` no hay duda: un diff por canal, compartido por todos los suscriptores.

### 6.3 Backpressure

El hub mira `socket.bufferedAmount` antes de enviar un delta o foto:

- `bufferedAmount > UMBRAL_LENTO` (propuesta 256 KiB) ⇒ **no se envía y no se encola**: el socket se marca
  `desactualizado` para ese canal (los deltas que siguen se saltan para él, porque ya no encadenan). Es coalescer por
  omisión: lo perdido se recupera con una foto, no con una cola.
- En el primer flush con `bufferedAmount < UMBRAL_RECUPERADO` (propuesta 32 KiB) se le manda `resync` + foto (con el
  rate limit de §3.1) y vuelve a encadenar.
- `desactualizado` durante más de `LENTO_MAX_S` (propuesta 15 s) ⇒ `socket.close(4408, 'cliente lento')`; el cliente
  reconecta y resuscribe (vuelve a empezar con foto).
- Una foto nunca se manda a un socket por encima del umbral; no hay colas por socket. La memoria por conexión es
  constante.
- Tope de suscripciones por socket (propuesta 64) y de fotos simultáneas en construcción; protege del cliente que
  suscribe a `vista/asentamiento/<id>` de cientos de plazas.

### 6.4 Métricas a medir (en `/admin/metricas` y en logs de diagnóstico)

Por canal-tipo (`vista/mapa`, `vista/heroe`, `vista/asentamiento`, `admin/partida`, …) y por partida:

| Métrica | Para qué |
|---|---|
| `suscriptores` (gauge) y `claves` (gauge) | razón de compartición = suscriptores/claves; decide si A compensa |
| `bytes_por_flush` (p50/p95/máx) y `bytes_por_tick` | tamaño del delta; comparar con `bytes_foto` |
| `bytes_foto` y `fotos_enviadas` (counter, por motivo: alta/resync/lento/visibilidad) | cuántas veces falla el encadenado |
| `ms_proyeccion`, `ms_diff`, `ms_serializacion` (histograma por flush) | dónde se va la CPU |
| `cambios_por_flush` (entidades por `op`) | densidad real de cambio |
| `coalescidos` (versiones por delta) | efecto de `COALESCER_MS` |
| `sockets_lentos`, `cierres_4408`, `bufferedAmount` máx | backpressure |
| `revocados` (por motivo) | seguridad / reevaluación |
| Del cliente (opcional, `POST` de diagnóstico o log local): `huecos_detectados`, `ms_aplicar_delta`, FPS | validar el lado cliente |

Línea base a capturar ANTES de la Fase 1: bytes/min y ms de CPU de los dos GET de sondeo con 5 clientes (lo que hay que
batir). Criterio de aceptación sugerido: ≤ 10 % de los bytes del sondeo con la partida de play-test, y `ms_proyeccion` por
flush ≤ el de una proyección HTTP × número de claves.

## 7. Seguridad

### 7.1 Invariantes

1. **El delta es siempre el diff de dos fotos ya filtradas para esa clave.** Nunca se filtra un diff del estado en
   bruto: así lo que sale de la niebla desaparece por `remove` (solo `id`) y nunca arrastra campos que el cliente no
   podría ver. Cada `upsert` lleva la entidad con la misma forma recortada que la proyección HTTP.
2. Nada de `vista/*` lleva lo que la proyección no lleva (ni `tokensParticipante`, ni escuadras ajenas, ni interior de
   plazas ajenas, ni inventarios; 02 §3.4, §4.1). El canal `admin/*` aplica `vistaAdminDeEstado` (que ya vacía los
   tokens de batalla).
3. Un `remove` no distingue «destruido» de «dejó de verse»: no confirma que siga existiendo.
4. Lo de intención (`ruta`, destino, `persiguiendo`) solo viaja a los dueños (`vista/heroe`) y al admin. A los demás,
   `desplazamientoPorTick`, que no se puede ocultar.
5. `exploracion` y la memoria (`asentamientosConocidos`) son de Facción y viajan por `vista/mapa`: solo lo ya explorado.
6. Los canales `admin/*` no se aceptan en el endpoint de jugador, ni al revés; un jugador no puede suscribirse a un
   canal `admin/*` aunque conozca el nombre (`canal_desconocido`, no `no_autorizado`, para no revelar su existencia).

### 7.2 Autorización: al suscribir y reevaluada

- **Al suscribirse:** `puedeSuscribirseA` (`session/canales.ts`) se amplía con los canales de vista: `vista/mapa` y
  `vista/heroe` exigen héroe con membresía (como `mapa/general` y `heroe/<id>`); `vista/asentamiento/<id>` exige
  `esCiudadano` de la Facción dueña **y estar dentro de la plaza** (implementado así: la ciudadanía habilita, la presencia ejerce; sin
  lo segundo, quien anda por el camino recibiría el interior de todas sus plazas); `admin/*`, `exigirAdministracion`.
- **Reevaluación, en cada flush** (barata: sets en memoria), por tipo:
  - `vista/asentamiento/<id>`: sigue existiendo la plaza, el héroe sigue siendo ciudadano de su Facción **y sigue dentro de ella** (salir de la plaza ⇒ `revocado` en el siguiente flush). Si no ⇒
    `revocado` y desuscribir. Casos: expulsión/salida de la Facción, anexión, fusión, destrucción o traslado de plaza,
    cambio de Facción por `fundar`/`responderSolicitud`.
  - `vista/mapa`: se recalcula la **clave** del héroe (Facción + firma de visión compartida). Si cambió ⇒ `resync` con
    `motivo: "visibilidad"` y foto del grupo nuevo (nunca un delta entre claves distintas). Casos: cambiar de Facción,
    alianza/vasallaje creados o rotos, quedarse huérfano.
  - `vista/heroe`: solo su `heroeId`; si la membresía se revoca, `revocado` y cierre del socket.
  - `admin/*`: sigue habiendo permiso de administración (la lista puede cambiar en caliente); si no ⇒ `revocado`.
- La reevaluación es **también un disparo de flush**: la pérdida de derecho se aplica en el mismo tick en que el estado
  cambia, no en el siguiente sondeo.
- El `batalla/<id>` actual sigue evaluándose solo al suscribirse (su alcance es público y mínimo, 02 §3.5); si
  más adelante se le añade estado, adoptará esta misma reevaluación.

## 8. Convivencia y migración

### 8.1 Qué se mantiene, qué se depreca

| Pieza actual | Destino |
|---|---|
| Canales de eventos `mapa/general`, `asentamiento/<id>`, `heroe/<id>`, `batalla/<id>` y mensaje `evento` | **Se mantienen sin cambios.** Los eventos son hechos (crónica, avisos, animaciones); un delta no los sustituye. Un cliente puede tener ambos |
| `GET /jugador/partidas/:gameId` (proyección) | **Se mantiene** como foto inicial/respaldo y para quien no use WS (Unity v1) |
| `GET /admin/partidas/:gameId` | **Se mantiene** (foto/respaldo, exportación) |
| `GET .../eventos?desde=` y `.../cronica` | **Se mantienen**: son el cursor de eventos, ortogonal al estado |
| Mensaje `tick` (sin datos) | **Deprecado** desde la Fase 4 (se sigue emitiendo hasta que ambos clientes lo dejen de usar y pase una versión menor del contrato); el latido de los canales de vista lo sustituye |
| Sondeo de 3-5 s | **Se retira** de los clientes por canal al migrar; queda un sondeo lento de seguridad (≥ 60 s) mientras el WS esté caído (§8.3) |
| Campos nuevos (`avancePorTick`, `desplazamientoPorTick`, `llegadaEstimada`, `intervaloTickMs`, `codigo` de error) | Aditivos también en HTTP; ignorables por los clientes actuales |

Cambios de contrato solo aditivos ⇒ **sigue siendo v1**. No hay `schemaVersion` nuevo de proyección; los mensajes de
canales de vista llevan su forma en `contratos.schema.json` (`MensajeFoto`, `MensajeDelta`, …) y entran con un cambio de
versión menor documentado en `02` y en el `GET /v1/openapi.json` (§10).

### 8.2 Fases

| Fase | Servidor | Clientes | Efecto / criterio de salida |
|---|---|---|---|
| **0** | Este contrato + fixtures + schemas; sin código | Revisan el contrato, responden las preguntas de §11 | Contrato aceptado; baseline de métricas (§6.4) |
| **1** | Infra del difusor (foto/delta/resync/backpressure/métricas), endpoint `admin/.../tiempo-real` y `admin/partida` + `admin/asentamiento/<id>` | **Admin:** abre el WS, aplica foto+deltas, deja el sondeo como respaldo | Admin sin sondeo de 3-5 s; sale si bytes del admin ≤ 10 % y sin huecos en 1 h de partida |
| **2** | `vista/mapa` + `vista/heroe`; campos `avancePorTick`/`desplazamientoPorTick`; claves de visibilidad; test de invariancia y de paridad con la proyección | **Jugador:** el mapa, columnas y caravanas dejan de sondear; interpolación | Mapa fluido sin GET de proyección entre acciones; sin fuga en el test de visibilidad |
| **3** | `vista/asentamiento/<id>` | **Jugador:** la vista de plaza deja el sondeo; sigue usando la respuesta de cada comando | Interior de plaza al día sin GET |
| **4** | `desde` para reanudar sin foto (opcional); `tick` deprecado | Ambos: quitan el consumo de `tick`; sondeo solo de respaldo ≥ 60 s | Retirada de `tick` en la siguiente versión menor |

Cada fase se despliega sola. Los clientes activan cada canal por detección (`canal_desconocido` ⇒ siguen sondeando),
así que no hay orden obligado entre despliegue de servidor y de clientes dentro de una fase.

### 8.3 Respaldo

Cuando el socket está caído o un canal no se pudo suscribir, el cliente sondea la proyección/estado a ritmo lento (≥ 60 s,
no 3-5 s) y reintenta el WS con retroceso exponencial. Al volver el WS, las fotos nuevas reemplazan lo sondeado. Las
respuestas de `POST .../comandos` siguen devolviendo la proyección propia y el cliente puede aplicarla como foto de
`vista/heroe` si su `version` es mayor que la del canal (opcional).

## 9. Fixtures, schemas y pruebas

- Fixtures en `Docs/Coordinacion/fixtures/tiempo-real/` (datos de ejemplo coherentes entre sí: mismo `gameId`, ids y
  versiones encadenadas `1042 → 1043 → …`). Backend los promueve a `src/contratos/v1/fixtures/` junto con los schemas
  (definiciones `MensajeSuscribir`, `MensajeFoto`, `MensajeDelta`, `MensajeResync`, `MensajeRevocado`, …) y la regla de
  `src/contratos/v1/__tests__/contratos.test.ts` (cada fixture cumple su definición y es idéntico a su fuente).
- Pruebas que Backend debe añadir (nombres orientativos): `difusorDeVistas.test` (encadenado, coalescer, hueco, latido),
  `visibilidadDeVistas.test` (**invariancia**: dos héroes de la misma clave obtienen el mismo `vista/mapa`; un héroe de
  otra Facción nunca recibe una entidad fuera de su niebla en foto ni delta; `remove` solo con `id`),
  `paridadDeVistas.test` (aplicar foto + N deltas sobre el almacén ≡ la proyección HTTP en los campos del canal, tras N
  ticks de una partida simulada), `reevaluacionDeVistas.test` (expulsar de Facción ⇒ `revocado`; alianza rota ⇒
  `resync`), `backpressure.test` (socket lento no acumula; foto al recuperar; 4408).
- El cliente tiene su propia prueba de paridad: aplicar los fixtures encadenados produce el almacén esperado; un hueco
  provoca `resincronizar`.

## 10. Reparto

**Backend** (sesión «Backend»)
- `server/difusion/`: `DifusorDeVistas` (flush coalescido, foto cacheada por clave, diff genérico según la tabla de §4,
  serialización única), backpressure (§6.3), métricas (§6.4) en `/admin/metricas`.
- `session/`: `proyectarSegmento` (recorta `proyectarParaJugador` por canal y calcula la clave de visibilidad), ampliar
  `puedeSuscribirseA` y la reevaluación (§7.2), canales `admin/*`.
- `server/rutas/tiempoReal.ts`: nuevas acciones `resincronizar`/`desde`, mensajes `foto`/`delta`/`resync`/`revocado`,
  `intervaloTickMs` en `suscrito`, `codigo` en `error`, y la ruta hermana `/admin/partidas/:gameId/tiempo-real` con
  `exigirAdministracion`.
- Campos aditivos de lo móvil (`avancePorTick`, `desplazamientoPorTick`, y opcional `llegadaEstimada`) en la proyección
  y en el estado admin.
- Contrato: schemas y fixtures en `src/contratos/v1/`, pruebas de §9, OpenAPI; sustituir el comentario de cabecera de
  `tiempoReal.ts` por un puntero a este documento (hoy duplica el protocolo en prosa).
- Medir la baseline antes de empezar y el efecto de cada fase.

**Cliente jugador** (sesión «Cliente jugador»)
- Cliente WS de segmentos: suscribir `vista/mapa` y `vista/heroe` (Fase 2) y `vista/asentamiento/<id>` de la plaza en
  pantalla (Fase 3, desuscribiendo al salir); almacén por canal, regla de versiones de §3.3, `resincronizar`.
- Motor de interpolación de §4.1 para columnas, ejércitos y caravanas propias y ajenas; repintar por
  `requestAnimationFrame`, no por mensaje.
- Quitar el sondeo de 3-5 s por canal migrado; sondeo de respaldo ≥ 60 s y detección de `canal_desconocido`.
- Mantener los canales de eventos para avisos y crónica; tratar `revocado`/`resync` en la UI sin parpadeo.

**Cliente admin** (sesión «Cliente admin»)
- Abrir el nuevo WS de admin (`/v1/admin/partidas/:gameId/tiempo-real`, mismas credenciales que sus GET) y suscribir
  `admin/partida` (Fase 1); `admin/asentamiento/<id>`, `admin/campamento/<id>` o `admin/heroe/<id>` al abrir el detalle.
- Aplicar foto + deltas al almacén que hoy alimenta el estado completo; actualizar solo las filas/capas que cambian
  (no repintar todo); interpolar lo móvil igual que el jugador (la `ruta` está siempre).
- Campos de la versión ligera: acordados (§2.1).
- Sondeo de respaldo ≥ 60 s.

**Coordinación** (esta sesión): mantener 03 y 02 al día con lo que decidan Backend y los clientes, y mantener el índice
`README.md`.

## 11. Preguntas abiertas

Técnicas (las contesta quien implementa; no son reglas de juego):
1. **Tamaño de `exploracion`** (la niebla explorada): si resulta grande, Backend propone en 03 su codificación por
   deltas de celdas (aditivo) en vez del bloque `set`. Hasta medirla, se envía como bloque cuando cambia.
2. ~~Cliente admin: campos de la versión ligera~~ **Resuelta** (§2.1). Pendiente para Backend: confirmar que cada campo de `resumen` sale de funciones del motor existentes (si alguno no, avisar a Cliente admin).
3. **Rate de flush:** ¿`COALESCER_MS = 100` y latido solo en tick es suficiente para el cliente jugador (24-60 fps con
   interpolación) o se necesita más frecuencia en combate/persecución?
4. **Reanudar con `desde`** (Fase 4): ¿merece el buffer en memoria del servidor (últimos ~64 deltas por clave) o basta
   con foto en cada reconexión? Decidir con las métricas de `bytes_foto`.
5. **Duración y frecuencia de la clave de visibilidad** si en el futuro la visión depende del héroe (exploración
   personal): hoy es de Facción; si cambia, esa parte se mueve a `vista/heroe`.

De reglas de juego (del usuario; **no se deciden aquí**):
6. **Agrupar `vista/mapa` por Facción.** Hoy se calcula por héroe porque la proyección del mapa incluye información personal
   (`exploracionPersonal`, `plazasRecordadas`, plaza pisada, almacén con la columna a la puerta). Agrupar por Facción exigiría que esa información
   pasara a ser de Facción o saliera del mapa personal. Es una decisión sobre qué ve cada jugador; no hace falta para el play-test
   (el coste por héroe es ≈0,5 ms) y solo importa si el número de jugadores crece mucho.

Otras (no son reglas): Este contrato no cambia qué se ve ni quién,
solo cómo se entrega. Si al implementarlo apareciera que la regla actual de visibilidad es ambigua en algún caso (p. ej.
qué ve un héroe sin Facción dentro de una plaza ajena), se anota aquí como pregunta y se espera respuesta del usuario.

## 12. Estado de la implementación y mediciones (2026-10-10)

Implementado por Backend (`2ab3b11..fe78215`): endpoint `/v1/admin/partidas/:gameId/tiempo-real`; canales `admin/{partida,asentamiento,campamento,heroe}`,
`vista/mapa`, `vista/heroe`, `vista/asentamiento/<id>`; foto, delta, resync, revocado; coalescer de 100 ms; backpressure (256/32 KiB, cierre 4408);
rate limit de `resincronizar`; `error.codigo`; `intervaloTickMs` en `suscrito`; métricas en `/admin/metricas` (campo `vistas`); schemas `MensajeFoto`,
`MensajeDelta`, `CambioDeVista`… en `contratos.schema.json`. Un test valida contra el schema todo lo que emite el difusor.

Desviaciones del diseño inicial: (1) `vista/mapa` por héroe (§6.2); (2) `vista/asentamiento` exige presencia (§7.2); (3) `avancePorTick`/`desplazamientoPorTick` solo por WS y
desde el segundo tick (§4.1); (4) `instante`/`version` solo en cabecera.

Mediciones con el mundo vivo (12 plazas, 62 héroes): foto de admin 275 KiB y delta 6,7 KiB por tick, contra 5586 KiB del GET cada 3 s; jugador, delta de 3,4 KiB por tick
(`vista/mapa` + `vista/heroe`) contra 254 KiB de proyección cada 5 s. Pendiente: Fase 4 (`desde`) y medir el mundo de 500 bots.
