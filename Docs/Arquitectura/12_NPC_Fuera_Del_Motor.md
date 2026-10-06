# NPC fuera del motor: el bot como cliente

Plan para sacar **todo el comportamiento NPC** (gobernanza incluida) del motor y del tick, y que los bots jueguen
**desde fuera, como un jugador más**: ven lo que ve un jugador y actúan con los comandos de un jugador.

Nace de dos decisiones del usuario (`Consideraciones/Campamentos_Entrada_Fundacion_Definicion.md`):

- **D52**: el NPC sale del motor porque ya ocupa demasiado del tick, y la lógica nueva de bots-héroe lo empeoraría.
- **D53-D59**: los bots-héroe nacen en los campamentos y siguen el mismo flujo que un humano (sesiones, llegada
  escalonada, tres perfiles), con esa lógica fuera del motor.

> **Estado (2026-10-04):** decidido el bot como cliente (§3), auto-comercio, ritmo y alta de bots (§8) y el orden
> de trabajo (§9). **Hechos los pasos 1 (diario, §5.1), 2 (runner de bots, §10.3), 3 (presencia e identidad,
> §11) y 4 (cerebro «sin plaza» y batch nuevo, §12) y el adaptador remoto (§13).** Queda desplegarlo y medir con batch.

## 1. Cómo es hoy (medido en el código)

- `RunnerDePartida.unTickCompleto` (`server/runnerDePartida.ts`) encadena **tick → auto-comercio → turno NPC** en
  **una sola entrada** de la cola serial. Mientras el NPC piensa, ningún comando de jugador entra.
- El NPC (`session/npcGobernanza.ts`, ~2400 líneas) **no usa comandos**: recibe el estado entero, sin niebla, y llama
  a funciones del motor directamente. Además hace cosas que ningún jugador puede hacer: crear héroes, anexionar
  Facciones, fundar a pie.
- ~~**Cada comando persiste la partida entera**~~ (resuelto con el diario, §5.1): `aplicarYPersistir` hacía
  `exportar()` + `guardarPartida` por comando. Con cientos de bots, eso pesaría más que el propio NPC.
- Los héroes bot tienen `jugadorId = null`: no tienen identidad con la que pasar por `autorizacion.ts`.
- Los comandos ya son deterministas dado el estado: `ContextoComando` trae `instante` (derivado del tick), `rng` e
  `ids` de la sesión; ni el motor ni la sesión leen `Date.now()` ni `Math.random()`. Es lo que permite el diario
  del §5.

## 2. Principio

**Un bot no puede hacer nada que un jugador no pueda hacer, ni saber nada que un jugador no sepa.**

- **Ve** por la proyección del jugador (`session/proyecciones/jugador.ts`), con su niebla.
- **Actúa** con los comandos de `session/comandos/`, con su autorización, con su héroe como actor.
- **Lo que solo existía para el NPC desaparece** (§7).

## 3. El runner de bots

Un **proceso aparte** que maneja a todos los bots. Cada bot tiene cinco piezas:

1. **Sesión.** Si está conectado o no, según un horario sacado de la semilla (D55: unas horas al día, en uno o dos
   bloques). Al conectar manda el comando de entrar al mundo y al desconectar el de salir (presencia, D33).
2. **Percepción.** La proyección del jugador: lo que ve ahora más lo que su Facción recuerda (niebla, campamentos,
   caminos). El motor ya guarda esa memoria por Facción; el bot no reconstruye el mapa.
3. **Memoria propia.** Lo que la proyección no trae: el plan en curso («voy a por el bandido X»), los intentos
   fallidos y su motivo, esperas, el objetivo de la Facción. Es pequeña y **desechable**: si el runner se reinicia,
   el bot rehace su plan a partir de lo que ve en el mundo (si su columna marcha hacia X, va hacia X).
4. **Cerebro.** Decide según la fase del bot:
   - **Sin plaza**: cazar bandidos, comprar en el mercado, aportar al fondo, comprar la caravana, conducirla, fundar.
   - **Con plaza**: lo que hoy hace la gobernanza (construir, reclutar, guarnecer, comerciar, expandirse), **solo
     si el bot tiene el cargo** que lo permite. El Gobernador bot gobierna con los comandos de Gobernador; el Rey
     bot acepta solicitudes.
   - **El perfil** (amigos, solitario, tardío; D57) cambia las prioridades, no las reglas.
5. **Acción.** Manda comandos y lee la respuesta. Un rechazo es información («el mercado ya no tiene madera») que
   va a la memoria.

**Pizarra de Facción.** Los bots de una misma Facción comparten una pizarra en el runner: quién lleva qué, cómo va
el fondo, cuándo sale la caravana. Es el equivalente al chat de voz entre amigos humanos: no da información que la
Facción no tenga.

**Ritmo.** Un bot no piensa en cada tick: piensa cada *k* ticks, con un desfase propio. Un humano tampoco actúa cada
minuto, y así la carga se reparte (300 bots, *k* = 5 → unos 60 por minuto).

**Coste de la percepción.** La proyección con niebla es lo más caro. Palancas, por orden:

1. **Una proyección por Facción**, compartida por sus bots (ven lo mismo).
2. Más adelante, **deltas por el canal de tiempo real** en vez de pedir la proyección entera.

### 3.1 Ejemplo: la primera hora de un grupo de amigos

1. Llegan tres bots juntos (D56) y eligen el campamento con menos residentes.
2. Uno manda `crearFaccion`, los otros dos `solicitarIngreso`, y el Rey bot los acepta.
3. Cada uno pide su tropa prestada, carga la ración y sale a un bandido de nivel 1 del anillo. Se lo reparten por
   la pizarra para no ir los tres al mismo.
4. Vuelven, repostan y repiten. Cuando el oro alcanza, uno compra madera y otro aporta al fondo.

## 4. Un puerto, dos adaptadores

El cerebro solo conoce un puerto: **observar** (proyección) y **actuar** (comando → resultado).

| Adaptador | Dónde | Para qué |
|---|---|---|
| **En proceso** | Llama a `GameSession` directamente. Tras cada tick, los bots piensan **en orden de id**, cada uno con su RNG derivado de la semilla | **Batch** (misma semilla → mismo resultado) y tests |
| **Remoto** | HTTP (comandos) + tiempo real (eventos), como BronzeAgeClient, con cuentas de bot | **Servidor**: proceso aparte (en Render, un *Background Worker*) |

El cerebro es el mismo en los dos. Si un proceso de bots no basta, se reparten los bots entre varios sin tocar el
cerebro.

### 4.1 Opciones descartadas por ahora

- **Kubernetes.** Orquesta muchos procesos en muchas máquinas. Con un servidor y un runner de bots en Render solo
  añade un clúster que mantener. Tendría sentido con muchas partidas a la vez o con muchas instancias de bots; el
  adaptador remoto ya permite ese salto sin rehacer nada.
- **`worker_threads` en el mismo proceso.** Saca la CPU del hilo principal, pero el NPC seguiría viendo el estado
  entero (copiarlo cada tick es caro) y no le obliga a jugar como un jugador. Vale como paso intermedio, no como
  destino.
- **Cola de mensajes (Redis/NATS).** HTTP y tiempo real ya existen; no hace falta otro canal.

## 5. Persistencia: guardado por tick + diario de comandos (decidido 2026-10-04)

Hoy cada comando guarda la partida entera. Con bots no escala. **Decisión del usuario: opción (b).**

- **Diario**: cada comando aceptado (y cada tick) se **añade** a un archivo de diario en el orden en que salió de
  la cola: manejador, parámetros, actor. Es una escritura de una línea, barata.
- **Guardado**: la partida entera se guarda **una vez por tick**, y el diario empieza de nuevo tras cada guardado.
- **Recuperación**: al arrancar, se carga el último guardado y se **repasa el diario** en orden. **No se pierde
  ningún comando aceptado.**
- **Lo que exige**: que repetir un comando sobre el mismo estado dé exactamente el mismo resultado. Ya es así por
  diseño (§1, último punto), pero hay que **probarlo**: un test que ejecute una partida, la reconstruya desde guardado
  + diario y compare las dos bit a bit.
- **Rechazados**: no van al diario (no cambiaron nada), como hoy no se persisten.
- **Eventos**: el JSONL de eventos (`eventosDePartida.ts`) sigue siendo historial derivado; el diario es otra cosa,
  la fuente para reconstruir el estado.

Beneficia también a los humanos: un comando deja de costar un guardado completo.

### 5.1 Diseño en detalle (implementado 2026-10-04)

Hoy hay **tres puertas** que mutan una partida, todas por `RunnerDePartida`: `ejecutar` desde
`rutas/comandos.ts` (comandos de jugador/admin), `ejecutar` desde `rutas/batallas.ts` (los 4 mensajes del
servidor de batalla) y `avanzarTick` (reloj de mundo y `POST .../tick` de admin). El diario se engancha ahí y en
ningún otro sitio.

**Archivo y línea.** `<gameId>.diario.jsonl`, hermano de `.json` y `.eventos.jsonl`, en el mismo
`AlmacenDeObjetos`. Una línea por mutación que **subió la versión**:

```json
{"v":1234,"t":"marcharA","a":"heroe-17","p":{"destino":{"x":40,"y":12}}}
```

- `v`: versión **resultante**. Sirve para saltar lo que ya está en el guardado y para verificar el repaso.
- `t`: nombre en un registro único de lo que puede aparecer en el diario: `REGISTRO_COMANDOS` + los 4 manejadores
  del servidor de batalla + las operaciones del sistema (`avanzarTick`, y mientras existan `avanzarAutoComercio`
  y `avanzarFaccionesNpc`). `RunnerDePartida.ejecutar` pasa a recibir el **nombre**, no la función.
- `a`: actor tal cual llegó a `GameSession.ejecutar`. `p`: params (se omite si es `undefined`).

**Invariante nuevo en `GameSession.ejecutar`: lo que no sube la versión no deja rastro.** Si el resultado no
cambió la versión (rechazo o `sinCambios`), se restauran el RNG y el generador de ids a como estaban. Sin esto, un
comando rechazado que hubiera tirado un dado antes de rechazar movería el RNG sin dejar línea, y el repaso
divergiría en silencio. Es lo que permite que el diario solo lleve lo aceptado.

**Ciclo de un comando** (sustituye a `aplicarYPersistir`):

1. Aplicar. Si la versión no cambió → responder; no hay nada que escribir (como hoy).
2. **Anexar la línea al diario.** Si falla → revertir a `previo` y lanzar (mismo contrato que hoy con el
   guardado: aceptado = durable).
3. Anexar sus eventos al JSONL de eventos (como hoy, best effort). Se mantiene por comando y no por tick porque la
   memoria guarda solo los últimos 5 000 eventos: con cientos de bots, un tick podría generar más y el tramo se
   perdería del historial.

**Ciclo de un tick:** el tick completo (tick + auto-comercio + NPC mientras existan) anexa sus 1-3 líneas en
**una** escritura, después `guardarPartida`, y si el guardado fue bien, **vacía el diario**
(`almacen.escribir(clave, '')`). Si el guardado falla, se grita y no se revierte nada: el tick ya es durable por
el diario, y el siguiente tick reintenta el guardado (el diario crece mientras tanto).

**Truncado sin carrera.** Un corte entre «guardar» y «vaciar» deja en el diario líneas que el guardado ya
contiene; al recuperar se saltan las de `v ≤ versión del guardado`. Por eso no hace falta que guardar y vaciar
sean atómicos juntos.

**Recuperación** (`cargarOCrear` cuando hay guardado):

1. `cargarPartida` como hoy (guardado + eventos `≤ versión`).
2. Leer el diario y repasar en orden las líneas con `v >` versión del guardado: `ejecutar(REGISTRO[t], p,
   {actor: a})`. Cada una **debe** salir `ok` con versión exactamente `v`; si no, la partida **no arranca** (error
   fuerte: es no-determinismo o un diario corrupto, y servir otro mundo sería peor). Una última línea ilegible
   (corte a mitad de escritura) se descarta, como en los otros JSONL; una ilegible en medio es error.
3. Cursor del JSONL de eventos = la última versión que ya tiene el archivo (no la del guardado): el repaso
   regenera en memoria los mismos eventos y no se vuelven a anexar los que ya estaban.

Todo esto vive en `cargarPartida` (`server/persistenciaPartida.ts` → `repasarDiario` de
`server/diarioDePartida.ts`), así que cualquier carga —el runner, la verificación de un respaldo, un test— ve la
partida tal y como estaba tras el último comando aceptado. No se guarda al acabar de repasar: lo hace el primer
tick.

**Bordes que tocan otros módulos:**

- `crearYPersistir` (y por tanto `descartarYCrear`) **vacía el diario**: si no, las líneas de la partida
  descartada se repasarían sobre la nueva.
- **Apagado limpio** (`RegistroDePartidas.cerrar`, SIGTERM de un despliegue): guardar y vaciar. Así un
  despliegue normal arranca con el diario vacío.
- **Respaldos** (`respaldos.ts`): el diario viaja como hermano del snapshot, igual que eventos y auditoría, así
  que un respaldo incluye lo aceptado desde el último tick sin forzar un guardado. Restaurar pone el diario del
  respaldo o **borra el vigente** (sus líneas son de la partida viva: repasadas sobre el respaldo, la harían
  avanzar hacia donde estaba).
- **Opciones del proceso** (`OpcionesSesion.batallasEnUnity`, de `SERVIDORES_BATALLA`): cambian el resultado de
  algunos comandos. Si cambian entre una caída y el arranque, el repaso podría divergir sin que la versión lo
  delate. Techo aceptado: con el apagado limpio, solo afecta a una caída dura seguida de un cambio de config.
- **libSQL**: `anexar` es `contenido || nuevo`, que reescribe la fila; con el diario vaciado cada tick, la fila
  nunca pasa de un tick de comandos.

**Coste.** Por comando: un append de ~200 bytes en vez de leer el snapshot entero (la comprobación de
concurrencia) y reescribirlo. Por tick: el guardado de hoy, una vez.

**Test de reconstrucción** (`src/server/__tests__/diarioDePartida.test.ts`), almacén en disco temporal. El guion
incluye una Facción NPC, cuyo turno tira dados en cada tick. Verificado también en vivo: servidor real, comandos
por HTTP, proceso matado con `taskkill /F` (snapshot en v3, diario hasta v5) → al reabrir, v5 con todo.

1. Partida con semilla fija; un guion intercalado de comandos de varios héroes y ticks (incluido un rechazo).
   «Caída»: se abandona el runner sin apagado limpio y se abre otro con `cargarOCrear` sobre el mismo almacén.
   El snapshot serializado (la misma forma que escribe `guardarPartida`) y el estado del RNG/ids deben ser
   **idénticos byte a byte** a los del runner original, y la partida debe seguir igual con los mismos comandos
   siguientes.
2. Caída entre guardar y vaciar: líneas viejas en el diario → se saltan, mismo resultado.
3. Última línea a medias → se descarta; línea con `v` que no cuadra → la carga falla.
4. Un rechazo que consume RNG no desincroniza (cubre el invariante nuevo).

## 6. Lo que necesita el motor y la sesión

1. **Presencia (D33)**: comandos de entrar y salir del mundo; el héroe y su tropa salen y vuelven; las unidades
   prestadas (escolta, guarnición) se quedan (D40b).
2. **Identidad de bot**: cuenta y membresía para los bots, para que pasen por `autorizacion.ts` como cualquiera.
3. **Comandos que falten**: inventario de cada acción de `npcGobernanza` (§7); si un jugador puede hacerlo y no hay
   comando, se crea.
4. **El tick deja de incluir al NPC**: `unTickCompleto` se queda en tick + auto-comercio (si sigue).

## 7. Inventario: qué pasa con cada cosa del NPC actual

- **Lo que un jugador también hace** (construir, reclutar, guarnecer, comerciar, marchar, atacar): se hace con el
  comando del jugador. Si falta, se crea.
- **Lo que solo existe para el NPC: desaparece.**
  - Poderes que ningún jugador tiene: crear héroes, fundar a pie (D19, D53).
  - Reglas especiales de `acogerHeroesNpc`: anexionar la Facción NPC derrotada a la ganadora, disolverla sin ganador
    (D59) y mudar al bot sin casa. Esto último ya lo cubre para todos el motor desde `9f9a098` (quien pierde la casa
    va al campamento más cercano en el momento del hecho). Un bot derrotado hace lo que haría un jugador: vuelve a un
    campamento y, si quiere, solicita entrar en otra Facción.
- **Estructura que se borra (D58, preliminar)**: `crearFaccionNpc` (comando y ruta de admin),
  `fundarAsentamientosIniciales`, `materializarFundadoresNpc`, `heroeBot` en su forma actual,
  `buscarPosicionFundacionInicialPorDefecto` y `MINERALES_BONUS_FUNDACION` (si nadie más los usa), la fundación
  inicial de `scripts/run-batch-sim.ts` (`elegirPosicionesFundacion`, `NUM_FACCIONES`), `avanzarFaccionesNpc` y su
  llamada en el tick, y `faccionesNpcIds` si todas las Facciones de bots juegan por el puerto. A confirmar con un
  barrido de usos al implementar.

## 8. Decidido el 2026-10-04

1. **Auto-comercio: sale con el NPC y su módulo se borra.** `engine/simulacionAutoComercio.ts` es «solo para
   simulación» (apagado por defecto, `SIMULACION_AUTO_COMERCIO.activo = 0`) y hace el trabajo de un Tesorero:
   proponer y aceptar trueques. **Encendido, `avanzarAutoComercio` comerciaría también por las Facciones humanas**
   (recibe el estado entero; solo la llamada desde la gobernanza filtra a las NPC). En el modelo nuevo, el Tesorero
   bot propone y acepta trueques con los comandos de un Tesorero, y el módulo, su constante y su entrada en el tick
   desaparecen (§7).
2. **Ritmo del bot (placeholder, a probar con batch)**: piensa **cada 5 ticks** (5 min de mundo), con un desfase
   propio sacado de su id, y **además se despierta** con los eventos que le tocan: su columna llega, termina un
   combate suyo, le rechazan un comando, recibe una solicitud de ingreso (si es Rey). Con eso un bot sin plaza no
   pierde 5 minutos al llegar a un bandido, y la gobernanza (obras de horas) no gasta de más.
3. **Alta de bots: por el camino de los humanos**, en dos modos:
   - **Automático (ahora)**: el runner de bots crea sus cuentas con el mismo registro que un humano
     (`POST /v1/registro` con proveedor `clave`), entra y llama `crearHeroe` en un campamento. Siguiendo D56, va dando
     de alta bots a lo largo de los días.
   - **Manual (más adelante)**: un humano se conecta al runner de bots y los crea desde allí (cuántos, perfil,
     campamento).
   Para poder retirarlos sin fricción (D54), el servidor debe **saber qué cuentas son de bot** sin darles ningún
   poder extra. Propuesta: un código de registro propio para bots (`CODIGO_REGISTRO_BOTS`, junto al
   `CODIGO_REGISTRO` humano) que marca la cuenta como bot. **Confirmado por el usuario (2026-10-04).**

## 9. Orden de trabajo (confirmado 2026-10-04)

1. **Diario de comandos** (§5.1). Vale ya para humanos.
2. **Puerto + adaptador en proceso**, con la gobernanza actual pasada a comandos (inventario acción por acción).
3. **Presencia e identidad de bot** (D33, §8.3).
4. **Cerebro «sin plaza»** de los bots-héroe sobre los campamentos (depende del diseño de campamentos).

## 10. Paso 2: la gobernanza acción por acción (inventario 2026-10-04)

Medido sobre `session/npcGobernanza.ts` (2358 líneas). Hoy la gobernanza piensa **por plaza** y escribe el estado
directamente; como jugador, cada acción la hace **un héroe** con el cargo que la autoriza (`autorizacion.ts`). La
columna «ve» dice si la información que usa la decisión está en la proyección del héroe que actúa (recordando que
un jugador solo ve el interior de la plaza donde está, Doc 1.10.1).

| # | Acción de hoy | Comando | Quién | ¿Ve lo que necesita? |
|---|---|---|---|---|
| 1 | Gobernador y Tesorero al primer fundador (`asegurarGobernanzaBase`) | `asignarCargoLocal` | Rey (gobernador); Gobernador (resto) | Sí |
| 2 | Reserva manual de madera y de la caravana de fundación | `calibrarReservaManual` | Tesorero | Sí (dentro) |
| 3 | 2 granjas, mercado, barracón, galería, caballerizas, palacio, sala de consejo | `anadirEdificioManualmente` | Gobernador | Sí (dentro) |
| 4 | Caravanas comerciales hasta el cupo (`construirCaravanaComercial`) | `crearCaravana` + `agregarCarroCaravana` + `comprarAnimalCaravana` | Residente | Sí (dentro) |
| 5 | Primer recinto y mejora a piedra | `comprometerRecinto`, `mejorarRecinto` | Gobernador | Sí (dentro) |
| 6 | Pedir la subida de nivel | `solicitarAscenso` | Gobernador | Sí (`ascensoDeAsentamiento`) |
| 7 | Trueque de supervivencia y para crecer: busca socio **leyendo el almacén de plazas ajenas** | `proponerTrueque` | Residente del lado A | **No**: el almacén ajeno no se ve, y las órdenes ajenas solo en su mostrador |
| 8 | Contestar trueques recibidos | `aceptarTrueque` / `rechazarTrueque` | Residente del lado B | Sí (su almacén) |
| 9 | Trueque de especialización (`avanzarAutoComercioSimulado`) | — | — | Se borra (§8.1) |
| 10 | Adoptar tecnología con la capital | `adoptarTecnologia` | Rey | Sí |
| 11 | Reclutar la mejor escuadra posible | `reclutarTropa` | Cada residente, para sí | Sí |
| 12 | Guarnecer todo menos la última escuadra | `asignarGuarnicion` | Cada héroe, lo suyo | Sí |
| 13 | Loadout de defensa en casa | `guardarLoadout` | Cada héroe | Sí |
| 14 | Cazar el campamento de bandidos de la plaza y atacarlo al llegar | `movilizarEjercito` + `atacar` | Héroe sin cargo | Sí, si el campamento se ve (`campamentosBandidos` trae su poder) |
| 15 | Campaña contra la plaza rival más cercana que **pueda ganar**, con la defensa prevista (héroes y escuadras ajenas) | `movilizarEjercito` + `unirseAEjercito` | Héroes sin cargo | **No**: la guarnición y los héroes de una plaza ajena no se ven |
| 16 | Quedarse en lo conquistado | `cambiarResidencia` + `guarnecer` | Héroes de la columna | Sí |
| 17 | Repartir héroes entre plazas de la Facción | `salirAlMundo` + `cambiarResidencia` | Héroe sin cargo | Con la pizarra (residentes por plaza) |
| 18 | Volver a casa / replegar las columnas acampadas | `entrarEnAsentamiento` / `replegarEjercito` | Líder | Sí |
| 19 | Órdenes de compra/venta por excedente o escasez | `colocarOrdenMercado` | Residente | Sí (dentro) |
| 20 | Perseguir columnas y caravanas enemigas a la vista | `perseguir` | Líder | Sí (`ejercitosAvistados`, `caravanasAvistadas`) |
| 21 | Expandir: lanzar la caravana de fundación desde la plaza, salir con ella enganchada, llevarla a un sitio y fundar (`src/bots/cerebro/fundar.ts`) | `lanzarCaravanaFundacion`, `movilizarEjercito`/`salirAlMundo`, `adjuntarCaravana`, `fundar` | Residente sin cargo (el titular) | Sí (`caravanas`, `asentamientosAvistados`) |
| 22 | Fundación inicial de Facciones NPC (`fundarAsentamientosIniciales`, `crearFaccionNpc`) | — | — | Se queda **solo como andamio del batch** hasta el paso 4 (D53/D58) |

**Lo que cambia por el camino aunque las decisiones sean las mismas:** los ids los da la sesión (no
`ejercito-npc-N`), cada comando se valida y aplica por separado, y el orden pasa a ser por héroe en vez de por
plaza. El batch actual (estado crudo + `avanzarNpcGobernanza`) no sirve para esto: pasa a correr sobre
`GameSession` con el adaptador en proceso. Las cifras de los diarios de batch anteriores dejan de ser comparables
(ya cerradas por D58).

### 10.1 Decisiones del usuario sobre el inventario (2026-10-04)

- **Trueques (fila 7)**: lo urgente, con órdenes de compra en el mercado propio; trueques solo con plazas de las
  que la Facción sabe algo. Para saberlo, un héroe bot de la Facción hace de **explorador**: recorre el mapa,
  mira el mostrador de las plazas (sus órdenes en pie, que es lo que un jugador ve en la puerta) y lo apunta en
  la pizarra. Nada de omnisciencia.
- **Campañas (fila 15)**: solo contra plazas **inspeccionadas** antes. Inspeccionar una plaza es regla nueva
  (Doc 5.12.3): desde el anillo de 40, revela guarnición y héroes dentro, no el almacén, y avisa a su Facción. El
  explorador es quien inspecciona.
- **Expansión (fila 21)**: la caravana de fundación (Doc 1.8) es la misma que la de un campamento: nace parada en la plaza,
  sin destino, con el residente sin cargo que la lanza como titular; este sale con su tropa, la engancha, la lleva a un
  sitio y funda con `fundar`. Solo expande una plaza con al menos tres residentes, para dejar a alguien en casa.
- **Render**: nada se despliega hasta terminar el bloque entero (pasos 2-4). Tampoco se lanzan batch ni pruebas de
  comportamiento hasta entonces: el servidor puede quedarse sin bots mientras tanto.
- **Asimetrías del motor que caen con D52** (no hace falta decisión nueva: «un bot no puede hacer nada que un
  jugador no pueda hacer»): el asedio automático al llegar y el combate automático al alcanzar a la presa de las
  columnas sin humanos (`engine/ejercitos.ts`), y las reglas solo-NPC de derrota (`session/derrotas.ts`, D59). Lo
  que se queda: un combate sin humanos se resuelve con números y no en Unity (`hayHumano`), porque eso decide
  *dónde* se juega, no *quién* puede hacerlo.

### 10.2 Cómo se implementa

1. **Motor y sesión**: fuera las asimetrías de §10.1; `inspeccionar` acepta una plaza.
2. **`src/bots/`** (cliente, no motor): puerto (`observar` = `proyectarParaJugador`, `actuar` = autorización +
   comando), adaptador en proceso sobre `GameSession`, runner (orden por id de héroe, RNG por bot, cada 5 ticks
   con desfase + despertar por eventos), pizarra por Facción y cerebros por rol: Rey, Gobernador, Tesorero,
   residente (reclutar, guarnecer, defender), cazador, campaña, explorador.
3. **El tick sin NPC**: `unTickCompleto` se queda en el tick. Se borran `avanzarFaccionesNpc`,
   `avanzarAutoComercio`, `simulacionAutoComercio` y `npcGobernanza`.
4. **Batch sobre `GameSession`** con el adaptador en proceso. La fundación inicial de Facciones de bots queda como
   andamio hasta el paso 4.

### 10.3 Hecho (2026-10-04)

- **Motor sin asimetrías**: llegar no asedia y alcanzar no combate, para nadie; el tick ya no abre batallas;
  `session/derrotas.ts` borrado (D59). `inspeccionar` acepta una plaza. Encontrado de paso y arreglado:
  `movilizarEjercito` y `unirseAEjercito` dejaban al héroe «dentro» de la plaza mientras marchaba (la gobernanza
  lo tapaba situándolo a mano); ahora va en su columna, como con `salirAlMundo`.
- **`src/bots/`**: `puerto.ts` (observar, actuar con autorización, mapa), `runner.ts` (ritmo, despertar por
  eventos, orden por id, RNG por bot, `intentar` con espera tras rechazo), `pizarra.ts`, y `cerebro/` (gobierno:
  Rey, Gobernador, Tesorero; militar: residente, cazador, campaña, explorador, mudanza, columna).
- **El tick sin NPC**: borrados `npcGobernanza`, `simulacionAutoComercio`, `avanzarFaccionesNpc`,
  `avanzarAutoComercio` y `SIMULACION_AUTO_COMERCIO`. `crearFaccionNpc` queda como andamio con su sitio inicial y
  su héroe bot.
- **Batch**: `run-batch-sim.ts` corre sobre `GameSession` con el runner de bots; checkpoint formato 2
  (`exportar()`), sin memoria de bots. Borrado `bench-batch-checkpoint.ts`. Sin corridas hasta terminar el bloque.
- **Lo que el cerebro hace distinto de la gobernanza**, además de lo decidido en §10.1: cada bot actúa sobre la
  vista con la que empezó a pensar (encadenar Rey → Gobernador → Tesorero cuesta un turno por paso); el Rey adopta
  tecnología solo desde dentro de una plaza y con lo que ve en su almacén; el destino de fundación se busca con las
  fichas de las plazas que ve, no con las completas; el reparto de héroes se cuenta con la pizarra.

## 11. Paso 3: presencia e identidad de bot (hecho 2026-10-04)

**Decisiones del usuario:** desde dentro de la plaza, las escuadras libres del campamento salen con el héroe (D33
literal); la guarnición se queda (D40b). Mientras D33b no exista con Conquest (`CQ-010`), quien está en una batalla no sale del
mundo hasta que termina. Reglas en el canon, Doc 1.10.6.

- **Motor** (`engine/presencia.ts`): `salirDelMundo` (plaza, columna sola —con sus caravanas adjuntas de vuelta al
  origen, D40—, o ejército con más gente —se separa y el mando pasa al de más antigüedad—) y `volverAlMundo` (a su
  plaza si sigue residiendo, si no a su punto con su columna). Contenedor de escuadra `fuera` y campos
  `Heroe.desconectaEn` / `Heroe.fuera`, también en el contrato v1 y en el modelo de datos compartido (aditivo).
- **Sesión**: comandos `conectarse` / `desconectarse` (cada uno el suyo); el tick saca a quien cumplió los 2:30 y no
  está en batalla; fuera del mundo la autorización solo deja `conectarse` (`fuera_del_mundo`).
- **Servidor**: abrir el WebSocket de tiempo real conecta; cerrar el último socket del jugador desconecta; apagar el
  servidor no desconecta a nadie.
- **Bots** (`src/bots/runner.ts`, D55): cada bot tiene su horario por semilla, uno o dos bloques que suman de 2 a 6 horas
  al día (placeholder). Al empezar su sesión manda `conectarse`, al acabarla `desconectarse`, y fuera no piensa. Los
  tests de comportamiento usan `horario: 'siempre'`; el batch, el de semilla.
- **Identidad** (§8.3): `POST /v1/registro` con `CODIGO_REGISTRO_BOTS` da de alta una cuenta de bot (la credencial
  local lleva `bot: true`). Su héroe nace con `controlador: 'bot'`, que pone el servidor y no el cliente. Ningún otro
  poder. El runner en proceso no usa cuentas (actúa con los héroes directamente); las usará el adaptador remoto.

## 12. Paso 4: el bot sin plaza (hecho 2026-10-04)

Sobre los campamentos ya implementados (pasos 0-7 del plan de campamentos). Código: `src/bots/cerebro/sinPlaza.ts`,
`src/bots/llegadas.ts`; el cerebro manda a un bot a `sinPlaza` mientras no reside en ninguna plaza.

- **Llegada (D56, D57)**: `planDeLlegadas(semilla, total, días)` reparte grupos de tres amigos (50 %), solitarios (30 %) y
  tardíos (20 %, en la segunda mitad). Quien da de alta (el batch, y el runner remoto cuando exista) crea el héroe por el
  puerto (`puerto.crearHeroe`, que en el servidor es el registro con `CODIGO_REGISTRO_BOTS` + `crearHeroe`) en el campamento
  con menos residentes de la pantalla de elección (`puerto.campamentos()`, la misma lista que ve un humano sin héroe).
- **Facción**: el líder de los amigos y el solitario la crean; los demás piden entrar (los amigos, a la de su líder; el
  tardío, a una al azar) y, sin respuesta en 2 h, crean la suya. El Rey bot acepta toda solicitud.
- **Su Facción ya tiene plaza** (tardío aceptado, o compañero que fundó sin él): sale, `cambiarResidencia` y marcha a ella.
- **Economía del campamento**: pide la leva prestada (solo los lanceros: con las tres comería el triple) y la repone; con el
  oro de botín compra lo que falta del coste de la caravana y lo aporta al fondo, el oro al final; con el fondo completo,
  compra la caravana.
- **Explorar sin tropa**: sin un bandido conocido al alcance, uno sale solo (sin tropa no come) a dar la vuelta al anillo
  (8 puntos a 200, que su vista de 80 cubre) y abre los alijos que ve. Lo que ve de los bandidos va a la pizarra, porque
  los campamentos de bandidos no tienen memoria en la proyección.
- **Cazar**: sale con tropa solo a un bandido apuntado al que le llegue la ración (D81) de ida y vuelta, con el coste del
  terreno (`mapa.costeEnPunto`, público). Uno de nivel 1 lo caza solo; para más, hacen falta `poder / 30 + 1` héroes
  listos dentro: el de menor id sale como ejército con el bandido de rumbo y los demás se le unen en el mismo tick (el
  runner hace una segunda pasada para los que llama la pizarra). Tras el combate, los invitados se separan y el líder se
  repliega a la puerta.
- **Fundar**: el titular no saca la caravana sin el anillo explorado en las últimas 3 h. Espera hasta 40 min a que estén
  dentro los compañeros que viven allí, elige el sitio más cercano que el mapa público da por bueno (no agua, recomendable,
  a la exclusión de los campamentos, lejos de las plazas que conoce y con el camino lejos de los bandidos que conoce) y sale
  con todos como ejército hacia él, cada uno con su leva de escolta; engancha la caravana en la puerta. Si `fundar` dice que
  no, descarta el sitio y se repliegan.
- **Batch (D58)**: `scripts/run-batch-sim.ts` ya no funda Facciones en el tick 0: llegan `BATCH_BOTS` bots (30 por
  defecto) a lo largo de `BATCH_DIAS_LLEGADA` días (3). Checkpoint con `bots` y `diasLlegada`.
- **Borrado (D58)**: `crearFaccionNpc` (queda como fixture de tests, `session/__tests__/faccionAsentadaDePrueba.ts`) y
  `faccionesNpcIds`. «En una Facción NPC no se entra» (D49) deja de tener objeto.
- **Arreglos de paso**: la autorización de `adjuntarCaravana`/`soltarCaravana` (y el resto de comandos «de quien va en el
  ejército») cuenta a quien va sin escuadras; antes un viajero solo no podía enganchar su propia caravana.

Huecos de campamentos que destapó y que arregló la otra línea de trabajo: la ración se filtraba al unirse y separarse
(`f84297f`), el ejército de campamento salía sin rumbo (`dbdd464`) y no podía volver (`bca66af`), y la ración fija de 60 no
llegaba al anillo con el coste real del terreno (D81, `c192ed8`).

**Medido a mano (3 días, 3 amigos, semillas 1, 3 y 7)**: forman la Facción, exploran y abren alijos, y cazan poco. Hay
anillos con bandidos de nivel 3 (hacen falta 5 héroes) o en terreno caro (un nivel 1 a 199 pide 183 de trigo de ida y vuelta,
con 124 de ración). En 3 días el fondo no llega. Es calibración del bloque (D51), para el batch.

## 13. El adaptador remoto (hecho 2026-10-04)

Los bots juegan desde otro proceso por la misma superficie que BronzeAgeClient. Código: `src/bots/remoto/`.

- **El puerto es asíncrono** (`observar`, `actuar`, `conectar`, `desconectar` y `llegar` devuelven promesas), y con él el
  runner y los cerebros. Los bots siguen pensando uno tras otro, así que el batch da la misma partida con la misma semilla.
- **`PuertoRemoto`** (`puertoRemoto.ts`): una cuenta por bot, dada de alta con `POST /v1/registro` y `CODIGO_REGISTRO_BOTS`.
  La proyección es `GET /v1/jugador/partidas/:gameId`, los comandos `POST …/comandos` (un 403 es el `noAutorizado` del
  puerto) y el mapa `GET …/mapa/:mapaId`, una vez, con el estado de sus nodos de la última vista. Estar conectado es tener
  abierto el WebSocket de tiempo real (Doc 1.10.6): `conectar` lo abre (y el servidor manda `conectarse`) y `desconectar` lo
  cierra; a un bot que no lo tenía abierto se le manda `desconectarse` por HTTP. Por el socket escucha `mapa/general` y el
  canal de su plaza, y esos eventos son los que despiertan a los bots. Si la sesión caduca (401), vuelve a entrar.
- **`ProcesoDeBots`** (`procesoDeBots.ts`): el runner sobre el puerto remoto. Guarda un **registro** en JSON con las cuentas
  de sus bots, su perfil, cuántas llegadas lleva y el tick en que empezó, para volver a entrar con los mismos héroes tras un
  reinicio. Las llegadas cuentan desde ese tick (D56). Lee el tick con una cuenta sin héroe (el «reloj»). Cada `cadaMs`, si
  el mundo avanzó, da de alta las llegadas que tocan y hace pensar a los bots, recuperando como mucho los últimos 5 ticks. Un
  fallo de un bot se avisa y no para a los demás (`alFallar` del runner).
- **Arranque**: `npm run bots` (`main.ts`), configurado por entorno (`BOTS_SERVIDOR`, `BOTS_PARTIDA`, `CODIGO_REGISTRO_BOTS`,
  `BOTS_REGISTRO`, `BOTS_TOTAL`, `BOTS_DIAS_LLEGADA`, `BOTS_SEMILLA`, `BOTS_CADA_MS`, `BOTS_HORARIO`). Al apagarlo cierra las
  conexiones: sus bots se desconectan como quien cierra el cliente.
- **El alta, compartida**: `darDeAlta` (`llegadas.ts`) la usan el batch, los tests y el proceso remoto. El grupo va al
  campamento que elige el primero.
- **Probado**: test de integración con servidor real por red (`__tests__/puertoRemoto.test.ts`: alta, Facción, presencia por
  el socket, reinicio con las mismas cuentas) y en vivo con `npm run server` + `npm run bots` (6 bots, 90 ticks).

Pendiente: el servicio en Render (Background Worker con disco para el registro), cuando el bloque vaya a Render.

## 14. El servicio administrable y la pestaña «Bots» (hecho 2026-10-06)

El proceso de bots deja de ser un script que arranca jugando: con `BOTS_TOKEN` es un **servicio** que arranca inerte y se maneja
desde el cliente admin por su propio canal (decidido: servicio siempre vivo, conexión directa del navegador, una partida por
proceso). Código: `src/bots/control/`, `cliente/src/bots/`.

- **Canal** (`servidorDeControl.ts`): WebSocket en `/control` (puerto `PORT` o `BOTS_PUERTO`, 4000) más `GET /salud`. La primera orden
  es `autenticar` con `BOTS_TOKEN` (comparación en tiempo constante; sin ella, 5 s y se cierra). Protocolo tipado en
  `control/contrato.ts`, sin imports: el cliente admin lo usa solo como tipos. Mensajes: `estado` (tras cada vuelta y cada orden),
  `accion` (cada comando de un bot, en vivo), `registro`, `historial` (al conectar) y `respuesta`.
- **Servicio** (`servicio.ts`): fases `inactivo → arrancando → corriendo ⇄ pausado → parando`, más `error`. Órdenes: `iniciar`,
  `pausar`, `reanudar`, `parar`, `reiniciarRegistro`, `ajustar` (cadencia), `forzarLlegada`, `modoBot` (auto, conectado,
  desconectado, congelado), `retirarBot`, `pensarYa`, `volcarMemoria`. La semilla y los días de llegada no se cambian en marcha: el
  plan de llegadas se indexa por orden. Recuerda si estaba en marcha (`<BOTS_DIRECTORIO>/servicio-bots.json`) y al reiniciarse retoma
  solo; apagar el proceso no cuenta como parar. El `CODIGO_REGISTRO_BOTS` del entorno solo viaja al `BOTS_SERVIDOR` configurado.
- **Instrumentación** (no cambia lo que deciden los bots): `RunnerDeBots` expone `info()`, `pizarrasInfo()`, `alActuar` y el modo de
  cada bot; `PuertoRemoto` cuenta latencia, errores, sesiones renovadas y sockets; `ProcesoDeBots` mide cada vuelta.
- **Arreglado de paso**: un alta de amigos que fallaba a medias dejaba héroes fuera del registro y repetía la llegada entera;
  ahora cada bot se apunta en cuanto nace y la llegada se da por hecha si nació alguno.
- **Panel** (pestaña «Bots», solo red; subpestañas como las del panel de asentamientos): arriba, conexión y estado; **Servicio**
  (indicadores de salud —vuelta frente a `cadaMs`, peticiones, errores, latencia p50/p95— y configuración), **Flota** (cruce con el
  servidor, tabla filtrable y ordenable, detalle de un bot con memoria, horario, esperas y acciones), **Llegadas** (línea de tiempo y
  «forzar»), **Facciones** (pizarras) y **Actividad** (comandos en vivo y registro del servicio).
- **Huérfanos**: el registro de cuentas del servicio y los héroes del mundo son dos listas; si el registro se pierde o se borra, sus
  héroes siguen en el mundo (`controlador: 'bot'`) sin nadie detrás. La subpestaña Flota los cruza con los héroes que el cliente admin
  ya lee del servidor (solo si el admin está en la misma partida que los bots) y marca los que nadie maneja. En Render el servicio es un Web
  Service (necesita puerto), no un Background Worker.
- **Pendiente**: retirar de verdad las cuentas bot en el servidor (hoy «retirar» desconecta y deja de manejar al bot; su héroe sigue en
  el mundo), retirar de verdad a los huérfanos (hoy solo se ven), y reconectar el socket de un bot cuya conexión se cae sin
  que el runner lo sepa. Despliegue en Render con disco para `BOTS_DIRECTORIO`.

