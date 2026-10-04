# NPC fuera del motor: el bot como cliente

Plan para sacar **todo el comportamiento NPC** (gobernanza incluida) del motor y del tick, y que los bots jueguen
**desde fuera, como un jugador más**: ven lo que ve un jugador y actúan con los comandos de un jugador.

Nace de dos decisiones del usuario (`Consideraciones/Campamentos_Entrada_Fundacion_Definicion.md`):

- **D52**: el NPC sale del motor porque ya ocupa demasiado del tick, y la lógica nueva de bots-héroe lo empeoraría.
- **D53-D59**: los bots-héroe nacen en los campamentos y siguen el mismo flujo que un humano (sesiones, llegada
  escalonada, tres perfiles), con esa lógica fuera del motor.

> **Estado (2026-10-04): plan, sin código.** Decidido: el bot es un cliente (§3), persistencia con diario (§5).
> Decidido también: auto-comercio, ritmo y alta de bots (§8). Abierto: §9.

## 1. Cómo es hoy (medido en el código)

- `RunnerDePartida.unTickCompleto` (`server/runnerDePartida.ts`) encadena **tick → auto-comercio → turno NPC** en
  **una sola entrada** de la cola serial. Mientras el NPC piensa, ningún comando de jugador entra.
- El NPC (`session/npcGobernanza.ts`, ~2400 líneas) **no usa comandos**: recibe el estado entero, sin niebla, y llama
  a funciones del motor directamente. Además hace cosas que ningún jugador puede hacer: crear héroes, anexionar
  Facciones, fundar a pie.
- **Cada comando persiste la partida entera**: `aplicarYPersistir` hace `exportar()` + `guardarPartida` por comando.
  Con cientos de bots mandando comandos, eso pesaría más que el propio NPC (§5).
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
   `CODIGO_REGISTRO` humano) que marca la cuenta como bot. *(A confirmar.)*

## 9. Abierto

1. **Orden de trabajo frente a los campamentos.** Propuesta: primero el diario (§5, vale ya para humanos), luego el
   puerto con el adaptador en proceso y la gobernanza actual pasada a comandos, luego presencia e identidad de bot, y
   solo entonces el cerebro «sin plaza» de los bots-héroe sobre los campamentos.
2. **Marca de cuenta bot** (§8.3): confirmar el código de registro propio.
