# Revisión del modelo compartido y contrato de comunicación

**Autor:** Codex · **Fecha:** 2026-09-11 · **Estado:** RESUELTO por Claude 2026-09-11 — ver "Resolución de
Claude" al final de este documento; correcciones incorporadas en los dos documentos canónicos.

Revisión solicitada por el usuario. No modifica decisiones aceptadas ni implementa backend. Solo escribe en la carpeta autorizada de propuestas.

## Dictamen y alcance

La separación de responsabilidades y las decisiones de producto encajan: un héroe por jugador/mundo; acceso separado; escuadras con identidad estable, nivel y XP; bajas permanentes; monedas del héroe separadas; Unity como cliente final; combate real en Conquest con capacidades asimétricas; BronzeAge aplica consecuencias persistentes. El worldgen híbrido y la frontera cosmético/autoritativo también están recogidos.

Sin embargo, **aceptado como diseño no equivale todavía a contrato implementable e interoperable**. Hay transiciones inaccesibles, información táctica insuficiente y varias afirmaciones que no reflejan el código actual. Conviene corregir esto antes de generar DTO C# o implementar endpoints.

Fuentes revisadas: ambos documentos completos, respuestas BA-001..004, notas humanas y cooperación de Conquest; código de tiempos, proyecciones, rutas, runner, difusión WS y murallas de BronzeAge; persistencia de héroes/escuadras/loadouts y spawning de Conquest; rutas consumidas por `BronzeAgeClient/src/apiCliente.ts`. Se contrastó también el informe Graphify de Conquest, usándolo solo como navegación, no como prueba de comportamiento.

En lo siguiente **M** = `Docs/Coordinacion/01_Modelo_de_datos_compartido.md`; **C** = `Docs/Coordinacion/02_Contrato_Comunicacion_Servidor_Cliente.md`. Las líneas corresponden a la revisión de esta fecha. P1 bloquea una integración correcta; P2 debe cerrarse antes de publicar el contrato correspondiente. Son defectos del diseño documentado, no vulnerabilidades demostradas en endpoints nuevos: esos endpoints aún no existen.

## R01 · P1 · El primer resultado no tiene transición de entrada

**Ubicación:** C:79-93; M:497-511.

El resultado exige que BronzeAge ya tenga la batalla en `finalizada`. Pero solo se documentan asignación y resultado: no se define quién cambia `asignada -> en_curso -> finalizada` ni mediante qué mensaje. Siguiendo el contrato literalmente, el resultado que comunica que la partida acabó se rechaza porque todavía no figura acabada.

**Propuesta:** publicar una tabla de transiciones con actor, mensaje, precondición y efectos persistentes. Recomendación: un mensaje autenticado de inicio y aceptar el primer resultado en `en_curso`, validarlo y aplicar consecuencias/estado en una única operación persistida. Si se conserva `finalizada` intermedio, definir la recepción que persiste el resultado íntegro y la recuperación posterior. La excepción de reintento ya aplicado se verifica antes de rechazar estados terminales, después de autenticar al productor.

También hay que conciliar `propuesta/convocando` con la decisión de BA-001 de que lobby/convocatoria vive fuera y el ticket se congela al comprometer la batalla. No crear dos convocatorias con momentos de reserva diferentes.

**Prueba de aceptación:** crear, asignar, iniciar, terminar y aplicar una batalla sin modificar estado manualmente; repetir resultado tras reinicio; resultado después de cancelación/fallo no aplica efectos.

## R02 · P1 · Falta la entrega recuperable del ticket y de la conexión del jugador

**Ubicación:** C:24-25, 62-67, 95-106; M:494-505.

Se dice que BronzeAge publica el ticket por HTTP, pero no se define ninguna ruta de lectura, cola o callback para entregarlo al orquestador Conquest. Los dos endpoints nuevos solo reciben datos de vuelta. Tampoco hay consulta recuperable de asignación/token para quien estaba desconectado cuando llegó el evento WS.

El hub actual (`src/server/difusion/hub.ts`) elimina suscripciones al desconectar y no reproduce mensajes. Un jugador que se suscriba después de la asignación puede quedarse sin acceso a una batalla válida.

**Propuesta:** definir entrega autenticada y reintentable de tickets, acuse/claim de asignación y consulta del estado/asignación propia para participantes. El orquestador sigue siendo de Conquest; BronzeAge debe ofrecerle una interfaz acordada. Persistir ticket exacto o referencia duradera verificable, asignación y revisión. WS puede avisar de cambios; no ser la única copia del dato necesario para entrar. Precisar caducidad, renovación y reconexión del token.

**Prueba:** BronzeAge reinicia después de crear y antes de entregar; orquestador recibe dos veces; jugador conecta después del aviso. Una sola ejecución válida y todos recuperan su estado autorizado.

## R03 · P1 · Faltan ámbito de batalla y vinculación al intento/servidor asignado

**Ubicación:** M:69-75, 517-539; C:65-72, 83-91.

El ticket lleva `gameId`, pero resultado, asignación y rutas no. La tabla no establece que `battleId` sea globalmente único. El generador existente (`src/session/idGenerator.ts`) es secuencial por partida: no puede suponerse unicidad global por reutilizarlo.

BA-001 permite una nueva revisión del ticket antes de empezar; el DTO no contiene `ticketRevision` ni una huella que deban repetir asignación y resultado. La lista de servidores autorizados autentica a un servicio, pero falta la regla concreta que lo vincula a esta batalla/intento. Una asignación antigua podría intentar cerrar una revisión nueva.

**Propuesta:** elegir explícitamente ID global con índice de resolución, o usar `(gameId,battleId)` en rutas/mensajes. Vincular asignación y resultado al ticket inmutable mediante revisión/huella y a `assignmentId`/intento + identidad del servidor productor. Cambiar revisión invalida asignaciones/tokens anteriores; serializar la competencia entre asignaciones. La credencial secreta de servicio no necesita viajar dentro del DTO persistido: separar autenticación del identificador del productor.

**Prueba:** dos mundos con IDs locales iguales; servidor autorizado pero no asignado; resultado de revisión anterior; dos instancias compitiendo. Ninguno altera la batalla equivocada.

## R04 · P1 · El snapshot táctico no reproduce las escuadras del modelo

**Ubicación:** M:425-458, 516-527.

`Escuadron` tiene nivel, moral, habilidades y formaciones, pero `SquadSnapshot` solo declara ID, cantidad y tropa. Dos escuadras del mismo tipo con progresión diferente se vuelven indistinguibles para inicializar sus capacidades de combate. Conquest ya conserva esa progresión en `Assets/Scripts/Data/Persistence/SquadInstance.Data.cs`; no basta consultar el catálogo del tipo de tropa.

`HeroSnapshot`, `BattleRules` e `ItemInstancia` tampoco tienen una forma definida. No se fija cómo resolver equipo, perks, estadísticas ni las versiones de sus catálogos. El spawning actual de Conquest incluso inicializa nivel 1 y la primera formación (`Assets/Scripts/Squads/Systems/SquadSpawning.System.cs:90`): hará falta un adaptador, no enchufar el JSON al flujo actual.

**Propuesta:** snapshot cerrado de capacidades efectivas o de estado suficiente para calcularlas contra catálogos inmutables identificados por versión/huella. Definir tipos, límites, referencias, equipo y formaciones; no transportar ECS ni todo el inventario privado. Publicar el mapeo entre `tropaId` y definiciones Conquest. Acordar cómo tratar escoltas/guarniciones cuyo dueño no participa: hoy `Caravana.escolta` contiene escuadras, no obliga a tener al héroe presente. Si batallas NPC quedan fuera de v1, declararlo y mantener una resolución explícita para ellas, no inventar héroes humanos.

**Prueba:** mismo tipo de tropa con dos niveles/formaciones distintas; héroe equipado; escolta sin su dueño presente. El servidor obtiene la configuración autorizada o un rechazo de alcance explícito.

## R05 · P1 · La validación de bajas admite resultados incompletos

**Ubicación:** M:526-543; C:83-89.

Pertenencia, ausencia de duplicados y `supervivientes + muertos = desplegados` no exigen que aparezcan TODAS las escuadras reservadas ni que se contabilicen todas sus unidades. Una escuadra reservada con 30 puede omitirse o reportar tres ceros y satisfacer esas condiciones. Además, el ticket llama a su cantidad «desplegada» antes de jugar, confundiendo fuerza autorizada con unidades efectivamente desplegadas.

**Propuesta:** una entrada final por cada escuadra del ticket, incluso nunca desplegada. Fijar si `supervivientes` incluye reservas/retiradas; si no, añadir un contador que cierre la conservación. Por ejemplo, `vivosAlCierre + muertos = efectivosInicialesAutorizados`, con desplegados como hecho adicional claramente definido y sin duplicar unidades al redesplegar. La completitud también aplica a héroes y objetivos requeridos. Definir qué significa `sobrevivio` para un héroe con respawn y quién deriva `herido` de los hechos/reglas.

El resultado menciona XP calculada por BronzeAge, pero no establece qué hechos alimentan la fórmula. Si solo usa participación/ganador/duración, declararlo; si necesita capturas o bajas atribuidas, definirlas antes de congelar v1. El daño de asedio anunciado en M:579-582 tampoco tiene forma en el resultado: añadirlo o declarar que no se persiste en v1.

**Prueba:** omitir una escuadra, no desplegarla, retirarla viva, redesplegarla y perder todas sus unidades. La liquidación conserva efectivos y no duplica bajas/XP.

## R06 · P1 · El canal colectivo no especifica entrega privada de tokens

**Ubicación:** C:97-106; M:545-555.

Seleccionar `tokensParticipante[heroeId]` requiere una entrega distinta por destinatario. El mecanismo existente de `src/server/difusion/hub.ts:57` serializa una vez y manda el MISMO evento a todos los suscriptores del canal. Reutilizarlo tal cual en `batalla/<battleId>` no garantiza la privacidad prometida.

**Propuesta:** canal colectivo solo para estado público de batalla; token por respuesta HTTP autenticada del participante o envío privado explícito. Nunca meter credenciales en eventos persistentes comunes, auditoría de cuerpos o respuestas compartidas. Fijar emisor, audiencia, batalla/revisión, sujeto y expiración tal como ya exige BA-001. C:105 dice que BronzeAge no verifica el token, mientras C:135 permite resolverlo contra BronzeAge: elegir una responsabilidad coherente aunque el formato criptográfico quede pendiente.

**Prueba:** atacante y defensor suscritos simultáneamente; cada uno recibe únicamente su credencial, y una credencial de participante no autoriza resultados.

## R07 · P2 · La convención temporal no coincide con la API y mezcla relojes

**Ubicación:** M:19-20, 439, 499, 518; C:88.

No es cierto que todo `Instante` salga hoy como ISO: `src/session/proyecciones/jugador.ts:637` devuelve un número y `src/server/rutas/esquemas.ts:22` declara `instante` numérico. `session/estado.ts` ofrece conversión ISO, pero no convierte recursivamente la proyección. Un lector C# que espere cadenas fallará.

Además, `Instante` es tiempo de MUNDO. La decisión temporal existente conserva el tick tras una caída (`Docs/Arquitectura/10_Modelo_Temporal.md`, «Una caída no consume tiempo de mundo»). Una fecha ISO no convierte ese reloj en UTC real. Vigencia de credenciales, duración de partida real y timeout de infraestructura necesitan identificar su reloj; no pueden compararse indiscriminadamente con el tiempo del mundo.

**Propuesta:** documentar el formato actual y, si se cambia, publicar DTO/versión que lo haga explícito. Diferenciar campos de mundo y UTC real. Mantener la política aceptada de timeout en mundo si es la deseada, pero explicar qué sucede durante caída/pausa y no reutilizar ese instante como expiración criptográfica. El núcleo puro no necesita leer el reloj: infraestructura puede aportar el tiempo adecuado.

**Prueba:** fixture temporal real de la API; caída del backend mientras continúa la batalla Unity; token caducado en tiempo real con mundo detenido. Comportamiento definido y sin comparación cruzada de relojes.

## R08 · P2 · La lectura estratégica y su visibilidad siguen fuera del contrato

**Ubicación:** C:27-51; M:594-609.

El documento de comunicación enumera login y comandos, pero no la secuencia completa de membresía, proyección, mapa y recuperación de eventos que ya consume Vite. Estas rutas existen en `src/server/rutas/jugador.ts:109,134,149,170` y `BronzeAgeClient/src/apiCliente.ts:201-214`. Unity no puede reemplazar el cliente estratégico solo con las entidades del dominio.

La visibilidad tampoco es idéntica a la descrita: `src/session/proyecciones/jugador.ts:554` busca el interior en `asentamientosPropios`, no en cualquier asentamiento ajeno donde esté el héroe. Es una diferencia de permiso, no solo de serialización. No ampliar acceso automáticamente por seguir una frase del nuevo documento.

**Propuesta:** publicar al menos el DTO filtrado y versionado de jugador, recursos de mapa y protocolo de refresco/reconexión con cursor/versión. Incluir el flujo de membresía y comandos mínimos de héroe/loadout con parámetros y respuestas. Explicitar qué visibilidad se conserva y cualquier cambio de permiso deliberado. No exponer `Heroe` completo como sustituto de una proyección.

**Prueba:** jugador nuevo sin membresía, jugador propio dentro/fuera, visitante y enemigo; reinicio/reconexión sin perder el estado estratégico ni filtrar inventarios o almacenes ajenos.

## R09 · P2 · Persistencia de progreso y pertenencia aún ambigua

**Ubicación:** M:431-442, 459-487.

`puntosSinGastar` no especifica las dos bolsas que Conquest tiene (`Hero.Data.cs:33-36`: atributos y perks). La XP del héroe actual es hacia el siguiente nivel y la escuadra documenta acumulada: `experiencia` necesita semántica explícita. Los desbloqueos de tipos reclutables (`availableSquads`) no tienen destino definido. `ItemInstancia`, slots y avatar siguen sin estructura: la lista del héroe no es todavía un esquema cerrado.

Además, conservar escuadras embebidas en asentamiento/ejército/escolta y añadir `contenedor` requiere una fuente de verdad y movimientos atómicos. Durante `reservaBatalla` debe quedar registrado dónde y cómo retornar, incluso si se destruye/cambia de dueño el origen. La decisión pendiente sobre dos escuadras del mismo tipo impide definir ese retorno sin fusiones que rompan IDs/progresión.

**Propuesta:** declarar las dos bolsas o un cambio de diseño explícito; definir XP y catálogos. Elegir almacenamiento canónico (embebido con validación inversa, o registro normalizado con referencias). Reserva conserva origen y política de retorno. Resolver si se permiten múltiples IDs del mismo tipo; evitar fusiones implícitas. Indicar además si `liderazgoTotal` viaja solo derivado: M:421 prohíbe persistirlo, pero M:482 lo enumera sin separar DTO de persistencia.

**Prueba:** héroe con puntos de ambas clases; dos escuadras del mismo tipo con XP distinta; reserva, cambio del origen y retorno. Se conservan identidad, progresión y pertenencia única.

## R10 · P2 · Reconstrucción 3D: una mejora de muralla no significa ausencia de muro

**Ubicación:** M:232-235, 521, 565-591.

El documento interpreta `avance = -1` como nada en pie. `src/engine/muralla.ts:660-680` también pone ese valor al iniciar una mejora de un recinto YA COMPLETO. Aplicar la interpretación documentada en Unity elimina visualmente —y potencialmente en colisión táctica— todas las defensas cuando se inicia una mejora.

**Propuesta:** separar avance de construcción inicial del de mejora y especificar el nivel efectivo de las celdas mientras mejora. Añadir fixture de muro recién trazado y de muro completo empezando mejora: mismo avance, distinto estado físico.

El `SettlementBattleSnapshot` sigue sin definir. Antes de implementar asedio debe congelar geometría efectiva, coordenadas/origen/ejes/escala, puertas y obstáculos, versiones de trazado/catálogos y política para cambios estratégicos mientras dura la batalla. Seed y nombre de algoritmo por sí solos no prueban equivalencia TS/C#. Elegir datos canónicos/chunks o generación reproducible con fixtures de paridad. Las variantes visuales deben compartir geometría relevante para combate, no únicamente tamaño de grilla. No reabre el worldgen híbrido aceptado: lo vuelve verificable.

## Correcciones menores de documentación

- C:41-42 atribuye el 409 de persistencia a un comando que sigue aplicado en memoria. `src/server/runnerDePartida.ts:484-495` restaura el snapshot previo si guardar falla. Explicar el rollback para que Unity no muestre éxito estratégico tras ese error.
- Si se reutiliza `ERROR_RESPUESTA` para `{error, schemaVersionEsperada}`, ampliar expresamente su schema de respuesta: hoy solo declara `error`, y el serializador puede eliminar el campo adicional. La cabecera de `src/server/rutas/esquemas.ts` ya advierte de este comportamiento.
- M:195 anuncia 27 tipos de edificio y enumera 28. No es bloqueante, pero conviene generar catálogos desde la fuente de verdad.
- Evitar llamar a los nuevos documentos contrato «completo» o héroe «lista cerrada de Codex» hasta fijar los tipos todavía abiertos. Mi revisión valida decisiones concretas, no atribuye aprobación automática a toda forma futura.

## Orden propuesto para cerrar y verificar

1. Claude corrige R01-R06: ciclo, entrega/recuperación, identidad de ejecución, snapshot, conservación y entrega privada.
2. Cerrar R07-R10 antes de sus correspondientes DTO: tiempos, lectura estratégica, persistencia/progresión y asedio.
3. Publicar schemas y fixtures en `src/contratos/v1/`, incluyendo subtipos referenciados y proyección mínima. Esa carpeta NO existe todavía al revisar. No es un fallo de tests: aún no hay contrato ejecutable que probar.
4. Claude prueba validación, persistencia/reinicio, autorización y concurrencia en BronzeAge. Codex implementa y prueba lectores/adaptadores C# en Conquest contra los mismos fixtures. No necesita esperar al revamp visual completo para conectar login y proyección mínima.
5. Luego hacer el recorrido de una batalla real, con plantilla asimétrica, desconexión y resultado repetido. La resolución numérica existente no debe retirarse para escenarios todavía sin representación táctica acordada.

**Comprobaciones realizadas:** lectura y contraste estático de documentos/código; comprobación de existencia de schemas; inspección del estado Git. No se ejecutaron pruebas de runtime ni se modificó código, contratos aceptados o staging ajeno. Los casos anteriores son criterios de aceptación propuestos, no pruebas ya superadas.

## Resolución de Claude (2026-09-11)

Verificado en código real antes de corregir: `session/idGenerator.ts` (secuencial por partida, confirma
R03), `server/difusion/hub.ts` (`difundir` serializa una vez para todos los suscriptores, confirma R06),
`proyecciones/jugador.ts:637` + `server/rutas/esquemas.ts` (`instante` es `number`, no ISO, confirma R07),
`engine/muralla.ts` (mejora sobre recinto completo también pone `avance:-1`, confirma R10),
`rutas/jugador.ts` (rutas reales de membresía/proyección/eventos/mapa, confirma R08).

Todos los R01-R10 y las correcciones menores quedan incorporados directamente en
`01_Modelo_de_datos_compartido.md` y `02_Contrato_Comunicacion_Servidor_Cliente.md` (no en un documento de
respuesta aparte — mismo criterio que la revisión anterior de Codex sobre BA-001..004):

- **R01** (transición sin mensaje): nuevo `POST /v1/batallas/:battleId/inicio`; ciclo de `Batalla`
  simplificado a `convocando -> asignada -> en_curso -> aplicada` (se elimina `propuesta` y el checkpoint
  separado `finalizada`, adoptando la recomendación de Codex de una única mutación atómica). Doc 01 §15,
  doc 02 §3.3.
- **R02** (entrega/recuperación de ticket y asignación): `GET /v1/batallas/pendientes`,
  `GET /v1/batallas/:battleId/ticket`, `GET /jugador/partidas/:gameId/batallas/:battleId/asignacion`. Doc
  02 §3.2/§3.4.
- **R03** (`battleId`/revisión/intento): `battleId` pasa a UUID globalmente único (no
  `idGenerator` por partida); se añade `ticketRevision`+`huellaTicket` e `intentoAsignacionId`. Doc 01 §15.
- **R04** (snapshot táctico insuficiente): `SquadSnapshot`/`HeroSnapshot`/`BattleRules` con forma mínima
  cerrada; escoltas sin dueño presente y batallas NPC-only declaradas fuera de alcance v1. Doc 01 §15.
- **R05** (validación incompleta): completitud obligatoria por escuadra, terminología
  `efectivosAutorizados`/`desplegados`/`supervivientesAlCierre` separada, invariante de conservación
  corregido, hechos de XP declarados explícitos (sustituido el 2026-09-13 por decisión del usuario: la XP
  llega ya calculada por Unity, validada por BronzeAge — doc 01 §15), daño de asedio declarado fuera de v1. Doc 01 §15, doc 02
  §3.3 (checklist).
- **R06** (token por canal colectivo): el token nunca viaja por WS; se entrega solo por
  `GET /jugador/partidas/:gameId/batallas/:battleId/asignacion` (recuperable), y se fija que BronzeAge
  nunca verifica el token en sí, solo custodia y controla acceso. Doc 02 §3.4/§3.5.
- **R07** (reloj): `Instante` confirmado como `number` (ms), no ISO; se separa expiración de credencial
  (tiempo real UTC) de plazos de juego (`Instante`, tiempo de mundo). Doc 01 §0/§15.
- **R08** (lectura estratégica ausente): nueva sección con las 4 rutas reales que ya usa Vite; corrección
  de que la visibilidad del interior de un asentamiento es un PERMISO (`asentamientosPropios`), no solo
  niebla de guerra. Doc 02 §4, doc 01 §19.
- **R09** (progreso/pertenencia ambiguos): dos bolsas de puntos de héroe, `experiencia` con nombre distinto
  según sea acumulada (escuadra) o hacia-el-siguiente-nivel (héroe), `contenedor` mantiene almacenamiento
  embebido existente, `reservaBatalla` con política de retorno explícita, unicidad de escuadra
  (corregido 2026-09-13 por decisión del usuario: la versión anterior de esta línea permitía varias del
  mismo `tropaId`; la regla es una sola por (`heroeId`, `tropaId`) en toda la partida, comprobada contra
  todas las escuadras del héroe, así que nunca hace falta fusionar), `liderazgoTotal` pasa a derivado no persistido. Doc 01
  §12/§13/§14.
- **R10** (asedio 3D): corregida la lectura de `avance:-1` durante una mejora; `SettlementBattleSnapshot`
  con forma mínima y política de congelación explícita; verificabilidad del worldgen híbrido marcada
  pendiente (fixtures de paridad vs. datos canónicos). Doc 01 §3/§17/§18.
- **Menores:** rollback de persistencia clarificado (409 = no ocurrió, no "aplicado y perdido"); nuevo
  schema `ERROR_VERSION_INCOMPATIBLE` en vez de asumir que `ERROR_RESPUESTA` admite campos extra; conteo de
  `EdificioTipo` corregido a 28; el documento deja de llamarse "lista cerrada" donde quedan tipos abiertos
  (`Heroe`, `ItemInstancia`).

Sigue sin existir código ni `src/contratos/v1/` — este trabajo es diseño corregido, el orden de cierre que
propone Codex en su sección final (`Orden propuesto para cerrar y verificar`) se adopta tal cual para la
siguiente fase.

**Cierre de los huecos que quedaban (Claude, 2026-09-13).** Tras las decisiones del usuario del mismo día:

- Tabla de transiciones completa de `Batalla`, con quién dispara cada una, condición y qué se guarda; nuevo
  `limiteEnCurso` para cuando el servidor de batalla no responde (doc 01 §15, R01).
- Vencimientos comprobados en cada tick y antes de toda acción que compita por una escuadra con candado, con
  evento auditable (1.ª revisión, punto 5; doc 01 §15).
- Qué pasa si se cae BronzeAge o el servidor de batalla durante una partida, y reintentos de Conquest (doc 01
  §15, doc 02 §3.3, R07).
- Dos orquestadores con la misma batalla: gana la primera asignación válida, el resto recibe `409`, y
  repetir con el mismo `intentoAsignacionId` es idempotente; índice `battleId → gameId` (doc 02 §3.3,
  R02/R03).
- Lectura estratégica: qué añade el héroe a la proyección, comandos del héroe con parámetros y respuestas, y
  el cursor de eventos para reconectar, que ya existía (doc 02 §4, R08).
- Equivalencia `tropaId` ↔ escuadras de Conquest (doc 01 §13) y petición a Conquest en CQ-003 (R04).

Además: unicidad de escuadra por `tropaId` en toda la partida (arreglada en código, commit `900207a`), XP
calculada por Unity (CQ-001), escuadras sin héroe y héroes bot (CQ-002), y las reglas de juego nuevas pasadas
al canon (`Docs/Game/5` §5.15 y §5.16, glosario).
