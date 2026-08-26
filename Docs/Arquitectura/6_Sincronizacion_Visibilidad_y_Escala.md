# Sincronización, visibilidad y escala (500+ jugadores por partida)

Documento de decisiones tomadas en la sesión del 2026-08-24, al analizar el objetivo real de escala del
proyecto: **mínimo 500 jugadores conectados simultáneamente en una misma partida**. Ese número no aparecía en
[2_Estudio_Evolucion_Backend_Multifrontend.md](2_Estudio_Evolucion_Backend_Multifrontend.md) y cambia varias
decisiones de diseño, así que se registran aquí antes de empezar la Fase B.

Todo lo de aquí es diseño acordado, no implementado. Las tareas derivadas están en
[4_Plan_Evolucion_Tareas.md](4_Plan_Evolucion_Tareas.md).

> **Ampliado 2026-08-26** con [§6, Modelo de sincronización](#6-modelo-de-sincronización-reglas-al-cliente-simulación-en-el-servidor):
> por qué este proyecto **no** puede usar lockstep determinista (el patrón clásico de RTS), y dónde cae de
> verdad la frontera cliente/servidor. Va al final para no renumerar las secciones, a las que ya apuntan
> comentarios del código y otros documentos, pero **justifica el §3 de abajo** y conviene leerlo antes.

## 1. Medición de escala del motor actual

Medido sobre el motor real (`avanzarSimulacion`) en la máquina de desarrollo, fundando N asentamientos con
una Facción cada uno (el cap de fundación de nivel 1 es 1 asentamiento por Facción), 50 ticks de calentamiento
y promediando 30 ticks:

| Asentamientos vivos | ms/tick | Estado serializado |
|---|---|---|
| 10 | 5.5 ms | 52 KB |
| 16 | 14.4 ms | 80 KB |
| 33 | 25.7 ms | 161 KB |
| 52 | 63.9 ms | 246 KB |

**Escalado: O(n^1.5)** — superlineal pero no cuadrático. Extrapolando a ~500 asentamientos: **~1.9 s por tick**
y **~2.4 MB de estado**.

> ⚠️ **Corrección posterior (2026-08-24): 500 jugadores NO son 500 asentamientos.** Un asentamiento aloja
> `CIUDADANIA.casasBasePorAsentamiento` = 5 residentes (+2 por nivel adicional), así que el objetivo de 500
> jugadores cabe en **~70-100 asentamientos** → **~170-300 ms por tick**, no 1.9 s. La cifra de 500
> asentamientos corresponde a una partida madura, con las facciones ya expandidas
> (`CAP_FUNDACION_POR_NIVEL`), no al punto de partida. Las conclusiones cualitativas de abajo no cambian (el
> cuello de botella sigue siendo la CPU del tick), pero la **urgencia** del problema del tick bloqueante sí:
> ~200 ms es tolerable, 1.9 s no. Ver [7_Diseno_GameSession.md](7_Diseno_GameSession.md) §8.

Consecuencias:

- **El cuello de botella es la CPU del tick, no la red ni la persistencia.** Un tick de 1.9 s frente a ~0.4 ms
  de serializar el estado: persistir cuesta menos del 0.1% del tick. Cualquier debate de rendimiento sobre
  framework HTTP o motor de persistencia es optimizar el margen equivocado.
- **El estado completo sigue siendo pequeño** (2.4 MB), lo que valida persistir por snapshot en vez de montar
  una base de datos relacional en la etapa provisional.
- **Problema nuevo: el tick bloquea la cola serial.** El principio 5 del doc 2 exige procesar en serie por
  `gameId`. Un tick de ~2 s son ~2 s en los que ningún comando de ningún jugador se procesa. Con 500 jugadores
  eso se nota. No está contemplado en el doc 2 y hay que resolverlo en Fase B/C — posibles vías: procesar
  comandos por lotes entre ticks, partir el tick en fases cedibles, o mover el tick a un worker aparte del
  hilo que atiende comandos. **Decisión pendiente.**

> Nota: las cifras son de una máquina de desarrollo y del balance vigente en 2026-08-24. Sirven para decidir
> orden de magnitud y forma de la curva, no como SLA. Conviene re-medir al cerrar la Fase D (el modelo
> temporal cambia el coste por tick).

## 2. Arquitectura de conexiones: una conexión, muchas suscripciones

**Decisión: una única conexión WebSocket permanente por jugador, con canales lógicos multiplexados encima.**

Se descartó la alternativa de abrir una conexión por pantalla/pestaña y cerrarla al salir. El motivo es que
una conexión WebSocket es **cara de abrir y barata de mantener**:

- Abrir: handshake TCP + TLS + upgrade HTTP, 2-3 round trips (100-300 ms en móvil), más re-autenticación y
  sincronización inicial. Con una conexión por pestaña, el jugador paga eso **en cada cambio de pestaña**.
- Mantener ociosa: ~10-30 KB de RAM en servidor. 500 conexiones simultáneas es una cifra trivial para Node.

El patrón adoptado es el estándar de industria (Phoenix Channels, Socket.IO rooms, MQTT topics): suscribirse
y desuscribirse son mensajes pequeños sobre la conexión ya establecida.

```text
conectar                            (una vez, al entrar a la partida)
  → suscribir   mapa/general
  → [el jugador abre la pestaña del asentamiento X]
  → suscribir   asentamiento/X
  → [sale de esa pestaña]
  → desuscribir asentamiento/X
desconectar                         (al cerrar el juego)
```

Se obtiene el objetivo buscado — el servidor solo empuja lo que el jugador está mirando — sin el coste de
reconectar. Reparto de responsabilidades entre transportes:

| Transporte | Para qué |
|---|---|
| HTTP | Carga inicial, consultas puntuales bajo demanda (detalle de un asentamiento propio, mercado, historial), comandos |
| WebSocket (única) | Notificar cambios en lo que el jugador tiene suscrito |

Detalles a tener en cuenta al implementar:

- Al reconectar **se pierden las suscripciones**: el cliente debe re-suscribirse solo. Encaja con la
  reconexión e idempotencia ya previstas en C3.
- Los navegadores limitan ~255 WebSockets por host (frente a 6 de HTTP/1.1) — otro motivo para no abrir una
  por vista.
- El WebSocket es canal de **notificación**, no de transmisión de estado masivo: ver regla (c) de la sección 4.

## 3. Modelo de visibilidad

Hoy **no existe ningún concepto de visibilidad en el código** (verificado: cero coincidencias de niebla de
guerra, visibilidad o conocimiento en `src/`). Coincide con lo que ya declaraba el doc 1: "No hay filtrado de
datos por facción, jugador, rol administrativo o niebla de guerra". Se diseña desde cero.

Un jugador nunca recibe el mapa completo. Su visibilidad sale de tres fuentes:

1. **Espacial** — zona de influencia de sus asentamientos más un valor propio de *visualización* que la
   extiende.
2. **Contacto** — asentamientos con los que tuvo trueque, mientras ese contacto siga vivo.
3. **Alianza** — actualizaciones de los aliados.

Esto es *interest management* (la técnica que la literatura de MMO usa para no difundir el mundo entero), pero
en una variante **semántica** —contacto y alianza— además de la puramente espacial, lo que la hace más
restrictiva y más barata que un AOI genérico.

### `ConocimientoJugador`: la visibilidad tiene memoria

"Los asentamientos con los que tuve contacto" no es función del estado actual: es historia acumulada por
jugador. Hace falta una entidad de dominio nueva que hoy no existe, que registre qué sabe cada jugador, desde
cuándo, y hasta cuándo sigue siendo válido.

### Decisión: se muestra el ÚLTIMO ESTADO CONOCIDO, no el actual

Ante la pregunta de si un jugador que comerció con el asentamiento X ve el estado *actual* de X o el que X
tenía *en el momento del contacto*, **se decide: el último conocido**.

Es el patrón clásico de niebla de guerra de los RTS — recuerdas lo que viste, no lo que está pasando ahora.
Consecuencias favorables:

- **No filtra información que el jugador no debería tener.** Con estado actual, un solo trueque daría
  telemetría en vivo del rival para siempre.
- **Tráfico de red casi nulo para lo ajeno**: no hay suscripción viva a entidades de otros; se envía una foto
  cuando el conocimiento se actualiza, y nada más.
- **Coste**: almacenar una foto por (jugador, entidad conocida). Es almacenamiento barato y acotado, y encaja
  con la estrategia de snapshots ya elegida para la persistencia.

Implicación para el diseño de proyecciones (doc 2, punto 7): la proyección de un jugador se compone de datos
**en vivo** (lo suyo, sus aliados) y datos **congelados** (lo conocido por contacto), y ambos deben ir marcados
con el momento en que se supieron, para que el frontend pueda mostrar "última información conocida: hace X".

## 4. Reglas de protocolo agnóstico al tick

El tick es una etapa provisional; el objetivo final es tiempo real (doc 2, Fase D). Para que esa transición no
obligue a rehacer API ni frontend, se adopta una regla rectora:

> **El protocolo entre servidor y cliente nunca debe mencionar ticks.**

Cuatro reglas concretas que la implementan:

**(a) Fechas absolutas, nunca contadores.**

```text
mal:   { ticksRestantes: 12 }
bien:  { completaEn: "2026-08-24T11:30:00Z" }
```

Doble beneficio, y el primero aplica ya hoy: con un contador el servidor tiene que empujar una actualización
*cada tick* solo para decrementarlo; con una fecha absoluta el cliente pinta la cuenta atrás localmente con
**cero tráfico adicional**. Reduce ancho de banda ahora y hace gratis la migración después. El doc 2 ya lo
anticipaba ("La API no debe exponer solo `ticksRestantes`"); esta es una segunda justificación independiente.

**(b) Los eventos llevan momento, no número de tick.** `EventoDominio` (Fase A5) debe llevar `momento` además
de `tick`, para que el día que el tick desaparezca el contrato no cambie.

**(c) Las suscripciones describen QUÉ, no CUÁNDO.** El cliente se suscribe a `asentamiento/X`; el servidor
empuja cuando X cambia. Al cliente le da igual si el cambio lo provocó un tick o un evento de tiempo real.
**Esta es la regla que hace que el frontend no cambie ni una línea el día de la migración.**

**(d) Las ventanas de validez son temporales.** "Contacto mientras esté vivo" se modela con `desde`/`hasta` en
tiempo, no en número de tick — igual que `heridoHastaTick`, `regeneraEnTick` y los cooldowns que el doc 2 ya
marca para migrar en D2.

## 5. Consecuencias para el roadmap

Tareas derivadas de este documento, reflejadas en
[4_Plan_Evolucion_Tareas.md](4_Plan_Evolucion_Tareas.md):

- ✅ **Hecho 2026-08-24** — `momento` (ISO 8601) añadido a `EventoDominio`, e introducido
  `ContextoSimulacion { tick, momento, rng }` como entrada única de `avanzarSimulacion` y
  `avanzarNpcGobernanza`. El motor no lee nunca el reloj por su cuenta; la capa de aplicación lo inyecta. Es
  la regla (b) de §4 implementada, y deja `tick` aislado en un solo sitio para poder retirarlo en Fase D sin
  tocar firmas.
- [ ] Resolver el bloqueo de la cola serial por ticks largos (Fase B/C, decisión pendiente).
- [ ] Diseñar `ConocimientoJugador` y la proyección "último conocido" (Fase C, junto a los DTOs por audiencia).
- [ ] Protocolo de suscripciones sobre conexión única (Fase C3).
- [ ] Ver §6: el mapa deja de ser estado y pasa a ser asset cacheable (C11), y `eventosDominio` deja de viajar entero (C13).

## 6. Modelo de sincronización: reglas al cliente, simulación en el servidor

Añadido 2026-08-26. Pregunta que lo abrió, del usuario: *"he escuchado que varios juegos tienen el motor en
cada front y uno en el back, y solo se manda la información de decisiones"*. Es un patrón real y con nombre;
la conclusión es que **aquí no aplica**, y el porqué fija de paso dónde cae la frontera de verdad.

### 6.1. Los tres modelos de la industria

| | Qué viaja | ¿El cliente simula? | ¿Admite información oculta? | Género típico |
|---|---|---|---|---|
| **Lockstep determinista** | Solo comandos | Sí, el mundo entero | **No, estructuralmente** | RTS: AoE, StarCraft, C&C |
| **State synchronization** | Comandos **y** estado | Sí, y se corrige contra el servidor | Parcial | Shooters, físicas |
| **Servidor autoritativo + cliente fino** | Solo estado ya filtrado | No | Sí, por diseño | MMOs, Quake 3 |

Lockstep es lo que describía la pregunta. Su propiedad estrella: el ancho de banda depende solo de **cuántos
comandos emite el jugador**, no de cuántas entidades hay en el mundo (Bettner & Terrano, *1500 Archers on a
28.8*, 2001).

### 6.2. Por qué lockstep queda descartado aquí

**Razón 1, y es definitiva: es incompatible con la niebla de guerra.** Lockstep exige que cada cliente conozca
el mundo entero para poder simularlo, **incluida la parte tras la niebla**. Por eso los maphacks existen en
StarCraft y Warcraft 3: el dato ya está en la máquina del jugador y la interfaz solo lo tapa. No es un fallo
que nadie arregló, es consecuencia del modelo — los juegos que quieren niebla incorruptible abandonan lockstep
justamente por esto.

Eso choca de frente con el §3 de este documento y con la Fase C4: las proyecciones se declararon **frontera de
seguridad, no refinamiento**. Adoptar lockstep sería renunciar a las tres fuentes de visibilidad de §3 y al
"último conocido" de §3.2.

**Razón 2: no hay latencia que esconder.** Lockstep y *client-side prediction* existen para dar respuesta
instantánea a 60 Hz. El tick de este motor mide ~1,9 s de CPU a 500 asentamientos (§1). El jugador ya espera
segundos por diseño. Se pagaría toda la disciplina de determinismo bit-exacto —que además arrastra que el
estado interno del RNG todavía no es serializable (A3, diferido a B3)— a cambio de nada.

**Se confirma el modelo 3**: servidor autoritativo, cliente sin simulación, estado filtrado por audiencia. Es
lo que ya construyen C4/C5, y este análisis no lo cambia — lo justifica.

### 6.3. La frontera real no es "motor sí / motor no", es REGLAS vs. SIMULACIÓN

El eje correcto no es si el cliente tiene motor, sino qué parte:

- **Simulación** (avanzar el mundo, resolver combates, producir, consumir): **solo servidor**, sin matices.
- **Reglas** (qué cuesta un granero, cuántos slots da este nivel, es válida esta posición): el cliente **las
  necesita** para dar respuesta inmediata. Un *tooltip* que hace viaje de ida y vuelta es inusable.

La industria manda las reglas al cliente **como datos, no como código**. EVE Online lo llama *Static Data
Export*: un *snapshot* publicado de todo lo que solo cambia con un parche. Y el problema que CCP describe al
rehacerlo es exactamente el de este proyecto — *"había que tener extremo cuidado sobre qué datos iban al
servidor y cuáles al cliente/SDE, para no filtrar información que no debía ser pública"*.

Aquí eso es **C7** (balance versionado **y servido**), y está medio hecho sin saberlo: `EDIFICIO_CATALOGO`,
`POLITICAS` y `NIVEL_FACCION` ya son tablas, no código.

> **La clasificación completa —módulo por módulo, de todo el motor— vive en
> [9_Reglas_vs_Simulacion.md](9_Reglas_vs_Simulacion.md).** Ese documento añade además un eje que esta sección
> no tiene y que resultó ser el que más decide: una regla puede ser perfectamente pura y aun así ser solo de
> servidor, porque su **entrada** es privilegiada (`calcularPrecioReferencia` suma el almacén de todos los
> rivales). Pureza y visibilidad son ejes independientes.

**Coste aceptado**: para las consultas que son tabla, si el cliente calcula, la fórmula acaba existiendo dos
veces (servidor por autoridad, cliente por presentación) y pueden divergir. Se acepta —la alternativa es un
viaje de red por *tooltip*— y se mitiga manteniendo las reglas lo más "tabla pura" posible, de modo que el
cliente haga *lookup* y no reimplemente lógica. Consecuencia para el hito C10: ver su desglose en el doc 4.

### 6.4. Medición: qué viaja hoy, de verdad

Estado real, 4 facciones fundadas, seed 42 (medido 2026-08-26):

| | tick 0 | tick 50 | tick 200 |
|---|---|---|---|
| `mapa` | 125,4 KB | 125,4 KB | 125,4 KB |
| `asentamientos` | 8,8 KB | 18,0 KB | 26,9 KB |
| `eventosDominio` | 2,0 KB | 41,0 KB | **102,0 KB** |
| `estadoMapa` | 0,0 KB | 0,2 KB | 0,3 KB |
| **Proyección total** | 129,6 KB | 144,3 KB | 160,4 KB |

Dos conclusiones, y ninguna era la esperada:

**(a) El mapa no es estado, es un asset.** 125,4 KB **idénticos byte a byte** en los tres cortes: se deriva de
la seed y no cambia jamás en toda la partida. Con solo 4 facciones ya es el 78% de la proyección, y desde C6
cada comando de jugador devuelve una proyección — así que hoy viajan 125 KB inmutables **en cada acción**. La
solución no es compresión delta: es servirlo una vez, cacheado por `seed`+`worldgenVersion` con
`Cache-Control: immutable`, y no volver a mandarlo. −78% sin maquinaria ninguna. → hito **C11**, que se abarata
mucho: la rasterización del terreno cae en el mismo sitio, porque también es determinista por seed.

**(b) El que crece sin techo es `eventosDominio`**: ×50 en 200 ticks, con 4 facciones, y viaja entero cada vez.
A tick 1000 con 500 asentamientos ese es el problema real. Se corta con un cursor (`?desde=<version>`), que
además es justo para lo que el WebSocket de C5 ya existe. → hito **C13**, cuyo objetivo cambia: el enemigo no
era el mapa.

## Fuentes consultadas

Rendimiento de frameworks y persistencia:

- [Express vs Fastify in 2026 — MG Software](https://www.mgsoftware.nl/en/vergelijking/express-vs-fastify)
- [Yet another nodejs benchmark — Pau Sanchez](https://www.pausanchez.com/en/articles/yet-another-nodejs-benchmark/)
- [Hono vs Express vs Fastify vs Elysia 2026 — PkgPulse](https://www.pkgpulse.com/guides/hono-vs-express-vs-fastify-vs-elysia-2026)
- [better-sqlite3 performance docs](https://github.com/WiseLibs/better-sqlite3/blob/master/docs/performance.md)
- [Event Sourcing with SQLite: Append-Only Design](https://www.sqliteforum.com/p/event-sourcing-with-sqlite)

Sincronización, visibilidad y conexiones:

- [Phoenix.Channel — HexDocs](https://hexdocs.pm/phoenix/Phoenix.Channel.html) y [Channels — Phoenix Guides](https://hexdocs.pm/phoenix/channels.html)
- [Scaling Real-Time Apps: The Art of WebSocket Multiplexing](https://me.aiyu.co.in/blogs/693d22c1986d3753c7357258)
- [Interest management in an MMO — GameDev.net](https://www.gamedev.net/forums/topic/609123-interest-management-in-an-mmo/)
- [Game Networking (5) — Compression, delta encoding, interest management — Daposto](https://daposto.medium.com/game-networking-5-compression-delta-encoding-interest-management-bit-packing-9316ff1c96db)
- [State Synchronization — Gaffer On Games](https://gafferongames.com/post/state_synchronization/)
- [MMO Architecture: Source of truth, Dataflows, I/O bottlenecks — PRDeving](https://prdeving.wordpress.com/2023/09/29/mmo-architecture-source-of-truth-dataflows-i-o-bottlenecks-and-how-to-solve-them/)

Modelos de sincronización y frontera cliente/servidor (consultadas 2026-08-26, para §6):

- [1500 Archers on a 28.8: Network Programming in Age of Empires and Beyond — Bettner & Terrano (2001)](https://www.gamedeveloper.com/programming/1500-archers-on-a-28-8-network-programming-in-age-of-empires-and-beyond) — el paper fundacional de lockstep determinista
- [What Every Programmer Needs To Know About Game Networking — Gaffer On Games](https://gafferongames.com/post/what_every_programmer_needs_to_know_about_game_networking/) — la taxonomía de los tres modelos
- [Client-Side Prediction and Server Reconciliation — Gabriel Gambetta](https://www.gabrielgambetta.com/client-side-prediction-server-reconciliation.html)
- [Network model for unhackable fog of war — GameDev.net](https://www.gamedev.net/forums/topic/583793-league-of-legends-network-model-for-unhackable-fog-of-war/) — por qué lockstep y niebla de guerra son incompatibles
- [EVE Online — Static Data Export](https://developers.eveonline.com/docs/services/static-data/) y [Reworking the SDE](https://developers.eveonline.com/blog/reworking-the-sde-a-fresh-start-for-static-data) — reglas al cliente como datos, y el riesgo de filtración al hacerlo

Antecedente de género (estrategia lenta por navegador, miles de jugadores por mundo, originalmente sin
WebSockets): [OGame — Wikipedia](https://en.wikipedia.org/wiki/OGame).
