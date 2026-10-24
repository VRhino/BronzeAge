# Batalla de Unity: el ciclo desde BronzeAge

## 1. Alcance y fuentes

Cubre **todo lo que BronzeAge hace con una batalla que se juega en Unity**: cuándo se abre, qué se congela en el ticket, qué queda bloqueado mientras dura, quién puede unirse, los cinco mensajes que manda el servidor de batalla, cómo se aplica el resultado y qué se cuenta a los jugadores. **Unity decide todo lo de dentro** (banderas, reaparición, cómo se gana); BronzeAge decide que la batalla ocurra, dónde, quién entra, quién sale y qué le queda a cada uno (Doc 5.15.1b).

- **Verificado contra:** `1850a73` + los cuatro arreglos de §14 (2026-10-12).
- **Implementado en Rust (2026-10-19, fase 4A-2):** `crates/combate::{ciclo, resultado, union}` como tipos y transiciones puras: `Batalla` con sus estados, ticket congelado, convocatoria, incorporaciones, salidas, candado (`escuadras_con_candado`, `candados_vencidos`), bloqueos (`bloqueos_de`), plazos, vencimiento, cancelación, retención (`retirable`), asignación, inicio, tokens, salida, checklist del resultado (`validar_resultado`, §8.5), idempotencia (§8.6), efectos (`efectos_del_resultado`: heridos, escuadras, experiencia, botín, candados), `lado_para_unirse` (§7.1) y `uuid_de_batalla` (§4.5). El núcleo (fase 4E) solo guarda las `Batalla`, programa los eventos a `expira_en` y `convocatoria_hasta` (prioridad 1) y aplica los efectos. El ticket lleva lo que el ciclo decide; los snapshots de atributos, equipo y mapa de la plaza son del codec (4E/4H).
- // DUDA (decidido al implementar): el bloqueo añade `pueblo` y `avanzada` a los cuatro campos de §3.1 (el saqueo y el ataque a una avanzada también dejan quieto su objetivo); una avanzada «se juega como asedio» en capacidad (15) y duración (30 min) pero **no** tiene convocatoria (solo el asedio); quien sale de una batalla no vuelve a entrar en ella; la salida de §8.4 no tiene que cubrir todas las escuadras del héroe (las que no vienen conservan su cantidad); la experiencia de escuadra que trae Unity son puntos enteros (se guardan ×1000, en milésimas); las escuadras sin dueño (carreteros, bandidos, mineros) usan el héroe `SIN_DUENO` = 0.
- **Implementado en Rust (2026-10-21, fase 4E: el ciclo en el núcleo).** `crates/nucleo/src/nucleo/batallas.rs` conecta la máquina de estados de `crates/combate` al mundo; el estado es `EstadoBatallas` (`estado_batallas.rs`, `codec_batallas.rs`) y la foto pasa a **`BAN13`** (rechaza `BAN12`; la regla `batallas_en_unity` entra en la cabecera y en la huella: recuperar con otro valor falla).
  - **Se abre** con `Atacar` (95) siempre que `Reglas::batallas_en_unity` (**también bot contra bot**, cuarta ronda), después de las mismas validaciones del combate con números y de declarar la guerra, si hacía falta; sin servidores sigue el combate numérico de la 4D (y no gasta ni un id más: la rechazada no deja rastro). Cada contexto tiene su apertura: columna (campal o persecución según la clase), caravana (escolta cedida o carreteros sin dueño; tipo A/B por la zona del instante del ataque), campamento de bandidos (milicia del nivel), asedio (la guarnición sin héroe; **nadie más entra de oficio, tampoco los residentes**: se unen ellos, §7.1), saqueo de pueblo (30 mineros de serie) y avanzada (guarnición de serie y de jugadores). Lo que no es de nadie (carreteros, bandidos) recibe un id de la partida para que el resultado lo cubra.
  - **Órdenes nuevas** (a continuación de las de la 4D): `FijarControlador` (98, administración: los héroes de `CrearEjercito` nacen bot), `UnirseABatalla` (99) y `CancelarBatalla` (100) —de un jugador, sin candado— y los cinco mensajes del servidor de batalla `RegistrarAsignacion` (101), `ConfirmarInicio` (102), `RegistrarTokens` (103), `RegistrarSalida` (104) y `AplicarResultado` (105), con el actor `batalla-servidor:<id>` (`Peticion::de_servidor_de_batalla`; otro actor: `batalla.sin_credencial`). El servidor de la asignación lo pone la credencial, no el mensaje. `ComandoSesion::Batalla` ya se traduce en `sesiones.rs`: asignación, inicio, salida y resultado del contrato (`MensajeDeServidorDeBatalla`; el contrato 05 no trae un mensaje de tokens) y lo rechazado vuelve a la sesión como un `Aviso` con el código estable. Repetir un mensaje ya aplicado es `SinCambios` (ni versión, ni hecho, ni dado, ni id).
  - **Rechazos:** `batalla.bloqueado` (§5), `batalla.no_existe` (404), `batalla.ya_asignada`, `batalla.sin_credencial`, `batalla.salida_incompleta` (§8.4) y `batalla.invalida` para el resto (el mensaje dice cuál: revisión vieja, servidor ajeno, lado lleno, «tu Facción no combate aquí», cada punto del checklist de §8.5, «ya se aplicó con otro resultado»…).
  - **El candado de las órdenes (§5)** es `Comando::implicados` (qué héroes, columnas, caravanas, plazas, pueblos y campamentos de bandidos nombra cada orden; el `match` no tiene comodín: una orden nueva obliga a decidirlo) y `Nucleo::rechazo_por_batalla`. La administración no lo lleva (sus órdenes son andamiaje de escenarios). Marchar hacia una plaza asediada se puede. Quien combata (en una columna o dentro de la plaza asediada) no sale del mundo hasta que acaba; `desconectarse` y `conectarse` no llevan candado.
  - **Plazos (§6) como eventos de prioridad 1** (`Carga::Batalla`, `PRIO_BATALLA`; ver [`Reloj_Y_Eventos.md`](Reloj_Y_Eventos.md)): `FinConvocatoria` (30 s exactos), `Vence` (`expira_en`, a `fallida` sin castigo) y `Retira` (`conservarCerradasMinutos` desde `cerrada_en`). Cada cambio de estado sube la generación de la batalla y reprograma; no hay eventos por entidad ociosa. Lo retirado sale del estado y se acumula fuera de él (`tomar_batallas_retiradas`) para el histórico de §3.3.
  - **Aplicar el resultado (§9):** heridos (`derrotado` o, sin él, el bando perdedor), supervivientes y experiencia de escuadra (×1000, la prestada no gana; también la guarnición de serie de un pueblo, que programa su reposición de 6 h), experiencia y monedas del héroe, candados fuera, y según el contexto: **asedio ganado** → `conquistar_plaza` de la 4D (sin Facción atacante no se conquista nada); **mundo abierto** → el bando perdedor entrega la mitad de su carro, repartida por rondas entre las columnas del ganador; **caravana ganada** → su escolta queda a 0 y vuelve al campamento, y `caer_caravana` (compartido con el combate numérico) pierde los carros que dijo Unity (`carros_perdidos`, `"<caravana>:<posición en su lista>"`) o el 70 % de la carga; **campamento ganado** → oro con rendimientos decrecientes y el campamento desaparece; **saqueo ganado** → `saquear` del almacén del pueblo; la chatarra de los caídos de los dos bandos, al carro del ganador o, si defiende y gana una plaza, a su almacén. Todo en una sola mutación tras validar el checklist entero.
  - **Consultas:** `batalla`, `batallas`, `batallas_activas`, `batallas_pendientes` (`GET /pendientes`), `ticket_publicado` (`None` en la convocatoria), `incorporaciones_vigentes`, `batalla_sobre` (la batalla que disputa una plaza, un pueblo o una avanzada: el estado de asedio, 2026-10-22), `token_de_jugador` (§8.7: `NoExiste`, `NoCombate`, `SinToken`, `Token`), `bloqueos`, `heroe_en_batalla`, `uuid_de_batalla` / `id_de_batalla` (el `gameId` del §4.5 es el hash del mundo y la semilla).
  - **// DUDA (decidido al implementar):** (1) se añaden dos hechos que §10 no lista: `batalla.convocatoria_cerrada` (la cita que el contrato necesita para publicar el ticket) y `batalla.retirada`; (2) el candado de §5 no incluye a la administración; (3) `sinCandadosVencidos` no existe: `Vence` es de prioridad 1 y toda orden se aplica con los eventos hasta su instante ya procesados, así que una batalla vencida está cerrada antes de que nada pueda mirarla; (4) «el Rey admite otras Facciones en sus ataques» es política (sistema 5): `rey_admite_otras` es siempre falso; aliada para batalla = alianza o vasallaje del andamiaje `FijarRelacion`; (5) un campamento de bandidos atacado puede seguir acosando caravanas mientras dura su batalla; (6) una columna apartada que pasa hambre sigue pasándola; (7) los objetos del botín no tienen dónde caer (los héroes no tienen inventario: CQ-004), así que `itemInstanceId` solo se compara con los de las batallas que aún se conservan; (8) tras la batalla la columna retoma su plan pero **no** la persecución que llevaba; (9) `UnirseABatalla` devuelve las secuencias de las incorporaciones en `creadas`.
  - **Hecho después en 4H T4** (2026-10-23, nota al final): `HeroSnapshot`, `SquadSnapshot`, `SettlementBattleSnapshot` y carros identificados en el ticket, su envío al servidor de batalla (ticket, incorporaciones, acuses), la proyección de la batalla (ámbito `BATALLA`) y el asedio visible. **Sin hacer** (filas en la tabla de paridad): la experiencia de Facción, la reputación por defender a un vasallo y los contadores de logros (§9, pasos 3, 6 y 7: sistemas 5 y 6).
- **Fuentes:** `session/batallas.ts` (el ciclo y el candado), `session/entradaEnBatalla.ts` (quién entra y a qué bando), `session/salidasDeBatalla.ts`, `session/resultadoBatalla.ts`, `session/comandos/batalla.ts` (los comandos y los mensajes de Conquest), `session/comandos/interaccion.ts` (`atacar`, que abre), `session/comandos/avanzarTick.ts` (vencer y bloquear), `server/rutas/batallas.ts` (las rutas), `server/identidad/servidoresDeBatalla.ts` (la credencial), `contratos/v1/dto.ts` y `contratos.schema.json` (la forma exacta de lo que viaja), `session/canales.ts` y `session/proyecciones/jugador.ts` (lo que se cuenta).
- **Canon:** [`Docs/Game/5_Sistema_Militar_y_Combate.md`](../Game/5_Sistema_Militar_y_Combate.md) 5.10, 5.15 y 5.16.4-5.16.6. **Contrato con Conquest:** [`typescript-final:Docs/Coordinacion/01_Modelo_de_datos_compartido.md`](../Coordinacion/01_Modelo_de_datos_compartido.md) §15-§17 y [`typescript-final:Docs/Coordinacion/02_Contrato_Comunicacion_Servidor_Cliente.md`](../Coordinacion/02_Contrato_Comunicacion_Servidor_Cliente.md) §3.
- **No está aquí** (está en su documento):
  - El combate **con números** (sin servidores de batalla; desde la cuarta ronda ya no por ser bot contra bot) → [`Combate.md`](Combate.md). Cada resolución numérica tiene su apertura equivalente aquí; sus consecuencias comunes (`aplicarConquista`, `desalojarResidentes`, `registrarDerrota`) se describen allí.
  - Las reglas que llevan el héroe a una columna, el Liderazgo y los loadouts → [`Heroe_Y_Tropa.md`](Heroe_Y_Tropa.md); el movimiento y la persecución → [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md).
  - Quién puede ejecutar cada comando (la matriz) y cómo se reparten los canales de eventos y de vista → [`Proyecciones_Y_Autorizacion.md`](Proyecciones_Y_Autorizacion.md).
  - Lo que pasa **dentro** de la partida: es de Conquest.

## 2. Cuándo se abre

No hay un comando para abrir una batalla: la abre `atacar` (contra una columna, una caravana, un campamento de bandidos o una plaza, que es asediarla). Con **`ctx.batallasEnUnity`** (el proceso declaró `SERVIDORES_BATALLA`, §8), `atacar` no resuelve el combate, **sea quien sea quien combata** (en el TypeScript hacía falta `hayHumano`; en Rust ya no, cuarta ronda de Entrar con todo, 2026-10-22): abre una `Batalla` y devuelve `{ battleId }`. Sin servidores se resuelve con números ([`Combate.md`](Combate.md) §2). Los bandidos que acosan a una caravana no pasan por `atacar`: el acoso es un evento del mundo y, si la caravana va adjunta a una columna con algún héroe sano, abre una batalla de caravana en Unity (§4, quinta ronda); en cualquier otro caso se resuelve con números aunque haya servidores.

- Se valida **igual que el combate con números** (`aperturaDeAtaque`: `exigirSano`, `validarAtaqueAColumna`, `validarAlcance`, `plazaAsediable`…). Ni un jugador abre una batalla solo por llegar o alcanzar a alguien (Doc 5.12.3-5.12.4): solo con la orden.
- Un asedio solo lo abre un **ejército**; una columna personal no asedia pero sí se une (Doc 5.15.1b). Una plaza que ya está en una batalla no se asedia otra vez.
- Quien ataca a una Facción de bots combate contra los héroes bot que estén **dentro** de la plaza y su guarnición; si el atacante es el único y nadie defiende, el asedio se juega igual, sin defensores. **Bot contra bot también se juega en Unity** (cuarta ronda): sin humanos no hay tokens, el bot entra en la batalla y Unity la resuelve como pueda (si gana a la guarnición, es cosa de Unity); BronzeAge nunca la resuelve con números mientras haya servidores. Con la partida sin servidores todo va con números.

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
| `iniciadaEn?`, `aplicadaEn?`, `cerradaEn?` | Instantes de mundo. `cerradaEn` es cuándo pasó a `aplicada`, `cancelada` o `fallida`: de ahí corre la retención (§3.3). Sin ella (una batalla guardada antes de existir) cuenta `aplicadaEn` o, en su defecto, `expiraEn`. |
| `asignacion?` | La `BattleServerAssignment` aceptada **más** `servidorId` (quién la registró). Los tokens que se van sumando viven aquí. |
| `resultado?` | El `BattleResult` aplicado, **entero** (idempotencia, §8.6). |

`estado.batallas` guarda las abiertas y las cerradas **recientes** (§3.3).

### 3.3 Retención y histórico

Una batalla cerrada se queda en `estado.batallas` `BATALLA.conservarCerradasMinutos` (§11) de mundo: es lo que hace falta para repetir el `resultado` de una `aplicada` y para el `GET …/ticket` de una cerrada. Cuando `ahora ≥ cerradaEn + retención`, **el último paso del tick** (`retirarCerradas`) la saca del estado y del snapshot.

Lo retirado no se pierde: `RunnerDePartida` lo compara con el estado previo y lo anexa, **antes** de anotar el tick en el diario, a `<gameId>.batallas.jsonl` (`server/historicoDeBatallas.ts`), hermano del snapshot como `.eventos.jsonl`. Cada línea es `{ gameId, archivadaEn (ISO de pared), batalla }` con la `Batalla` entera (ticket, incorporaciones, salidas, asignación y resultado). Un corte entre anexar y anotar repite la línea; la lectura (`leerHistoricoDeBatallas`) descarta las repetidas por `batalla.id` y las líneas a medias. El respaldo de una partida lo copia y restaurarla lo devuelve al punto respaldado; descartar o eliminar la partida lo vacía o lo borra.

Después de retirada, la batalla **no existe** para el contrato: `GET …/ticket`, asignación, inicio, tokens, salida y resultado responden `404`, también un `resultado` idéntico al aplicado.

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
- `recintos`: `recintoId`, `nivel` **físico en pie** y solo las `celdas` **en pie**, en su orden de recorrido. Son las celdas hasta `avance` incluida (`celdas.slice(0, avance + 1)`) al nivel que el recinto tiene en pie; una mejora en curso (que desde el 2026-10-22 lleva su progreso aparte, [`Murallas.md`](Murallas.md) §2) no cambia nada de esto. Un recinto sin celdas en pie no viaja. Las puertas son las celdas de clase `puerta`. *Implementado en Rust (2026-10-22, murallas, tercera tanda):* `Nucleo::recintos_en_pie(plaza)` da `{ id, nivel, celdas }` con esta regla; el ticket del asedio la congela desde 4H T4 (`Ticket.plaza`, `BAN21`).
- `layoutVersion` = `LAYOUT_VERSION`.
- Las calles no van (Unity las toma de `trazadoPorAsentamiento` de la proyección).

## 4. Abrir la batalla

Cada contexto tiene su **apertura** (`Apertura`: contexto, punto, iniciadora, los dos bandos, bloqueo, mapa). Todos los bandos salen de **héroes sanos** y de las escuadras que llevan en ese contexto.

| Contexto | Quién lo abre | Atacante | Defensor | Bloquea | `mapa` |
|---|---|---|---|---|---|
| `campo_abierto` | `atacar` a otra columna (`validarAtaqueAColumna`: no a los tuyos ni a un aliado, misma clase, a ≤ `radioEncuentro`, héroe sano en las dos) | su columna | la columna atacada | las dos columnas | `mapa`, centro = posición del defendido |
| `caravana` | `atacar` a una caravana (no adjunta) | su columna | **sin héroes**: su escolta cedida (`escoltaIds`); sin escolta, una escuadra de **carreteros** sin dueño (`BATALLA.tropaCarreteros`); `faccionId` = la de su plaza de origen (o `null`) | la columna atacante y la caravana | `mapa`, en la caravana |
| `caravana` (acoso) | un campamento de bandidos que cruza el plan de una columna que lleva caravanas adjuntas y **algún héroe sano** (las caravanas son parte del combate: entran todas las que son presa) (evento del mundo, no `atacar`; solo con servidores de batalla) | los **bandidos**: una escuadra de `milicia_lanceros` con las unidades del nivel del campamento, sin dueño, sin héroes ni tokens; `faccionId: null` | los héroes sanos de la columna con lo que llevan en ella; `faccionId` = la de la columna | la columna, todas las caravanas en juego y el campamento | `mapa`, en la columna |
| `campamento_bandidos` | `atacar` a un campamento (un evento PvE) | su columna | una escuadra de `milicia_lanceros` con las unidades del nivel del campamento (`CAMPAMENTOS_BANDIDOS.niveles`), sin dueño; `faccionId: null` | la columna y el campamento | `mapa`, en el campamento |
| `asedio` | `atacar` a una plaza, **un ejército** | su ejército | los **residentes sanos que están dentro** (`heroesQueDefienden`: conectados, ubicación = esa plaza), cada uno con las escuadras de su **loadout activo** que tiene en el campamento y no en la guarnición; más la **guarnición** sin héroe (`escuadrasSinHeroe`); `faccionId` = la de la plaza | el ejército y la plaza | `SettlementBattleSnapshot` (§3.3) |
| `saqueo_pueblo` | `atacar` a un pueblo extractor, **un ejército** (Combate §7.3) | su ejército | la **guarnición de serie** (30 mineros sin dueño) y quien defienda el pueblo; `faccionId` = la de la plaza madre | el ejército y el pueblo | `mapa`, en el pueblo (el trazado del pueblo lo pone Unity) |
| `avanzada` | `atacar` a una avanzada operativa, **un ejército** (Combate §7.4) | su ejército | la guarnición de serie y la de jugadores; `faccionId` = la de la plaza madre | el ejército y la avanzada | `mapa`, en la avanzada (uno por etapa) |

- **Los héroes de una columna** son los sanos de `participantes`, con las escuadras suyas cuyo contenedor es **esa** columna y que tienen efectivos. Los heridos no entran (sus escuadras ni se reservan).
- **El lado defensor de un asedio, un pueblo o una avanzada solo trae de oficio la guarnición** (2026-10-22, tercera ronda): ni los residentes ni los visitantes amigos (misma Facción, aliados, vasallos) que están **dentro** entran solos; se **unen** con `UnirseABatalla` (§7), durante la convocatoria (entran en el ticket) o después (incorporación), con las escuadras que llevan encima (el loadout activo si residen en su plaza, las de su columna si no). Han de estar **dentro** (§7.1: una columna del mapa, a la puerta, no defiende). El combate con números sí los mete de oficio ([`Combate.md`](Combate.md) §3.2).
- **Capacidad** (`capacidadMaxima`, en héroes): `BATALLA.capacidad.asedio` (15) en un asedio y en una avanzada (que se juega como asedio), `.resto` (5) en las demás, también en el saqueo de un pueblo. Las escuadras sin héroe no ocupan plaza. **Al abrir no se recorta**: si la columna lleva más héroes que la capacidad, todos van al ticket (la cola de los que sobran es cosa de Unity); la capacidad solo frena a quien se une.
- **Duración** (`reglas.duracionMaximaSegundos`) = `BATALLA.duracionMinutos.{asedio|resto} × 60`. Si se agota, **gana el defensor**, en cualquier batalla (§8.6).
- **Convocatoria** (solo asedio): `convocatoriaHasta = ahora + BATALLA.convocatoriaSegundos`. Mientras dura (`enConvocatoria`: `ahora < convocatoriaHasta`), quien se une **entra en el ticket** (revisión 0) como uno de los del principio, y el ticket **no se publica ni se asigna** (§8.1, §8.2). La convocatoria acaba en un **evento exacto** a `convocatoriaHasta` (sin tick: son 30 s justos). Los demás contextos no la tienen y salen hacia Unity al abrirse.
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

**Una batalla activa** es la que está `convocando`, `asignada` o `en_curso` **y** dentro de plazo (`ahora < expiraEn`): una vencida deja de bloquear ya, aunque la cierre su evento de vencimiento. Lo mismo con el candado de las escuadras: antes de ejecutar cualquier comando con candado, `sinCandadosVencidos(estado, ahora)` suelta el `reservaBatalla` de las escuadras de las batallas vencidas (sin cerrarlas ni narrar nada: eso es del tick), así que reponer o guarnecer no dependen de que el tick haya pasado.

`bloqueosDe(estado, ahora)` reúne, de las activas: los **héroes que combaten** (`participacionesDe`), los ids de `bloqueo.ejercitoIds`, `caravanaIds`, el `asentamientoId` asediado y el `campamentoId` atacado.

- **Mientras dura**: lo bloqueado **no se simula** (los ejércitos y caravanas bloqueados se apartan, no se mueven, no comen, no son perseguidos; vuelven al final de las listas), y el héroe bloqueado **no sale del mundo** aunque cumpla su espera ([`Reloj_Y_Eventos.md`](Reloj_Y_Eventos.md) §4.1).
- **En un comando** (`conCandadoDeBatalla` envuelve **todo el registro** salvo los comandos de §5.1): se rechaza con **`batalla.bloqueado`** (200, `ok: false`, sin gastar un dado ni un id) si `tocaLoBloqueado(estado, ahora, actor, params)`:

```
tocaLoBloqueado:  (solo mira si hay alguna batalla en estado activo)
  heroeBloqueado(id) = id ∈ bloqueos.heroes  ∨  (su ubicación es una plaza ∈ bloqueos.asentamientos)    ← quien está DENTRO de la plaza asediada tampoco da órdenes
  (cuarta ronda, Rust) si la orden es SALIR del lugar al mapa (salirDeAsentamiento / salirDePueblo / salirDeAvanzada / salirAlMundo), el lugar y el estar dentro NO cuentan: solo choca si el héroe combate
  devuelve verdadero si CUALQUIERA de:
    – el actor, o params.heroeId, está bloqueado
    – alguno de params.{asentamientoId, atacanteId, defensorId, destinoId, origenAsentamientoId, asentamientoAId, asentamientoBId} ∈ bloqueos.asentamientos
    – params.ejercitoId ∈ bloqueos.ejercitos
    – alguno de params.{caravanaId, desdeCaravanaId, haciaCaravanaId} ∈ bloqueos.caravanas
    – params.objetivo = {tipo, id} apunta a un ejercito / caravana / campamento bloqueado
```

Mira los parámetros **por su nombre** (la convención de todo el registro): cubre los comandos que lleguen sin tocarlos uno a uno. **Marchar hacia** una plaza asediada sí se puede (se llega y se espera a la puerta, o se une uno a la batalla).

### 5.1 Los que no llevan candado

> **Implementado en Rust (2026-10-22, entrar con todo, cuarta ronda).** *Quien está dentro de un lugar atacado y no se ha unido a la defensa solo puede salir.* En un asedio, un saqueo de pueblo o un ataque a una avanzada, la orden de salir del lugar al mapa (`SalirDeAsentamiento`, `SalirDePueblo`, `SalirDeAvanzada` y, para el residente de su propia plaza, `SalirAlMundo`: `Comando::es_salida_de_lugar`) se admite aunque el lugar esté bloqueado y el héroe esté dentro: `rechazo_por_batalla` ignora entonces el lugar y el estar dentro y solo rechaza si el héroe combate (`bloqueos.heroes`). Cualquier otra orden de quien está dentro sigue con `batalla.bloqueado`. Quien se unió a la defensa combate y no sale hasta que acaba. Al salir queda fuera, en el mapa, con todo lo suyo (su columna tal cual); si es neutral o enemigo del dueño puede unirse al ataque desde fuera (§7.1). Decisión del autor (quinta ronda, 2026-10-22): `SalirAlMundo` cuenta como salir del lugar (es la única salida del residente de su plaza). Lo que reservó la batalla (la guarnición con dueño) no se puede llevar.

`unirseABatalla` y `cancelarBatalla` (actúan sobre la propia batalla) y **`conectarse` y `desconectarse`**: cerrar el cliente en plena batalla tiene que **anotarse** (el evento de salida se aplica al terminar, D33b) y volver a conectarse tiene que **anular** esa petición. Los mensajes de Conquest y el tick no pasan por el registro de comandos, así que no llevan candado.

## 6. Plazos y vencimientos

`expiraEn` es de **mundo** (no avanza con el proceso caído: una caída no consume tiempo de mundo, ni el margen se gasta).

| Cuándo | `expiraEn` = |
|---|---|
| Al abrir | `convocatoriaHasta` (ahora, si no hay convocatoria) + `BATALLA.plazoAsignacionMinutos` |
| Al asignarla (`asignada`) | ahora + `BATALLA.plazoInicioMinutos` |
| Al empezar (`en_curso`) | ahora + `duracionMaximaSegundos / 60 + BATALLA.margenMinutos` minutos |
| Tras una salida previa al inicio (vuelve a `convocando`) | ahora + `BATALLA.plazoAsignacionMinutos` |

El margen existe para que un retraso entre Unity y el servidor no haga fallar una batalla que ya terminó. (El reloj de mundo se congela mientras el servidor está apagado, así que una caída no consume plazo.)

**`vencerBatallas`** (evento programado a `expiraEn`, prioridad 1 en [`Reloj_Y_Eventos.md`](Reloj_Y_Eventos.md) §2): toda activa con `instante ≥ expiraEn` pasa a **`fallida`** sin castigo: suelta los candados de sus escuadras (`cerrarSinResultado`), nadie gana ni pierde, y se narra `batalla.fallida`. Esto vale desde `convocando`, `asignada` y `en_curso`.

## 7. Unirse

**`unirseABatalla { heroeId, battleId, lado? }`** (comando de jugador, §5.1 sin candado). La batalla tiene que estar activa (si no, `batalla.no_existe`). Se rechaza (`batalla.invalida`) si:

1. el héroe está **herido**;
2. ya está en una batalla (`bloqueos.heroes`);
3. no tiene **columna**, o su columna ya está en una batalla;
4. su columna está a más de `LOGISTICA.radioEncuentro` del `punto` de la batalla (la plaza asediada, en un asedio);
5. `ladoParaUnirse` no le da bando (§7.1), o le toca **defender** un asedio, un pueblo o una avanzada y **no está dentro** (decisión del autor, 2026-10-22);
6. el bando se pasaría de su `capacidadMaxima` (`yaDentro + entran > capacidad`): si está lleno, **no entra**; la cola es solo de los que ya estaban al abrir.

Entra **su columna entera**: todos sus héroes sanos con lo que llevan en ella (`heroesDeColumna`). Si `enConvocatoria`, entran **en el ticket** (como los del principio); si no, cada héroe es una **incorporación** (`secuencia = incorporaciones.length + i + 1`) y **el ticket no cambia ni sube `ticketRevision`** (no invalida la asignación ni los tokens de nadie). Su columna se suma al `bloqueo.ejercitoIds` y sus escuadras reciben el candado. Devuelve `{ secuencias }`; evento `batalla.refuerzos`.

### 7.1 A qué bando (`ladoParaUnirse`)

| Contexto | Regla |
|---|---|
| `campo_abierto`, `columnas: 'ejercitos'` (batalla campal) | **nadie de fuera**: rechaza |
| `campo_abierto`, `columnas: 'solitarios'` (persecución) | solo una **columna personal** (un ejército no se une); **cualquier Facción**, y **`lado` es obligatorio** |
| `campamento_bandidos` (PvE) | **cualquiera**, también sin Facción, **siempre atacante**; pedir `defensor` rechaza (nadie ayuda a los bandidos) |
| `asedio` y `caravana` (y `saqueo_pueblo` y `avanzada`, igual) | hace falta **Facción** (si no, rechaza). **Defensor** si su Facción es la del defensor o **aliada para batalla** (alianza **o** vasallaje activos, en cualquier sentido: `sonAliadasEnBatalla`, distinto de `estanAliadas`); **atacante** si es la del atacante o —en un asedio, un pueblo o una avanzada (2026-10-22, tercera ronda)— cualquier otra que no sea del defensor ni aliada suya (neutrales y enemigos del dueño); en una **caravana** solo si el Rey de la Facción atacante lo admite (`admiteOtrasEnAtaques`), si no rechaza («tu Facción no combate aquí»). Si pide un `lado` distinto del que le toca, rechaza |

`lado_para_unirse` (lo que se proyecta a cada espectador) **no mira si el bando está lleno**: lo rechaza la orden al intentarlo (§7, punto 6). Se calcula al proyectar la batalla y no cambia si el espectador se acerca o se aleja hasta el siguiente cambio de la batalla (decisiones del autor, 2026-10-23).

**Defender exige estar dentro y atacar exige estar fuera** (decisiones del autor, 2026-10-22; [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md) §11.1): para defender un asedio, un pueblo o una avanzada hay que haber entrado en ellos (la columna del visitante, que entró con él, no está en el mapa y es lo que trajo consigo); una columna del mapa a la puerta, aunque sea de la Facción del defensor y esté a ≤ `radioEncuentro`, solo puede atacar: `batalla.invalida`, «para defender hay que estar dentro…». Y al bando atacante solo se une quien está **fuera**, en el mapa: quien está dentro de lo que se ataca (o de cualquier otro lugar) no ataca desde dentro, ni siquiera siendo de la Facción atacante (`batalla.invalida`, «para atacar hay que estar fuera…»). La caravana, la persecución y el campamento de bandidos van por el mapa, no tienen dentro: se unen a ellos los que están en el mapa, y quien está dentro de algo, no. En `combate::union`, `Reparto::donde` (`Fuera`, `Dentro` o `EnOtroLugar`).

**Quién cuenta como aliado para unirse**: el vasallaje también, para que señor y vasallo se defiendan entre sí y ninguno se una al ataque contra el otro (decisión del autor, 2026-10-06).

## 8. Lo que manda Conquest

**Rutas** (`/v1/batallas/*`, sin `/admin` ni `/jugador`), con `Authorization: batalla-servidor <token>`:

| Ruta | Para qué |
|---|---|
| `GET /batallas/pendientes` | Las `convocando` activas y **fuera de convocatoria**: `{ battleId, gameId, ticketRevision }`. Idempotente y sin efectos; el orquestador hace polling. |
| `GET /batallas/:id/ticket` | El ticket vigente, recuperable mientras la batalla siga en el estado (también una cerrada, durante la retención, §3.3); `404` mientras dure la convocatoria. |
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
- **`eliminado`, `abandono`, `desconexion`** solo con la partida **`en_curso`**: se aplican las **bajas** de sus escuadras (`supervivientes + muertos` tiene que cerrar sobre lo autorizado, cada `squadId` suyo y sin repetir, y **la salida nombra todas las escuadras autorizadas del héroe**: si omite alguna se rechaza entera con `batalla.salida_incompleta`, decisión del autor del 2026-10-23) y, si `derrotado`, **queda herido** (`herir`, Doc 5.16.4). **El ticket no cambia.** El resultado final ya no lo repite.
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
   - **Mundo abierto** (columna, caravana o campamento): cada columna del bando que **pierde** entrega **la mitad de su carro** (`trasDerrota`), que se reparte entre las columnas del que gana, lo que quepa (contra una caravana o un campamento esa mitad no se la lleva nadie). Si **gana el atacante**: **caravana** → su escolta queda a 0 y vuelve al campamento de su héroe, según el tipo (A o B, [`Combate.md`](Combate.md) §7.1) la caravana pierde la mitad de sus carros con su animal, o el 70 % de su carga, y lo que quepa va al carro de los atacantes; **campamento** → oro de bandidos para cada héroe atacante (con el decrecimiento por reincidencia, `botinDeBandidos`) al carro de su columna, el campamento desaparece y se agenda su reaparición. El reparto (`repartirBotin`) es por rondas: cada ronda divide lo pendiente a partes iguales entre las columnas con sitio (`capacidadCargaDe − lo que ya lleva`); la que se llena sale de la siguiente; lo que no cabe en ninguna se pierde. Termina en, como mucho, tantas rondas como columnas.
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

> **Implementado en Rust (2026-10-23, 4H T5).** Los hechos de batalla llevan en el payload el `battleId` del contrato y los datos de §10 (`contexto`, las Facciones de los dos bandos, los héroes y la capacidad de cada uno; `ganaElAtacante` al aplicarse); los oyen los héroes de las columnas bloqueadas, las plazas implicadas, **las Facciones de los dos bandos** (la plaza de una batalla de campo abierto no basta para que se entere el otro hogar), quien combate por su `BATALLA` y quien ve la batalla en su `VISTA`. `batalla.abierta`, `.asignada` y `.aplicada`/`.cancelada`/`.fallida` (y sus `evento_pve.`) van además tipados (`BatallaAbierta` con el contexto del ticket, `BatallaAsignada`, `BatallaCerrada` con estado, ganador y razón). `batalla.retirada` la oyen las Facciones de los dos bandos.

> **Implementado en Rust (2026-10-24, 4H T7).** `batalla.abierta` (y `evento_pve.abierta`) lleva además en sus datos contra qué se juega —`plaza` del asedio, `pueblo`, `avanzada` y su `etapa`, `campamento` de bandidos, `solitarios` en campo abierto, `modalidad` (0 disputa, 1 aguante) y `caravanasIds` de una batalla de caravana—, `iniciador` (si lo hay) y `convocatoriaHasta` (un asedio): el tipado `BatallaAbierta` sale de esos datos y se tipa igual aunque la batalla ya se haya retirado cuando se proyecta el hecho (de su contexto, los carros de una caravana y lo protegido de un pueblo se leen del estado de ahora, como en el ticket). Los bandidos no son una Facción: su `faccionAtacanteId` va vacío. La administración ve en `Batalla.asignacion.servidor_id` qué servidor de batalla tiene cada una (el id de su credencial `batalla-servidor:<id>`, que el núcleo guarda en `Asignacion.servidor`); los jugadores y los servidores de batalla no lo reciben.

## 11. Parámetros

> **2026-10-19:** el bloque `BATALLA` es el vigente: `capacidad.asedio` vale **15** (el motor TypeScript tenía 5) y se usa también en la avanzada; el saqueo de un pueblo usa `capacidad.resto`.

<!-- parametros:BATALLA -->
```json
{
  "capacidad": {
    "asedio": 15,
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
  "conservarCerradasMinutos": 60,
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

Los que intervienen y viven en otro documento: `LOGISTICA.radioEncuentro` (distancia de choque y de unión; [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md)), `NIVEL_FACCION.xp`, `REPUTACION.penalizacionAtacarAliado`, `CAMPAMENTOS_BANDIDOS.niveles` y `PRESENCIA` ([`Combate.md`](Combate.md), [`Presencia_Y_Convocatoria.md`](Presencia_Y_Convocatoria.md)).

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
| El candado de una escuadra no sobrevive al plazo de su batalla, aunque el evento aún no la haya cerrado | `batallas.test.ts` |
| Quien puede seguir una batalla: quien combate, o la ve; no un forastero | `canalDeBatalla.test.ts` |

## 13. Casos límite y decisiones que parecen raras

- **Un asedio sin defensores se juega igual**: la batalla se abre aunque el defensor no tenga ni héroes ni guarnición; basta con que haya un atacante (humano o bot).
- **El ticket de un asedio no recorta por capacidad**: el atacante puede llevar más héroes que `capacidadMaxima`; Unity hace la cola.
- **El defensor de un asedio solo trae la guarnición**: los residentes y los amigos (Facción dueña, aliados, vasallos) que están **dentro** se unen ellos; quien no se une no combate (y, si la plaza cae, no pierde nada). Desde **fuera**, a ≤ `radioEncuentro`, se une al ataque la Facción atacante y los **neutrales y enemigos del dueño**; los amigos del dueño no atacan. Un neutral o un enemigo que haya entrado no defiende ni ataca desde dentro (`batalla.invalida`, «hay que estar fuera»).
- **Entrar durante la convocatoria no sube la revisión** y **entrar después tampoco**: la revisión solo sube con una salida previa al inicio.
- **`no_conectado` y `derrotado`**: la primera sale sin castigo y reabre la convocatoria; la segunda hiere. Quien sale antes de empezar no se sustituye.
- **Un `derrotado: false` explícito protege** al perdedor de la herida; quien no lo trae, se hiere con su bando.
- **Un mensaje a una batalla cerrada** responde `404` (asignación, inicio, tokens, salida) o `409` (resultado), salvo el `resultado` idéntico de una `aplicada`, que responde bien. Pasada la retención (§3.3) todo responde `404`.
- **La Facción de una columna sin bandera** es `null` en el ticket y en `faccionAtacanteId`/`faccionDefensoraId`; una conquista con atacante sin Facción **no ocurre**.
- **El plazo vencido suelta todo al instante**, columna y escuadras (`bloqueosDe` mira el plazo; `sinCandadosVencidos` suelta el `reservaBatalla` al validar cada comando). El evento de vencimiento solo cierra la batalla (`fallida`) y la narra. Con el reloj del motor (los comandos corren en el instante del tick y el tick cierra antes) esa ventana no llega a verse; la comprobación perezosa lo garantiza sin depender de él.
- **Una batalla cerrada sale del estado a la hora de la retención** (`BATALLA.conservarCerradasMinutos`) y pasa al histórico de la partida: el estado y el snapshot no crecen con cada batalla (§3.3).
- **El `battleId` solo es único entre partidas si el `gameId` lo es**: sale de un resumen de 64 bits del `gameId` más el contador; no hay aleatoriedad.
- **La salida del mundo espera**: aunque el héroe lleve su `desconectaEn` cumplido, no sale mientras combate; la hace el primer tick tras acabar. Si Conquest lo da por desconectado, lo saca de la batalla con una `SalidaBatalla` (`desconexion`, §8.4) y entonces sale en el tick siguiente.

## 14. Lo que se corrigió al documentar (2026-10-12)

El ciclo se escribió antes de varias cosas que han cambiado (columnas sin bandera, presencia, canales personales). Al contrastarlo con el contrato y el modelo de datos salieron cinco desajustes **del código**, ya corregidos con su test:

1. **Un recién llegado sin Facción que atacaba un campamento de bandidos generaba un ticket inválido.** Su columna lleva la Facción vacía (`''`) y el bando salía con `faccionId: ''`; el contrato exige un id no vacío o `null`. Ahora es `null` (`bandoDeColumna`).
2. **Cerrar el cliente en plena batalla no se anotaba.** `desconectarse` llevaba el candado de batalla y se rechazaba con `batalla.bloqueado`; el tick ya sabía esperar al fin de la batalla (D33b) pero la petición nunca llegaba a registrarse, así que el héroe se quedaba «conectado» para siempre. `conectarse` tampoco podía anular la petición. Ahora los dos están libres de candado.
3. **Los eventos de una batalla no llegaban por el cursor a quien no tiene plaza** (el caso más habitual de quien ataca a los bandidos al llegar): el evento va a la plaza de origen de su columna, que para una salida de campamento es `''`, y no lleva lista de héroes. Ahora el cursor incluye los de las batallas en que combate.
4. **Código muerto**: `faccionesEnBatalla` (la IA de Facciones NPC «no gobierna mientras dura») no la usaba nadie desde que no hay Facciones NPC en el motor; se borra con su test.
5. **El candado de las escuadras de una batalla vencida lo soltaba solo el tick** (el contrato pide comprobar el plazo justo antes de validar la acción). Ahora lo suelta `sinCandadosVencidos` al validar cada comando con candado, sin esperar al tick. Con el reloj actual la ventana no se llegaba a ver, pero un servidor sin tick (la reconstrucción) sí la habría tenido.
6. **`estado.batallas` crecía sin límite** (cada batalla de Unity cerrada se quedaba en el estado y en el snapshot, un asedio con el mapa de la plaza congelado). Ahora se conserva `BATALLA.conservarCerradasMinutos` y después pasa al histórico de la partida (§3.3).

## 15. Divergencias

**Abiertas** (a decidir por el autor, también en [`Divergencias.md`](Divergencias.md)):


**Pendientes de Conquest** (no son divergencias del servidor; el contrato las anota como decididas):

- `atributosEfectivos` sin el equipo y objetos de botín sin tipo ni precio (CQ-004); la XP del héroe se suma pero **el nivel no sube** (CQ-001); no se valida que el objeto exista ni un tope de XP o monedas.

## Modalidades añadidas por la guerra logística (2026-10-13)

El ticket y el contrato de resultado (v1) se amplían, siempre en aditivo, con las batallas nuevas (Doc 5.2, 5.15.1b; detalle de la resolución con números en [`Combate.md`](Combate.md) §7):

- **Asalto de caravana, modalidad `disputa` o `aguante`.** El servidor decide el tipo por la zona donde está la caravana en el instante del ataque y lo manda en `contextoEstrategico.caravana { caravanaId, punto, modalidad }`, con los carros identificados (id y carga de cada uno; en el acoso de bandidos a una columna con varias caravanas adjuntas, la lista de **todas** las caravanas en juego, cada una con sus carros). La respuesta de Unity añade `carrosPerdidos: carroId[]` (en Rust, `"<caravana>:<posición>"` por carro, para poder nombrar los de varias caravanas) (en A y en B); el servidor destruye esos carros con su animal y calcula la carga perdida (A: la de los carros; B: el 70 % de la carga, el 30 % se salva). En A, los carros cambian de manos dentro de la partida y el atacante se lleva el que logre sacar; en B, el defensor aguanta hasta que se agota el tiempo.
- **Saqueo de un pueblo extractor.** Batalla propia; Unity decide el dentro (mapa = el trazado del pueblo). El resultado es solo saqueo, nunca conquista: el pueblo protege el 50 % de su capacidad y el atacante se lleva solo lo que esté por encima, hasta lo que le quepa en los carros.
- **Ataque a una avanzada.** La abre solo un ejército y dura 30 minutos como un asedio; hay un mapa interior por etapa (atalaya, fuerte, ciudadela). Resultado: destruida, en ruinas o resistida según la etapa (Combate §7.4).

> **Implementado en Rust (2026-10-22, fase 4F).** `Nucleo::apertura_de_avanzada` (en `nucleo/batallas.rs`) y `Nucleo::cae_avanzada` (en `nucleo/avanzadas.rs`). **Sin convocatoria** (decisión del autor: `Contexto::tiene_convocatoria` es solo del asedio): la batalla nace `convocando`, pendiente de asignar, con capacidad de asedio (15) y duración de 30 minutos. El defensor es la Facción de la plaza madre, **sin participantes** (nadie está dentro) y con las escuadras de la guarnición de serie (sin dueño) y de jugadores (con su héroe dueño, que quedan con el candado `reserva_batalla`) como escuadras sin héroe; la guarnición de serie ya existe (nace al primer ataque). El bloqueo nombra la avanzada: ninguna orden la toca mientras dura (`batalla.bloqueado`: mejorar, reparar, destruir, elegir tropa, guarnecer, retirar, repostar, depositar, entregar, atacar). Al aplicar el resultado: la experiencia y los supervivientes de la guarnición de serie se vuelcan a su estado (y empieza su reposición gratuita si queda mermada); si gana el atacante, `cae_avanzada` hace lo mismo que con números (saqueo del almacén por rondas entre sus columnas, destruida o en ruinas, la guarnición de jugadores a 0); si gana el defensor, la chatarra va al almacén de la madre. Una avanzada que ya no existe al llegar el resultado (su madre cayó en ruinas durante la batalla) no cambia nada. Unas ruinas, o una operativa sin nadie en pie, no abren batalla: caen sin combate.

> **Implementado en Rust (2026-10-22, entrar con todo, segunda ronda).** Foto `BAN18`. `unirse_a_batalla` calcula `Reparto::donde` de la ubicación del héroe (`Columna` → fuera; la plaza, el pueblo o la avanzada que se disputa → dentro; cualquier otra → en otro lugar). La apertura de un saqueo de pueblo y de un ataque a una avanzada mete de oficio, como participantes defensores, a los **residentes de la madre** que están dentro (sanos y conectados, con las escuadras de su columna); los demás amigos que están dentro se unen. Quien está dentro de un pueblo o una avanzada atacados no da órdenes (`batalla.bloqueado`). Al aplicar el resultado, si gana el atacante, los residentes que defendieron pierden la mitad del carro, que se reparte entre las columnas atacantes (asedio con conquista, saqueo del pueblo y avanzada); la avanzada que cae saca al mapa a todos los de dentro. Pruebas en `crates/nucleo/tests/{batallas,avanzadas}.rs`.

> **Implementado en Rust (2026-10-22, entrar con todo, tercera ronda).** Foto `BAN18` (sin cambios de formato: no hay estado nuevo). **Unirse a la defensa no es automático.** Al abrirse un asedio, un saqueo de pueblo o un ataque a una avanzada entra de oficio solo la guarnición (`apertura_de_asedio`, `_de_pueblo` y `_de_avanzada` ya no meten participantes; `residentes_dentro` desaparece). Quien está **dentro** y es de la Facción dueña, aliado o vasallo —residentes incluidos— puede unirse con `UnirseABatalla`, durante la convocatoria (entra en el ticket) o después (incorporación): no hay hecho de invitación ni aviso dirigido, es una cuestión de **elegibilidad** y el cliente muestra la opción. Se une con lo que lleva encima, un solo participante: el residente en su plaza, las escuadras de su loadout activo que tiene en el campamento; los demás (un visitante, o un residente dentro de un pueblo o una avanzada), las de la columna con la que entraron (`defensor_de_dentro`); no hace falta columna ni distancia (`Batalla::unirse` recibe `Option<ColumnaId>`: sin columna no aparta ninguna) y el rechazo «hay que llegar con tu columna» queda solo para quien está fuera. **Desde fuera, en el mapa, a ≤ `radioEncuentro`, se une al bando atacante la Facción atacante y los neutrales y enemigos del dueño del elemento** (`combate::union`: en un asedio, un pueblo o una avanzada, quien no es amigo del defensor ataca desde fuera; el Rey ya no decide, solo en las caravanas); los amigos del dueño, desde fuera, se rechazan con «para defender hay que estar dentro». Quien no se une no combate y no pierde nada. Que el elemento está en asedio se consulta con `Nucleo::batalla_sobre(Ubicacion)` (su proyección al contrato es la 4H). (La cuarta ronda sustituye la apertura condicionada a un humano: con servidores toda batalla va a Unity.) La vía numérica (sin servidores) no cambia (defienden todos los amigos de dentro). Al aplicar el resultado, **todos** los defensores que perdieron pierden la mitad del carro. Pruebas en `crates/nucleo/tests/{batallas,avanzadas}.rs` y `crates/combate/src/union.rs`.

> **Implementado en Rust (2026-10-22, entrar con todo).** Foto `BAN17`. Sin cambios en el ciclo: `unirse_a_batalla` ya pedía estar dentro y `combate::union` ya dejaba defender solo a la Facción del defensor y a las aliadas para batalla (alianza o vasallaje). Lo que cambia es el modelo de «dentro»: la columna del visitante **entra con él** (ya no «aparcada a la puerta») y la puerta de la plaza la abre el dueño por relación (`FijarAcceso`, [`Presencia_Y_Convocatoria.md`](Presencia_Y_Convocatoria.md)); un neutral o un enemigo que haya entrado no se une a defender (prueba en `crates/nucleo/tests/batallas.rs`). Si la plaza cae, los de dentro salen al mapa con su columna sin perder nada.

> **Implementado en Rust (2026-10-22, decisiones de 4F).** Foto `BAN16`. **Unirse a defender exige estar dentro** (§7.1): `combate::union::Reparto` gana `dentro` y `ErrorUnion::HayQueEstarDentro`; `unirse_a_batalla` lo calcula (la ubicación del héroe es esa plaza en un asedio; falso siempre en un pueblo y en una avanzada). Una columna del mapa a la puerta de la plaza, el pueblo o la avanzada ya no se une a la defensa (`batalla.invalida`); sí al ataque. La apertura (`apertura_de_asedio`, `_de_pueblo`, `_de_avanzada`) ya solo metía lo de dentro.

> **Implementado en Rust (2026-10-22, cuarta ronda).** Foto `BAN18` (sin estado nuevo). **Con servidores de batalla toda batalla va a Unity, también bot contra bot; sin ellos, todo con números.** Se retira `Apertura::hay_humano` y `humano_por_defender` (y la regla «un héroe humano en los dos bandos» de la 4E, con su `// DUDA` de la tercera ronda): `atacar` abre la batalla siempre que `Reglas::batallas_en_unity`, en los cinco contextos (columna, caravana, campamento de bandidos, asedio, pueblo y avanzada). Una batalla sin humanos no lleva tokens (`RegistrarAsignacion` con la lista vacía); `no_conectado` no le aplica (los bots no se conectan) y, si llegara, cancelaría la batalla porque no queda ningún humano (§8.4). Si nadie la asigna, falla por plazo sin castigo, como cualquier otra. Salir mientras dura: ver §5.1. Pruebas en `crates/nucleo/tests/{batallas,avanzadas}.rs`.

> **Implementado en Rust (2026-10-22, quinta ronda).** Foto `BAN18` en esa ronda (las decisiones del autor del mismo día, más abajo, la subieron a `BAN19`). **Los bandidos que acosan una caravana con héroes juegan en Unity.** `bandidos.rs::acosar` abre una batalla de caravana (`Nucleo::apertura_de_acoso`, modalidad `disputa`, atacante = la escuadra de bandidos con un id de la partida, defensor = los héroes sanos de la columna a la que va adjunta la caravana) cuando `Reglas::batallas_en_unity` y la columna lleva algún héroe sano; sin héroes, o sin servidores, tira con números como siempre. `iniciada_por` es 0 (`SIN_HEROE`): nadie la inicia y nadie la cancela. Bloquea la columna, la caravana y el campamento. Las consecuencias son las del mundo abierto (`consecuencias_de_batalla`): ganan los bandidos → los carros que dice Unity, la carga no se la lleva nadie y la columna derrotada pierde la mitad de su carro; ganan los héroes → nada más. Un campamento no vuelve a acosar a la columna mientras se conserve esa batalla (`acoso_ya_jugado`). Pruebas en `crates/nucleo/tests/batallas.rs`. // DUDA: el prefijo es `batalla.` y no `evento_pve.`; el bando atacante sin héroes ni tokens y la batalla sin iniciador son del contrato (4H).

> **Implementado en Rust (2026-10-22, decisiones del autor sobre la quinta ronda).** Foto **`BAN19`** (el contexto de caravana del ticket es una lista; `BAN18` se rechaza con su motivo). **Varias caravanas adjuntas: entran todas en la batalla de acoso** (decisión del autor; ya no es `// DUDA`). `Nucleo::apertura_de_acoso(t, campamento, columna, punto)` pone en el contexto **todas** las caravanas adjuntas que son presa (`es_presa_de_bandidos`: en el mapa, fuera de toda zona y de la protección de un campamento de mercenarios) y en `bloqueo.caravanas` a todas ellas; `bandidos.rs::acosar_columna` abre una sola batalla por columna y cruce (sin servidores o sin héroes sanos, cada caravana adjunta tiene su tirada con números). `Contexto::Caravana { caravanas: Vec<CaravanaEnBatalla { id, carros }>, punto, modalidad }` (`combate::ciclo`; `Contexto` pierde `Copy`). **El resultado nombra los carros perdidos como `"<caravana>:<posición>"`** (`Resultado::carros_perdidos`, `Efectos::carros_perdidos` = pares (caravana, posición)): `validar_resultado` rechaza entero un carro mal escrito, repetido, de una caravana que no está en la batalla o que la caravana no tiene (la posición sola, el formato de antes, ya no vale). **Consecuencias** (`captura_de_batalla`, por caravana en juego): si ganan los bandidos, cada una pierde los carros que dice Unity (con su animal y su carga; desaparece si se queda sin ninguno) y su escolta cedida queda a 0 y vuelve al campamento de su héroe, y la columna derrotada pierde la mitad de su carro (decisión del autor: es lo que ya hacía el mundo abierto); la carga perdida no se la lleva nadie. Si ganan los héroes, nada más. «Escoltada» es «con algún héroe sano a bordo» (decisión del autor). La batalla de caravana que ataca un ejército sigue teniendo una sola caravana. Pruebas: `crates/nucleo/tests/batallas.rs` (dos caravanas adjuntas, ambas bloqueadas, consecuencias en las dos según el resultado, foto y determinismo normal/acelerado/a trozos) y `crates/combate/src/ciclo/pruebas.rs`. // DUDA: del contrato (4H) falta mapear al Protobuf la lista de caravanas del contexto y el `string` de cada carro perdido con su caravana.

> **Implementado en Rust (2026-10-23, 4H T4: la batalla de Unity de extremo a extremo por el contrato).** Foto **`BAN21`** (el ticket congela la plaza de un asedio y el nivel, la experiencia y la moral de cada escuadra; el informe de inspección de una plaza trae sus recintos; `BAN20` se rechaza con su motivo). Sin cambio de esquema: el contrato de T1 ya traía todo. Pruebas por el servidor WebSocket real con servidores de batalla de prueba en `crates/nucleo/tests/batalla_unity.rs`.
> - **El ticket completo** (`proyeccion::batalla::ticket`). `combate::ciclo::Ticket.plaza` (`PlazaCongelada`) guarda al abrir un asedio los edificios internos (id, tipo, centro de la huella en milésimas de unidad local, huella en celdas con la rotación aplicada, nivel interno, estado, dañado) y los recintos con solo las celdas en pie y su nivel (`Nucleo::recintos_en_pie`), con `layoutVersion` (`trazado::LAYOUT_VERSION`) y `unidadesPorCelda` (`trazado::CELDA_UL`); lo construido o mejorado después no la cambia. `EscuadraEnBatalla` lleva nivel, experiencia y moral del instante de abrir (`SquadSnapshot`; una escuadra sin dueño, nivel 1, sin experiencia y moral al máximo); el `HeroSnapshot` se lee del héroe: nombre «Héroe N», clase `default`, sin género, avatar ni equipo (decisión 4), atributos efectivos = base, `casillasInventarioLibres` del ticket. Los carros de una caravana se nombran `"<caravana>:<posición>"` con la carga que les toca por capacidad (`Caravana::carga_por_carro`), y el contexto de caravana lleva la lista de las que se juegan. El resto del terreno es `BattleMapReference` (hash del mundo).
> - **Ámbito `BATALLA:<uuid>`.** Derecho de quien combate (`heroes_que_combaten`, el héroe de la sesión o el de campo de su cuenta) mientras el núcleo conserve la batalla (así ve el resultado); `Revocado` al salir (`sin_derecho`) o al retirarse (`ya_no_existe`). La `Batalla` ya no va a `HEROE`: va a `BATALLA` y a `FACCION` de las Facciones de los dos bandos y de quienes combaten. Quien combate la ve entera (ticket, incorporaciones, resultado, `lado_propio`, `asignacion_propia` con **su** token, `Nucleo::token_de_jugador`); el resto la ve resumida con `lado_para_unirse` (`Nucleo::lado_para_unirse`, que comparte con `UnirseABatalla` las condiciones de §7 en `elegibilidad`). Un espectador de otra Facción la verá por la `VISTA` con niebla (T3d).
> - **Publicación a los servidores de batalla** (`sesiones.rs`). El ticket sale a **todas** las sesiones de servidor de batalla conectadas cuando la batalla queda pendiente de asignar con una revisión que no se mandó(al abrirse, al acabar la convocatoria de un asedio y tras una salida `no_conectado`, con la revisión subida); las incorporaciones, a todos mientras nadie la tiene y solo al servidor de la asignación después; un servidor que llega recibe las pendientes y las suyas. **Gana la primera asignación** (decisión 7); la otra recibe `batalla.ya_asignada`.
> - **Acuses.** Cada mensaje del servidor de batalla recibe un `AcuseBatalla` (`mensaje`, `battle_id`, `resultado` y `rechazo`) en lugar del `Aviso` de antes, después de lo que el mensaje provocó. Los tokens entrantes (`tokens`, orden 103) ya se traducen; el mensaje saliente `tokens` no se usa.
> - **Batallas sin humanos y sin iniciador.** Se publican y se asignan como las demás (el bando de los bandidos: sin Facción, sin participantes, una escuadra sin héroe; `iniciada_por` vacío en la `Batalla`), con la lista de tokens vacía.
> - **Asedio visible** (`batalla_id`). `Asentamiento`, `Pueblo` y `Avanzada` lo llevan mientras la batalla está activa (`Nucleo::batalla_sobre`); el núcleo marca el lugar al abrir y al cerrar la batalla (`tocar_lo_disputado`).
> - **Informe de inspección.** `InformeInspeccion` en `HEROE` del que inspeccionó (uno por objetivo, hasta su `expira_en_ms`); `militar::inspeccion::Contenido::Plaza.recintos` guarda los recintos con alguna celda levantada con el anillo entero y las celdas levantadas.
> - **// DUDA (decidido al implementar):** ver las filas «Contrato (4H, T4)» de `Docs/Mecanicas a desarrollar.md` (espectadores sin Facción en juego, `lado_para_unirse` al proyectar y sin mirar el bando lleno, valores por defecto del héroe, experiencia y moral en puntos, carga proporcional por carro, incorporaciones solo al servidor de la asignación, recálculo de derechos en cada versión que toca una batalla).

- **Asedio de 15 contra 15** (`BATALLA.capacidad.asedio`).
- Toda agresión entre Facciones exige guerra o la declara; unirse como atacante a un asedio ajeno no la declara.
- No se envía nada de esto a Conquest sin permiso del usuario.

## Catálogo de eventos

Los códigos de los hechos de dominio de este documento (4H T5, 2026-10-23; contrato 05 §8). La prueba `crates/nucleo/tests/catalogo.rs` lee el bloque de cada documento de `Docs/Mechanics` y exige que todo código que emite el núcleo esté en alguno, y que los que el núcleo aún no emite figuren como paridad pendiente en `Docs/Mecanicas a desarrollar.md`. El núcleo los emite todos.

<!-- eventos -->
```json
[
  "batalla.abierta", "batalla.aplicada", "batalla.asignada", "batalla.cancelada", "batalla.convocatoria_cerrada", "batalla.en_curso", "batalla.fallida",
  "batalla.refuerzos", "batalla.retirada", "batalla.revisada", "batalla.salida", "evento_pve.abierta", "evento_pve.aplicada", "evento_pve.asignada",
  "evento_pve.cancelada", "evento_pve.en_curso", "evento_pve.fallida", "evento_pve.refuerzos", "evento_pve.revisada", "evento_pve.salida"
]
```
<!-- /eventos -->
