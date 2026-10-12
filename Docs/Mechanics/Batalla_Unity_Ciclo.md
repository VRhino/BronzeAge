# Batalla de Unity: el ciclo desde BronzeAge

## 1. Alcance y fuentes

Cubre **todo lo que BronzeAge hace con una batalla que se juega en Unity**: cuándo se abre, qué se congela en el ticket, qué queda bloqueado mientras dura, quién puede unirse, los cinco mensajes que manda el servidor de batalla, cómo se aplica el resultado y qué se cuenta a los jugadores. **Unity decide todo lo de dentro** (banderas, reaparición, cómo se gana); BronzeAge decide que la batalla ocurra, dónde, quién entra, quién sale y qué le queda a cada uno (Doc 5.15.1b).

- **Verificado contra:** `1850a73` + los cuatro arreglos de §14 (2026-10-12).
- **Fuentes:** `session/batallas.ts` (el ciclo y el candado), `session/entradaEnBatalla.ts` (quién entra y a qué bando), `session/salidasDeBatalla.ts`, `session/resultadoBatalla.ts`, `session/comandos/batalla.ts` (los comandos y los mensajes de Conquest), `session/comandos/interaccion.ts` (`atacar`, que abre), `session/comandos/avanzarTick.ts` (vencer y bloquear), `server/rutas/batallas.ts` (las rutas), `server/identidad/servidoresDeBatalla.ts` (la credencial), `contratos/v1/dto.ts` y `contratos.schema.json` (la forma exacta de lo que viaja), `session/canales.ts` y `session/proyecciones/jugador.ts` (lo que se cuenta).
- **Canon:** [`Docs/Game/5_Sistema_Militar_y_Combate.md`](../Game/5_Sistema_Militar_y_Combate.md) 5.10, 5.15 y 5.16.4-5.16.6. **Contrato con Conquest:** [`Docs/Coordinacion/01_Modelo_de_datos_compartido.md`](../Coordinacion/01_Modelo_de_datos_compartido.md) §15-§17 y [`02_Contrato_Comunicacion_Servidor_Cliente.md`](../Coordinacion/02_Contrato_Comunicacion_Servidor_Cliente.md) §3.
- **No está aquí** (está en su documento):
  - El combate **con números** (sin Unity: bot contra bot, o sin servidores de batalla) → [`Combate.md`](Combate.md). Cada resolución numérica tiene su apertura equivalente aquí; sus consecuencias comunes (`aplicarConquista`, `desalojarResidentes`, `registrarDerrota`) se describen allí.
  - Las reglas que llevan el héroe a una columna, el Liderazgo y los loadouts → [`Heroe_Y_Tropa.md`](Heroe_Y_Tropa.md); el movimiento y la persecución → [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md).
  - Quién puede ejecutar cada comando (la matriz) y cómo se reparten los canales de eventos y de vista → [`Proyecciones_Y_Autorizacion.md`](Proyecciones_Y_Autorizacion.md).
  - Lo que pasa **dentro** de la partida: es de Conquest.

## 2. Cuándo se abre

No hay un comando para abrir una batalla: la abre `atacar` (contra una columna, una caravana, un campamento de bandidos o una plaza, que es asediarla). Con **`ctx.batallasEnUnity`** (el proceso declaró `SERVIDORES_BATALLA`, §8) y **al menos un héroe `humano` entre los participantes de los dos bandos** (`hayHumano`), `atacar` no resuelve el combate: abre una `Batalla` y devuelve `{ battleId }`. En cualquier otro caso (sin servidores, o bot contra bot / guarnición contra bot / bandidos contra una caravana sin humanos) se resuelve con números ([`Combate.md`](Combate.md) §2).

- Se valida **igual que el combate con números** (`aperturaDeAtaque`: `exigirSano`, `validarAtaqueAColumna`, `validarAlcance`, `plazaAsediable`…). Ni un humano abre una batalla solo por llegar o alcanzar a alguien (Doc 5.12.3-5.12.4): solo con la orden.
- Un asedio solo lo abre un **ejército**; una columna personal no asedia pero sí se une (Doc 5.15.1b). Una plaza que ya está en una batalla no se asedia otra vez.
- Un humano que ataca a una Facción de bots combate contra los héroes bot que estén **dentro** de la plaza y su guarnición; si el humano es el único y nadie lo defiende, el asedio se juega igual, sin defensores.

## 3. Modelo de datos

### 3.1 `Batalla` (estado de la partida: `estado.batallas`)

| Campo | Contenido |
|---|---|
| `id` | UUID (§4.5), único entre partidas: las rutas de Conquest la nombran sin `gameId`. |
| `estado` | `convocando` → `asignada` → `en_curso` → `aplicada`, o `cancelada` / `fallida` (finales). |
| `iniciadaPor` | Héroe que comprometió el combate; puede cancelarla antes de que empiece. |
| `punto` | Dónde se ve en el mapa (en un asedio, la plaza). |
| `ticket` | El `BattleTicket` **congelado al abrir**, con su `ticketRevision` (empieza en 0). |
| `incorporaciones` | `IncorporacionBatalla[]`: los que se unen **después** del ticket. Solo crece. |
| `salidas?` | `{ heroeId, motivo, derrotado, en }[]`: quienes han salido. Solo crece. |
| `bloqueo` | `{ ejercitoIds, caravanaIds, asentamientoId?, campamentoId? }`: lo que se queda quieto, además de sus héroes. |
| `expiraEn` | Plazo del estado actual, en **tiempo de mundo** (§6). |
| `convocatoriaHasta?` | Solo un asedio: hasta cuándo se entra «como si estuviera desde el principio». |
| `iniciadaEn?`, `aplicadaEn?` | Instantes de mundo. |
| `asignacion?` | La `BattleServerAssignment` aceptada **más** `servidorId` (quién la registró). Los tokens que se van sumando viven aquí. |
| `resultado?` | El `BattleResult` aplicado, **entero** (idempotencia, §8.6). |

`estado.batallas` guarda las abiertas y las cerradas: **no se purga** (§13).

### 3.2 Lo que viaja a Conquest (el contrato v1)

La forma exacta es `contratos.schema.json` (JSON Schema draft-07, una entrada por entidad), con su espejo `dto.ts`. `SCHEMA_VERSION = 1`. Las claves están en castellano; los ids son cadenas no vacías; el tiempo de **mundo** (`Instante`, ms) no se mezcla con el **real** (`IsoUtc`).

```
BattleTicket        { schemaVersion, battleId, gameId, ticketRevision, contextoEstrategico, bandos{atacante,defensor}, mapa, reglas }
ContextoEstrategico  asedio { asentamientoId } | campo_abierto { punto, columnas: 'ejercitos'|'solitarios' }
                   | caravana { caravanaId, punto } | campamento_bandidos { campamentoId, punto }
BattleSide          { faccionId: id|null, capacidadMaxima, participantes: BattleParticipantSnapshot[], escuadrasSinHeroe: SquadSnapshot[] }
BattleParticipantSnapshot { heroeId, faccionId: id|null, controlador: 'humano'|'bot', heroe: HeroSnapshot, escuadras: SquadSnapshot[] }
HeroSnapshot        { displayName, classDefinitionId, nivel, genero, avatar, perksDesbloqueados, equipamiento,
                      atributosEfectivos, casillasInventarioLibres }
SquadSnapshot       { squadId, heroeId: id|null, tropaId, efectivosAutorizados, nivel, experiencia, moral,
                      habilidadesDesbloqueadas, formacionesDesbloqueadas, formacionSeleccionada }
mapa                { tipo:'mapa', mapaId, centro }  |  SettlementBattleSnapshot (solo asedio, §3.3)
reglas              { duracionMaximaSegundos, versionBalance, versionCatalogoTropas, versionCatalogoHeroe, versionCatalogoObjetos }
IncorporacionBatalla { schemaVersion, battleId, secuencia (1, 2, 3…), lado, participante }
BattleServerAssignment { schemaVersion, battleId, ticketRevision, intentoAsignacionId, instancia{host,puerto,protocolo}, tokensParticipante[{heroeId,token,expiraEn: IsoUtc}] }
TokensBatalla       { schemaVersion, battleId, intentoAsignacionId, tokensParticipante[] }
InicioBatalla       { schemaVersion, battleId, ticketRevision, intentoAsignacionId }
SalidaBatalla       { schemaVersion, battleId, ticketRevision, intentoAsignacionId, heroeId, motivo, derrotado, escuadras[{squadId,supervivientes,muertos}] }
BattleResult        { schemaVersion, battleId, resultId, ticketRevision, intentoAsignacionId, inicio, fin, ganador, razon, objetivos[],
                      porEscuadra[{squadId,desplegados,supervivientesAlCierre,muertos,xpGanada}],
                      porHeroe[{heroeId,participo,sobrevivioAlCierre,xpGanada,derrotado?,botin?}], versionServidor }
Botin               { objetos[{itemDefinitionId,cantidad,itemInstanceId?,estadisticas?}], monedas{bronce,plata,oro} }
```

- **`faccionId` de un bando** es `null` cuando el bando no es de nadie: los bandidos y **una columna sin bandera** (un recién llegado sin Facción, cuya columna lleva la Facción vacía `''`: se traduce a `null`, `bandoDeColumna`). El esquema rechaza la cadena vacía.
- **`SquadSnapshot` de lo que no es de nadie** (bandidos, carreteros): `heroeId: null`, nivel 1, experiencia 0, moral 100, formación 0 (`tropaSinDueno`); no persisten entre batallas.
- **`HeroSnapshot.atributosEfectivos` = `atributosBase`** (`ponytail:`): hasta que Conquest publique el catálogo de objetos (CQ-004) el equipo no suma. `casillasInventarioLibres = BATALLA.casillasInventario − ocupadas` (cuánto botín cabe).
- **`efectivosAutorizados`** = la **cantidad** de la escuadra al abrir: la fuerza **reservada**. El resultado cierra sobre esto, no sobre lo `desplegado`.
- Los catálogos: `versionCatalogoTropas` es `VERSION_CATALOGO_TROPAS` (`contratos/v1/catalogoTropas.ts`, que se sube a mano al cambiar cualquier tropa; un test impide regenerar el JSON sin subirla); `versionBalance` = `BALANCE_VERSION`; `versionCatalogoHeroe`/`Objetos` son cadenas fijas de `BATALLA` hasta que Conquest las publique.

### 3.3 `SettlementBattleSnapshot` (el mapa de un asedio)

Se congela **al abrir**, de la plaza real en ese instante, y no cambia aunque la plaza siga cambiando (lo construido o mejorado mientras dura se aplica a la plaza real, no al snapshot). Coordenadas **locales**: origen en el centro del Centro Urbano, `y` hacia abajo, una celda = `REJILLA_ASENTAMIENTO.tamanoCelda` unidades (`unidadesPorCelda`); la celda (col, row) ocupa de (col, row) a (col+1, row+1).

- `edificios`: solo los **internos** (`edificiosInternos`): `edificioId`, `tipo`, `posicion` (centro de la huella, en unidades), `ancho`, `alto` (huella en celdas, con la rotación ya aplicada), `nivelInterno?`, `estado` (`en_cola`/`en_construccion`/`activo`), `danado?`.
- `recintos`: `recintoId`, `nivel` **físico en pie** y solo las `celdas` **en pie**, en su orden de recorrido. Con una mejora en curso (`mejorandoA`) está en pie el anillo **entero** al nivel anterior; si no, las celdas hasta `avance` incluida (`celdas.slice(0, avance + 1)`). Un recinto sin celdas en pie no viaja. Las puertas son las celdas de clase `puerta`.
- `layoutVersion` = `LAYOUT_VERSION`.
- Las calles no van (Unity las toma de `trazadoPorAsentamiento` de la proyección).

## 4. Abrir la batalla

Cada contexto tiene su **apertura** (`Apertura`: contexto, punto, iniciadora, los dos bandos, bloqueo, mapa). Todos los bandos salen de **héroes sanos** y de las escuadras que llevan en ese contexto.

| Contexto | Quién lo abre | Atacante | Defensor | Bloquea | `mapa` |
|---|---|---|---|---|---|
| `campo_abierto` | `atacar` a otra columna (`validarAtaqueAColumna`: no a los tuyos ni a un aliado, misma clase, a ≤ `radioEncuentro`, héroe sano en las dos) | su columna | la columna atacada | las dos columnas | `mapa`, centro = posición del defendido |
| `caravana` | `atacar` a una caravana suelta | su columna | **sin héroes**: su escolta cedida (`escoltaIds`); sin escolta, una escuadra de **carreteros** sin dueño (`BATALLA.tropaCarreteros`); `faccionId` = la de su plaza de origen (o `null`) | la columna atacante y la caravana | `mapa`, en la caravana |
| `campamento_bandidos` | `atacar` a un campamento (un evento PvE) | su columna | una escuadra de `milicia_lanceros` con las unidades del nivel del campamento (`CAMPAMENTOS_BANDIDOS.niveles`), sin dueño; `faccionId: null` | la columna y el campamento | `mapa`, en el campamento |
| `asedio` | `atacar` a una plaza, **un ejército** | su ejército | los **residentes sanos que están dentro** (`heroesQueDefienden`: conectados, ubicación = esa plaza), cada uno con las escuadras de su **loadout activo** que tiene en el campamento y no en la guarnición; más la **guarnición** sin héroe (`escuadrasSinHeroe`); `faccionId` = la de la plaza | el ejército y la plaza | `SettlementBattleSnapshot` (§3.3) |

- **Los héroes de una columna** son los sanos de `participantes`, con las escuadras suyas cuyo contenedor es **esa** columna y que tienen efectivos. Los heridos no entran (sus escuadras ni se reservan).
- **El lado defensor de un asedio no incluye a los visitantes** (héroes de fuera, aunque sean aliados, que estén dentro): entran, si quieren, **uniéndose** durante la convocatoria (§5). El combate con números sí los mete de oficio ([`Combate.md`](Combate.md) §3.2).
- **Capacidad** (`capacidadMaxima`, en héroes): `BATALLA.capacidad.asedio` en un asedio, `.resto` en las demás. Las escuadras sin héroe no ocupan plaza. **Al abrir no se recorta**: si la columna lleva más héroes que la capacidad, todos van al ticket (la cola de los que sobran es cosa de Unity); la capacidad solo frena a quien se une.
- **Duración** (`reglas.duracionMaximaSegundos`) = `BATALLA.duracionMinutos.{asedio|resto} × 60`. Si se agota, **gana el defensor**, en cualquier batalla (§8.6).
- **Convocatoria** (solo asedio): `convocatoriaHasta = ahora + BATALLA.convocatoriaSegundos`. Mientras dura (`enConvocatoria`: `ahora < convocatoriaHasta`), quien se une **entra en el ticket** (revisión 0) como uno de los del principio, y el ticket **no se publica ni se asigna** (§8.1, §8.2). Como el tiempo de mundo avanza por ticks de un minuto, en la práctica la convocatoria dura hasta el siguiente tick. Los demás contextos no la tienen y salen hacia Unity al abrirse.
- **Candado** (`reservaBatalla`): se pone `{ battleId }` a **todas las escuadras con dueño** del ticket (`escuadrasConDueno`: las de los héroes y las sin héroe que tengan `heroeId`, como la guarnición y la escolta). Mientras lo tengan: no se **reponen** y no entran ni salen de la **guarnición** ([`Heroe_Y_Tropa.md`](Heroe_Y_Tropa.md)); los loadouts no se ven afectados.
- **El evento** `batalla.abierta` se atribuye a las plazas implicadas (§10) y al historial del héroe.

### 4.5 El `battleId`

Un UUID con la forma `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`, **determinista**, sin aleatoriedad global (sale de la partida y de su contador de ids): `idDeBatalla(gameId, n)` con `n = ctx.ids.siguiente()`:

```
resumen(semilla)  = FNV-1a de 32 bits sobre los caracteres del gameId (primo 0x01000193, empezando por `semilla`), en hex de 8 cifras
a = resumen(0x811c9dc5)       b = resumen(0x01000193)       c = n en hex de 16 cifras
battleId = a '-' b[0..4) '-' b[4..8) '-' c[0..4) '-' c[4..16)
```

## 5. Lo bloqueado y el candado de comandos

**Una batalla activa** es la que está `convocando`, `asignada` o `en_curso` **y** dentro de plazo (`ahora < expiraEn`): una vencida deja de bloquear ya, aunque la cierre el siguiente tick. Lo mismo con el candado de las escuadras: antes de ejecutar cualquier comando con candado, `sinCandadosVencidos(estado, ahora)` suelta el `reservaBatalla` de las escuadras de las batallas vencidas (sin cerrarlas ni narrar nada: eso es del tick), así que reponer o guarnecer no dependen de que el tick haya pasado.

`bloqueosDe(estado, ahora)` reúne, de las activas: los **héroes que combaten** (`participacionesDe`), los ids de `bloqueo.ejercitoIds`, `caravanaIds`, el `asentamientoId` asediado y el `campamentoId` atacado.

- **En el tick**: lo bloqueado **no se simula** (los ejércitos y caravanas bloqueados se apartan, no se mueven, no comen, no son perseguidos; vuelven al final de las listas), y el héroe bloqueado **no sale del mundo** aunque cumpla su espera ([`Tick.md`](Tick.md) §4.1).
- **En un comando** (`conCandadoDeBatalla` envuelve **todo el registro** salvo los comandos de §5.1): se rechaza con **`batalla.bloqueado`** (200, `ok: false`, sin gastar un dado ni un id) si `tocaLoBloqueado(estado, ahora, actor, params)`:

```
tocaLoBloqueado:  (solo mira si hay alguna batalla en estado activo)
  heroeBloqueado(id) = id ∈ bloqueos.heroes  ∨  (su ubicación es una plaza ∈ bloqueos.asentamientos)    ← quien está DENTRO de la plaza asediada tampoco da órdenes
  devuelve verdadero si CUALQUIERA de:
    – el actor, o params.heroeId, está bloqueado
    – alguno de params.{asentamientoId, atacanteId, defensorId, destinoId, origenAsentamientoId, asentamientoAId, asentamientoBId} ∈ bloqueos.asentamientos
    – params.ejercitoId ∈ bloqueos.ejercitos
    – alguno de params.{caravanaId, desdeCaravanaId, haciaCaravanaId} ∈ bloqueos.caravanas
    – params.objetivo = {tipo, id} apunta a un ejercito / caravana / campamento bloqueado
```

Mira los parámetros **por su nombre** (la convención de todo el registro): cubre los comandos que lleguen sin tocarlos uno a uno. **Marchar hacia** una plaza asediada sí se puede (se llega y se espera a la puerta, o se une uno a la batalla).

### 5.1 Los que no llevan candado

`unirseABatalla` y `cancelarBatalla` (actúan sobre la propia batalla) y **`conectarse` y `desconectarse`**: cerrar el cliente en plena batalla tiene que **anotarse** (el tick lo aplica al terminar, D33b) y volver a conectarse tiene que **anular** esa petición. Los mensajes de Conquest y el tick no pasan por el registro de comandos, así que no llevan candado.

## 6. Plazos y vencimientos

`expiraEn` es de **mundo** (no avanza con el proceso caído: una caída no consume tiempo de mundo, ni el margen se gasta).

| Cuándo | `expiraEn` = |
|---|---|
| Al abrir | `convocatoriaHasta` (ahora, si no hay convocatoria) + `BATALLA.plazoAsignacionMinutos` |
| Al asignarla (`asignada`) | ahora + `BATALLA.plazoInicioMinutos` |
| Al empezar (`en_curso`) | ahora + `duracionMaximaSegundos / 60 + BATALLA.margenMinutos` minutos |
| Tras una salida previa al inicio (vuelve a `convocando`) | ahora + `BATALLA.plazoAsignacionMinutos` |

El margen existe para que, tras una caída corta del servidor (que recupera los ticks de golpe), una batalla que ya terminó no se dé por fallida.

**`vencerBatallas`** (primer paso del tick): toda activa con `instante ≥ expiraEn` pasa a **`fallida`** sin castigo: suelta los candados de sus escuadras (`cerrarSinResultado`), nadie gana ni pierde, y se narra `batalla.fallida`. Esto vale desde `convocando`, `asignada` y `en_curso`.

## 7. Unirse

**`unirseABatalla { heroeId, battleId, lado? }`** (comando de jugador, §5.1 sin candado). La batalla tiene que estar activa (si no, `batalla.no_existe`). Se rechaza (`batalla.invalida`) si:

1. el héroe está **herido**;
2. ya está en una batalla (`bloqueos.heroes`);
3. no tiene **columna**, o su columna ya está en una batalla;
4. su columna está a más de `LOGISTICA.radioEncuentro` del `punto` de la batalla (la plaza asediada, en un asedio);
5. `ladoParaUnirse` no le da bando (§7.1);
6. el bando se pasaría de su `capacidadMaxima` (`yaDentro + entran > capacidad`): si está lleno, **no entra**; la cola es solo de los que ya estaban al abrir.

Entra **su columna entera**: todos sus héroes sanos con lo que llevan en ella (`heroesDeColumna`). Si `enConvocatoria`, entran **en el ticket** (como los del principio); si no, cada héroe es una **incorporación** (`secuencia = incorporaciones.length + i + 1`) y **el ticket no cambia ni sube `ticketRevision`** (no invalida la asignación ni los tokens de nadie). Su columna se suma al `bloqueo.ejercitoIds` y sus escuadras reciben el candado. Devuelve `{ secuencias }`; evento `batalla.refuerzos`.

### 7.1 A qué bando (`ladoParaUnirse`)

| Contexto | Regla |
|---|---|
| `campo_abierto`, `columnas: 'ejercitos'` (batalla campal) | **nadie de fuera**: rechaza |
| `campo_abierto`, `columnas: 'solitarios'` (persecución) | solo una **columna personal** (un ejército no se une); **cualquier Facción**, y **`lado` es obligatorio** |
| `campamento_bandidos` (PvE) | **cualquiera**, también sin Facción, **siempre atacante**; pedir `defensor` rechaza (nadie ayuda a los bandidos) |
| `asedio` y `caravana` | hace falta **Facción** (si no, rechaza). **Defensor** si su Facción es la del defensor o **aliada para batalla** (alianza **o** vasallaje activos, en cualquier sentido: `sonAliadasEnBatalla`, distinto de `estanAliadas`); **atacante** si es la del atacante, o —si el Rey de la Facción atacante lo admite (`admiteOtrasEnAtaques`)— de cualquier otra que no sea del defensor ni aliada suya; si no, rechaza («tu Facción no combate aquí»). Si pide un `lado` distinto del que le toca, rechaza |

**Quién cuenta como aliado para unirse**: el vasallaje también, para que señor y vasallo se defiendan entre sí y ninguno se una al ataque contra el otro (decisión del autor, 2026-10-06).

## 8. Lo que manda Conquest

**Rutas** (`/v1/batallas/*`, sin `/admin` ni `/jugador`), con `Authorization: batalla-servidor <token>`:

| Ruta | Para qué |
|---|---|
| `GET /batallas/pendientes` | Las `convocando` activas y **fuera de convocatoria**: `{ battleId, gameId, ticketRevision }`. Idempotente y sin efectos; el orquestador hace polling. |
| `GET /batallas/:id/ticket` | El ticket vigente, **recuperable siempre** (también de una cerrada); `404` mientras dure la convocatoria. |
| `GET /batallas/:id/incorporaciones` | Las incorporaciones vigentes (sin las de quien salió antes de empezar). |
| `POST /batallas/:id/asignacion` | Dónde se juega. |
| `POST /batallas/:id/inicio` | La partida empezó de verdad. |
| `POST /batallas/:id/tokens` | Los tokens de los humanos que se unieron después de la asignación. |
| `POST /batallas/:id/salidas` | Un héroe sale. |
| `POST /batallas/:id/resultado` | El resultado. |
| `GET /jugador/partidas/:g/batallas/:id/asignacion` | **El token del propio jugador** (sesión de jugador; ver §8.7). |

- **La credencial** es de **servidor**, nunca la de una sesión de jugador, y **no es la misma que el token de entrada** de cada participante: un token de jugador no puede asignar, iniciar ni firmar; la credencial de servidor no se reenvía a un cliente. `SERVIDORES_BATALLA=servidorId:token,…` es **además el interruptor del ciclo entero**: sin ella (`batallasEnUnity = false`) todo se resuelve con números. Una entrada mal escrita es un fallo de arranque. El token se compara en **tiempo constante**. Sin credencial válida, `401`.
- **El `battleId` se busca en todas las partidas abiertas** (no hay `gameId` en la ruta); no existe, `404`.
- Cada mensaje se valida contra el **schema** de su definición (forma: `400` con los errores) y contra la ruta (`battleId` del cuerpo distinto del de la ruta: `400`), tras comprobar **`schemaVersion`** (distinta: `409` con `schemaVersionEsperada`). Después entra como un **comando de la cola de la partida** (el diario lo anota con su nombre: `registrarAsignacion`, `confirmarInicio`, `registrarTokens`, `registrarSalida`, `aplicarResultado`, con el actor `batalla-servidor:<id>`). Una operación rechazada responde `404` si la batalla ya no está activa (`batalla.no_existe`) y `409` con el código en cualquier otro caso. Éxito: `200 { battleId, estado }`.
- **Un mensaje repetido que ya estaba aplicado responde bien sin cambiar nada** (`sinCambios`: ni versión, ni evento, ni dado): es lo que deja a Conquest reintentar, espaciando los intentos, hasta recibir un 2xx o un 4xx no transitorio.
- La batalla tiene que estar **activa**; una cerrada rechaza todo mensaje salvo el `resultado` repetido de una `aplicada`.

### 8.1 Asignación (`convocando → asignada`)

- La **misma** asignación (mismo `intentoAsignacionId` **y** mismo servidor) repetida: sin cambios.
- Si no, la batalla tiene que estar `convocando` (si no, **`batalla.ya_asignada`**: otro servidor la tiene; gana la primera), **fuera de convocatoria**, con la **`ticketRevision` vigente**, y los `tokensParticipante` solo de **héroes humanos de esta batalla** (los bot no se conectan).
- Se guarda `asignacion = { ...mensaje, servidorId }` y `expiraEn` = §6. No hace falta reservar antes: la cola serial de la partida impide dos asignaciones a la vez.
- Narra `batalla.asignada`.

### 8.2 Inicio (`asignada → en_curso`)

Solo el servidor **de la asignación activa** (`intentoAsignacionId` y `servidorId`), con la `ticketRevision` vigente. Ya `en_curso`: sin cambios. Guarda `iniciadaEn` y `expiraEn` (§6). Narra `batalla.en_curso`.

### 8.3 Tokens

Solo del servidor de la asignación activa, solo de humanos de la batalla. Un token nuevo del mismo héroe **sustituye** al anterior. **Sin evento**: tener ya su token no es noticia para nadie más.

### 8.4 Salida (CQ-011)

Un héroe sale de la batalla. **Repetir** la salida de quien ya salió: sin cambios (se mira **antes** de comprobar la asignación, porque tras una salida previa al inicio la revisión y la asignación ya no son las del mensaje). Si no: asignación activa y revisión vigente; el héroe tiene que **combatir** en ella.

- **`no_conectado`** solo con la batalla `asignada` (**antes de empezar**): sale **sin castigo**, con sus escuadras intactas y libres; **sube `ticketRevision`** y la batalla vuelve a `convocando` (la asignación y los tokens anteriores dejan de valer, `expiraEn` = §6); su participación se quita del ticket y **no entra nadie en su lugar**. Se **cancela sin castigo** si ya no queda ningún héroe **humano** o el atacante se queda **sin héroes**.
- **`eliminado`, `abandono`, `desconexion`** solo con la partida **`en_curso`**: se aplican las **bajas** de sus escuadras (`supervivientes + muertos` tiene que cerrar sobre lo autorizado, cada `squadId` suyo y sin repetir) y, si `derrotado`, **queda herido** (`herir`, Doc 5.16.4). **El ticket no cambia.** El resultado final ya no lo repite.
- En los dos casos: se **libera el candado** de todas sus escuadras, y su **columna deja de estar quieta** si no queda nadie más de ella en la batalla. Si salió quien la inició, `iniciadaPor` pasa al **primer atacante que queda**.
- Narra `batalla.cancelada` (si se cancela), `batalla.revisada` (no conectado) o `batalla.salida`, y deja una línea en su historial.

### 8.5 Resultado: el checklist (`validar`)

Cualquier fallo **rechaza el resultado entero** (nunca se aplica a medias). La batalla debe estar `en_curso` y dentro de plazo. En este orden:

1. la asignación activa (`intentoAsignacionId` + `servidorId`) y la **revisión vigente**;
2. **sin ids repetidos** en `porEscuadra` ni `porHeroe`;
3. **completitud**: `porEscuadra` cubre **exactamente** las escuadras de la batalla (`escuadrasDe`: las de los participantes presentes, las incorporadas y las sin héroe), también las nunca desplegadas (con ceros); `porHeroe`, exactamente a los héroes que **siguen** combatiendo (no a quien salió antes);
4. por escuadra: `supervivientesAlCierre + muertos == efectivosAutorizados` y `desplegados ≤ efectivosAutorizados`;
5. `razon = tiempo_agotado` ⇒ `ganador = defensor`;
6. `fin` posterior a `inicio` y la duración **≤ `duracionMaximaSegundos`**;
7. **botín**: cada `itemInstanceId` es **nuevo** en la partida (no repetido ni ya existente en inventarios y equipos); solo con `participo: true`; los objetos **caben** (`≤ casillasInventarioLibres` del héroe en el ticket);
8. lo que valida el **schema** por forma: todas las cifras son enteros ≥ 0 (XP incluida, monedas, supervivientes…), `cantidad` de un objeto ≥ 1, el id de batalla es un UUID, el tiempo es ISO real en UTC.

No se comprueba (hoy): que cada `itemDefinitionId` exista en el catálogo de objetos (`ponytail:` hasta que Conquest lo publique, CQ-004; decisión del autor, 2026-09-15), ni un tope de XP o de monedas por batalla (no hay hasta que Conquest publique su curva, CQ-001), ni una «versión de build o de balance autorizada» (el resultado solo trae `versionServidor`, que se guarda y no se compara).

### 8.6 Resultado: idempotencia

`resultado` se **guarda entero** en la batalla (no una huella): repetir el mismo (`JSON.stringify` idéntico) sobre una `aplicada` responde bien y **no cambia nada**; **otro distinto** sobre una ya aplicada se rechaza («ya se aplicó con otro resultado», `409`). Sobre una `cancelada` o `fallida` (o una que no está `en_curso`), `409`. Aplicar las consecuencias y pasar a `aplicada` es **una sola mutación**: si falla guardar, el comando no tuvo efecto.

### 8.7 El token del jugador

`GET /jugador/partidas/:g/batallas/:id/asignacion` (sesión de jugador con héroe): `404` si la batalla no está activa; `403` si no combate en ella; `404` mientras no haya asignación o token para él; si no, `{ battleId, instancia, token, expiraEn }`. **BronzeAge nunca verifica el token**: solo lo custodia y controla **quién** lo lee. El token **nunca** viaja por un canal colectivo ni por la proyección, y el estado de administración lo vacía.

## 9. Aplicar el resultado

`aplicarResultado` (`en_curso → aplicada`), en este orden y en **una** mutación. Repetir lo ya aplicado: §8.6.

1. **Heridos.** Quedan heridos (`herir`, 2 min de mundo, Doc 5.16.4) los de `porHeroe` con `derrotado: true`; **sin ese campo**, los del **bando perdedor**. Un `derrotado: false` explícito **no** hiere aunque haya perdido, y un ganador con `derrotado: true` sí.
2. **Lo que trajo.** A cada escuadra con entrada: `cantidad = supervivientesAlCierre` y la experiencia que dice Unity (`conExperiencia`: una tropa **prestada** no gana); a cada héroe, su `xpGanada` **sumada** a `experienciaHaciaSiguienteNivel` —**el nivel no cambia** hasta que Conquest publique su curva (CQ-001; decisión del autor, 2026-09-15)— y su botín (objetos en casillas libres, tipo `visual` y precio 0 hasta CQ-004; monedas sumadas). Se **sueltan todos los candados** de la batalla.
3. **Logros del servidor** (`contadoresDeBatalla`): `batallas.libradas` +1; `batallas.conHoplitas` si hay una escuadra de `hoplitas_ciudadanos`; en `campo_abierto`, `batallas.campoAbierto`; ganando el atacante, `caravanas.destruidasOCapturadas` (caravana) o `bandidos.campamentosDestruidos` (campamento); en un asedio, `asedios.contraMurallaCompleta` (algún recinto con integridad ≥ 1), y si resiste `asedios.resistidosEnCombate` (más `asedios.resistidosConResidentes` si hubo defensores con héroe) o, si cae con muralla completa, `conquistas.conMurallaCompleta`.
4. **Consecuencias** (`consecuencias`), según el contexto:
   - **Asedio, gana el atacante** → **conquista**: la plaza pasa a la Facción atacante con el saqueo de siempre (`aplicarConquista`, Doc 5.12.9); `desalojarResidentes` (quien estaba dentro sale; los defensores que combatieron salen con las escuadras que les quedaron; el resto del campamento a 0 y a su nueva residencia, [`Combate.md`](Combate.md) §9); `registrarDerrota`. Si el bando atacante no tiene Facción, **no se conquista nada**. **Unity no reporta daño**: lo pone el saqueo. Gana el defensor: nada más (no hay pérdida de carro en un asedio).
   - **Mundo abierto** (columna, caravana o campamento): cada columna del bando que **pierde** entrega **la mitad de su carro** (`trasDerrota`), que se reparte entre las columnas del que gana, lo que quepa (contra una caravana o un campamento esa mitad no se la lleva nadie). Si **gana el atacante**: **caravana** → su escolta queda a 0 y vuelve al campamento de su héroe, la caravana desaparece y `MILITAR.umbralCapturaCaravana` de su carga va al carro de los atacantes; **campamento** → oro de bandidos para cada héroe atacante (con el decrecimiento por reincidencia, `botinDeBandidos`) al carro de su columna, el campamento desaparece y se agenda su reaparición.
5. **Chatarra** (Doc 4.2.1): el equipo de **todos los caídos, de los dos bandos**, se lo queda quien gana: a los carros de sus columnas (a partes iguales, lo que quepa); si defiende una plaza y gana, a **su almacén**; sin columna que la lleve (caravana o campamento vencedores) se pierde.
6. **XP de Facción**: por cada héroe que **participó**, `NIVEL_FACCION.xp.combate` a **su propia Facción** (un bando puede mezclar); en un asedio ganado por el atacante, además `NIVEL_FACCION.xp.conquista` a la atacante. Asediar a un **aliado** (alianza) cuesta `REPUTACION.penalizacionAtacarAliado`.
7. **Reputación por defender a un vasallo** (Doc 2.7): si el bando defensor es de una Facción **vasalla**, su señora gana reputación si algún héroe suyo defendió y el bando aguantó, y la pierde, menos, si perdió y ninguno estuvo (`ajustesPorDefensaDeVasallo`, compartido con el combate con números).
8. La batalla queda `aplicada` con `resultado` y `aplicadaEn`.

Los efectos de una batalla en curso sobre las plazas (construir, mejorar…) se aplican a la plaza real, no al snapshot congelado.

## 10. Lo que se cuenta

**Eventos** (`eventosDeBatalla`), uno **por plaza implicada** (la de origen de cada columna bloqueada y de cada caravana, y la asediada): se entera cada hogar, no el mundo. Códigos: `batalla.abierta`, `.refuerzos`, `.asignada`, `.en_curso`, `.salida`, `.revisada`, `.aplicada` (con `ganador`), `.cancelada`, `.fallida`. Contra un **campamento de bandidos** el prefijo es **`evento_pve.`** en lugar de `batalla.` (no es una batalla entre Facciones).

`PayloadBatalla` = `{ battleId, contexto, punto, bandos (por lado: Facción, nº de héroes, capacidad), faccionAtacanteId, faccionDefensoraId, ganador? }`: **nunca** tokens, escuadras ni listas de héroes. Las épicas de los Aedas leen de aquí las Facciones de cada bando (Doc 6.7).

**Quién los recibe:**
- El canal `asentamiento/<id>` de cada plaza implicada, y los de **batalla** (`batalla/<id>`, abierto a quien combate —también a quien ya salió— o la ve ahora bajo la niebla), [`Proyecciones_Y_Autorizacion.md`](Proyecciones_Y_Autorizacion.md) §7.2.
- **El cursor** `GET …/eventos`: los de plaza propia, los que nombran al héroe, **y los de las batallas en que combate o combatió** (§14): sin esto, un héroe sin plaza —una columna salida de un campamento de mercenarios, cuyos eventos van a `asentamientoId: ''`— no los recibiría nunca, porque el evento no lleva lista de héroes.

**La proyección** (`BatallaVisible`): `{ battleId, estado, contexto, punto, bandos, ladoPropio? }`; sin tokens, sin escuadras. Se ve la batalla en el mapa si combate el héroe **o** su punto cae bajo la niebla (`seVeAhora`); **las columnas y caravanas que están en una no viajan** en los avistados: la batalla las sustituye. Solo las **activas** viajan. Detalle de capas y canales: [`Proyecciones_Y_Autorizacion.md`](Proyecciones_Y_Autorizacion.md) §6.

## 11. Parámetros

<!-- parametros:BATALLA -->
```json
{
  "capacidad": {
    "asedio": 5,
    "resto": 5
  },
  "duracionMinutos": {
    "asedio": 30,
    "resto": 15
  },
  "convocatoriaSegundos": 30,
  "plazoAsignacionMinutos": 5,
  "plazoInicioMinutos": 5,
  "margenMinutos": 5,
  "tropaCarreteros": {
    "tropaId": "milicia_lanceros",
    "unidades": 13
  },
  "casillasInventario": 72,
  "versionCatalogoHeroe": "conquest-heroes-1",
  "versionCatalogoObjetos": "conquest-items-1"
}
```
<!-- /parametros -->

Los que intervienen y viven en otro documento: `LOGISTICA.radioEncuentro` (distancia de choque y de unión; [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md)), `MILITAR.umbralCapturaCaravana`, `NIVEL_FACCION.xp`, `REPUTACION.penalizacionAtacarAliado`, `CAMPAMENTOS_BANDIDOS.niveles` y `PRESENCIA` ([`Combate.md`](Combate.md), [`Presencia_Y_Convocatoria.md`](Presencia_Y_Convocatoria.md)).

## 12. Invariantes

| Invariante | Test |
|---|---|
| Sin servidores de batalla, `atacar` se sigue resolviendo con números | `session/__tests__/batallas.test.ts` |
| El ticket de un campamento, de una caravana y de un asedio **cumple el schema**, y también el de una columna **sin Facción** (bando `null`) | `batallas.test.ts`, `contratos/v1/__tests__/contratos.test.ts` |
| El candado impide reponer y entrar o salir de la guarnición; se suelta al cerrar | `batallas.test.ts`, `comandosHeroe.test.ts`, `tropas.test.ts` |
| Lo que combate no recibe órdenes ni se mueve; nadie más lo ataca; la plaza asediada no recibe órdenes | `batallas.test.ts` |
| Quien se desconecta en plena batalla sale del mundo al terminar, no antes; volver a conectarse lo anula | `batallas.test.ts` |
| Convocatoria de un asedio: quien se une entra en el ticket y no se asigna hasta que acaba; después, incorporación | `batallas.test.ts` |
| Quién se une y a qué bando (campal, persecución, PvE, asedio, aliados y vasallos, Rey que admite) | `session/__tests__/entradaEnBatalla.test.ts` |
| Nadie asigna, inicia ni firma sin ser el servidor de la asignación activa y la revisión vigente; el reintento responde bien | `batallas.test.ts`, `server/__tests__/batallas.test.ts` |
| El resultado se rechaza **entero** si falla el checklist; repetido, no cambia nada; otro distinto, se rechaza | `batallas.test.ts` |
| Una salida previa al inicio sube la revisión y reabre la convocatoria; sin humanos, se cancela; en curso, aplica las bajas y hiere si `derrotado` | `batallas.test.ts` |
| El herido sale de `derrotado`, no del bando | `batallas.test.ts` |
| Conquista, captura, destrucción de campamento, carro, chatarra y XP de Facción | `batallas.test.ts` |
| Los eventos del ciclo no llevan tokens, escuadras ni listas de héroes; sí el recuento por bando | `session/__tests__/canalDeBatalla.test.ts` |
| Los eventos de una batalla llegan por el cursor a quien combate en ella aunque no tenga plaza | `batallas.test.ts` |
| El candado de una escuadra no sobrevive al plazo de su batalla, aunque el tick aún no la haya cerrado | `batallas.test.ts` |
| Quien puede seguir una batalla: quien combate, o la ve; no un forastero | `canalDeBatalla.test.ts` |

## 13. Casos límite y decisiones que parecen raras

- **Un asedio sin defensores se juega igual**: la batalla se abre aunque el defensor no tenga ni héroes ni guarnición; basta con que haya un humano atacante.
- **El ticket de un asedio no recorta por capacidad**: el atacante puede llevar más héroes que `capacidadMaxima`; Unity hace la cola.
- **El defensor de un asedio no incluye a los visitantes aliados**: se unen ellos (con su columna aparcada a la puerta, a ≤ `radioEncuentro` de la plaza). Los residentes sí entran de oficio.
- **Entrar durante la convocatoria no sube la revisión** y **entrar después tampoco**: la revisión solo sube con una salida previa al inicio.
- **`no_conectado` y `derrotado`**: la primera sale sin castigo y reabre la convocatoria; la segunda hiere. Quien sale antes de empezar no se sustituye.
- **Un `derrotado: false` explícito protege** al perdedor de la herida; quien no lo trae, se hiere con su bando.
- **Un mensaje a una batalla cerrada** responde `404` (asignación, inicio, tokens, salida) o `409` (resultado), salvo el `resultado` idéntico de una `aplicada`, que responde bien.
- **La Facción de una columna sin bandera** es `null` en el ticket y en `faccionAtacanteId`/`faccionDefensoraId`; una conquista con atacante sin Facción **no ocurre**.
- **El plazo vencido suelta todo al instante**, columna y escuadras (`bloqueosDe` mira el plazo; `sinCandadosVencidos` suelta el `reservaBatalla` al validar cada comando). El tick solo cierra la batalla (`fallida`) y la narra. Con el reloj del motor (los comandos corren en el instante del tick y el tick cierra antes) esa ventana no llega a verse; la comprobación perezosa lo garantiza sin depender de él.
- **`estado.batallas` no se purga**: las cerradas se quedan (hace falta `aplicada` para la idempotencia y el ticket es «recuperable en cualquier momento»); crece con cada batalla de Unity (§15).
- **El `battleId` solo es único entre partidas si el `gameId` lo es**: sale de un resumen de 64 bits del `gameId` más el contador; no hay aleatoriedad.
- **La salida del mundo espera**: aunque el héroe lleve su `desconectaEn` cumplido, no sale mientras combate; la hace el primer tick tras acabar. Si Conquest lo da por desconectado, lo saca de la batalla con una `SalidaBatalla` (`desconexion`, §8.4) y entonces sale en el tick siguiente.

## 14. Lo que se corrigió al documentar (2026-10-12)

El ciclo se escribió antes de varias cosas que han cambiado (columnas sin bandera, presencia, canales personales). Al contrastarlo con el contrato y el modelo de datos salieron cinco desajustes **del código**, ya corregidos con su test:

1. **Un recién llegado sin Facción que atacaba un campamento de bandidos generaba un ticket inválido.** Su columna lleva la Facción vacía (`''`) y el bando salía con `faccionId: ''`; el contrato exige un id no vacío o `null`. Ahora es `null` (`bandoDeColumna`).
2. **Cerrar el cliente en plena batalla no se anotaba.** `desconectarse` llevaba el candado de batalla y se rechazaba con `batalla.bloqueado`; el tick ya sabía esperar al fin de la batalla (D33b) pero la petición nunca llegaba a registrarse, así que el héroe se quedaba «conectado» para siempre. `conectarse` tampoco podía anular la petición. Ahora los dos están libres de candado.
3. **Los eventos de una batalla no llegaban por el cursor a quien no tiene plaza** (el caso más habitual de quien ataca a los bandidos al llegar): el evento va a la plaza de origen de su columna, que para una salida de campamento es `''`, y no lleva lista de héroes. Ahora el cursor incluye los de las batallas en que combate.
4. **Código muerto**: `faccionesEnBatalla` (la IA de Facciones NPC «no gobierna mientras dura») no la usaba nadie desde que no hay Facciones NPC en el motor; se borra con su test.
5. **El candado de las escuadras de una batalla vencida lo soltaba solo el tick** (el contrato pide comprobar el plazo justo antes de validar la acción). Ahora lo suelta `sinCandadosVencidos` al validar cada comando con candado, sin esperar al tick. Con el reloj actual la ventana no se llegaba a ver, pero un servidor sin tick (la reconstrucción) sí la habría tenido.

## 15. Divergencias

**Abiertas** (a decidir por el autor, también en [`Divergencias.md`](Divergencias.md)):

- **`estado.batallas` crece sin límite.** Cada batalla de Unity cerrada se queda en el estado y en el snapshot (un asedio lleva el mapa de la plaza congelado). Falta decidir cuánto se conserva (la idempotencia del resultado y el `GET …/ticket` «recuperable en cualquier momento» piden que una cerrada siga unos minutos).

**Pendientes de Conquest** (no son divergencias del servidor; el contrato las anota como decididas):

- `atributosEfectivos` sin el equipo y objetos de botín sin tipo ni precio (CQ-004); la XP del héroe se suma pero **el nivel no sube** (CQ-001); no se valida que el objeto exista ni un tope de XP o monedas.
