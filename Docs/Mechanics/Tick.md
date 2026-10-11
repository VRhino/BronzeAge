# El tick: reloj, orden y efectos

Cómo avanza el mundo: quién dispara un tick, qué hace el comando `avanzarTick`, **en qué orden** corre cada sistema dentro de un tick y qué estado lee cada uno. Es el documento que une a los demás: cada
paso de la cadena se detalla en el suyo; aquí se fija el **orden**, las **instantáneas** que lee cada paso (principio de tick o resultado del paso anterior) y el **orden de consumo del generador aleatorio**.

- **Verificado contra:** `4d65075` (2026-10-11).
- **Módulos:** `src/engine/simulation.ts` (`avanzarSimulacion`, la cadena), `src/session/comandos/avanzarTick.ts` (el comando: batallas, bloqueos, salidas del mundo), `src/session/gameSession.ts` (`ejecutar`: transacción de un comando),
  `src/session/comandos/tipos.ts` (`exito`: lo que se estampa en todo comando), `src/session/estado.ts` (`instanteDeTick`, `conResultadoDeSimulacion`, `anteponerEventos`), `src/server/runnerDePartida.ts` (reloj de mundo, cola, diario, guardado),
  `src/engine/presencia.ts` (desconexión), `src/session/batallas.ts` (`vencerBatallas`, `bloqueosDe`).
- **Canon:** Doc 5.15.1 (qué bloquea una batalla), Doc 1.10.6 (presencia), Docs de Arquitectura 7 (GameSession) y 10 (modelo temporal).
- **Fuera de alcance aquí:** el contenido de cada sistema que la cadena invoca (lo dicen sus documentos: [`Economia_Construccion.md`](Economia_Construccion.md), [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md), [`Combate.md`](Combate.md)
  y los pendientes de comercio, diplomacia, tecnología y niebla), y el generador aleatorio (solo se fija *quién lo consume y en qué orden*).

## 1. Tiempo

- **1 tick = 1 minuto de mundo** (`SIMULACION.duracionTickMs` = 60 000 ms). El estado guarda `tick` (entero); **el instante de mundo se deriva, nunca se almacena**:
  `instanteDeTick(tick) = epocaInicial + tick × duracionTickMs` (en ms desde la época; `epocaInicial` es el `2026-01-01T00:00:00.000Z`). Todos los campos `*En` / `*Hasta` / `expiraEn` del juego son `Instante` en ms de mundo.
- El reloj de pared (`Date.now()`) **no entra en el estado**: dos partidas con la misma semilla y el mismo tick están en el mismo instante. `momento` es el mismo instante en ISO 8601, para fechar los eventos.
- Un plazo declarado en minutos (`*Minutos`) se convierte con `minutos(n)` = `n × 60 000` ms. Comparar siempre contra el `instante` del tick: «vence» significa `instante ≥ vencimiento`.

## 2. El reloj de mundo (`RunnerDePartida`)

Quien dispara los ticks es el *runner* de cada partida, no el motor. Es un detalle de despliegue, pero cambia cuánto dura el mundo, así que se fija:

- **Una cola serial por partida**: tick y comandos de jugador comparten la misma cola y nunca se solapan (un solo mutador a la vez).
- **`iniciarRelojDeMundo(intervaloMs)`** arranca un `setInterval(intervaloMs)` y ancla `referenciaMs = ahora` (reloj de pared). **Al arrancar no se debe ni un tick**: el mundo no avanza mientras el servidor está parado ni mientras el reloj está parado.
- En cada disparo: `adeudados = floor((ahora − referenciaMs) / intervaloMs)`.
  - `adeudados ≤ 0` → nada.
  - `adeudados > MAX_TICKS_POR_PASADA` (**5**) → el atraso **se descarta**: `referenciaMs = ahora`, no se ejecuta ningún tick y se cuenta en `omitidos`. 5 separa la deriva normal del temporizador (≤ 2 ticks, que **sí** se recupera) de una caída del proceso (decenas o cientos, que no).
  - si no → `referenciaMs += adeudados × intervaloMs` (se adelanta **antes** de encolar, y **exactamente** ese múltiplo, para no perder el resto sub-intervalo) y se encola **una sola entrada** que ejecuta `adeudados` ticks seguidos.
- Con «mundo = tiempo real», `intervaloMs` = 60 000: un tick por minuto de pared.
- Cada tick va por `aplicarYAnotar(…, guardar = true)`: se apunta una línea en el **diario** (`{v: versión, t: 'avanzarTick', a: 'sistema'}`) antes de darlo por bueno; si esa escritura falla se reconstruye la sesión desde el estado previo y se lanza.
  Después se anexan los eventos nuevos al registro de eventos de la partida y **se guarda la partida entera y se vacía el diario** (un fallo del guardado no revierte nada: el diario lo conserva). Tras cada tick del reloj se avisa a quien escucha (`alEmitir`) con los eventos del tick.
- Un comando de jugador o un tick manual (`avanzarTick()`) entran por la misma cola.

## 3. Una transacción (`GameSession.ejecutar`)

Todo cambio del estado —tick incluido— pasa por aquí, y solo por aquí:

```
ctx = { instante: instanteDeTick(estado.tick), momento, actor, rng, ids, batallasEnUnity }       ← el instante es el del tick ACTUAL (el anterior al que va a producir el comando)
mapa = fachada nueva sobre una COPIA del estado del mapa
transicion = manejador(estado, mapa, ctx, params)
si resultado.ok ∧ el comando escribió en el mapa ∧ no devolvió estadoMapa nuevo → error de programación (se lanza)
estado = transicion.estado
si resultado.version == version anterior (rechazo o «sin cambios»): se RESTAURA el generador aleatorio y el contador de ids al valor de antes
```

- La **versión** sube en **un único sitio**: `exito()`. Un comando aceptado que cambió algo sube la versión en 1; uno rechazado o sin cambios no la sube y no deja rastro (ni dados tirados, ni ids gastados).
- El comando recibe un `rng` único de la partida (`createRng(semilla)` al crear; `restaurarRng(estadoRng)` al cargar un guardado): su estado se guarda con la partida, por eso el orden de consumo (§6) importa.

## 4. El comando `avanzarTick`

Es una operación **del sistema** (actor `sistema`), no de un jugador. Siempre termina en `exito` (la versión sube siempre).

```
tick'    = estado.tick + 1
instante = instanteDeTick(tick')            ← el instante RESULTANTE: los eventos del tick se fechan con él (ctx.instante, el anterior, no se usa)

1. vencerBatallas(estado, instante):   toda batalla activa (convocando|asignada|en_curso) con instante ≥ expiraEn pasa a 'fallida' y suelta sus candados de escuadras (nadie gana ni pierde)
2. bloqueos = bloqueosDe(estado, instante):  de las batallas activas y DENTRO de plazo (instante < expiraEn): héroes que combaten, ids de ejércitos, caravanas, el asentamiento asediado y el campamento
3. simulacion = estadoSimulacionDe(estado), pero SIN los ejércitos y caravanas bloqueados («apartados»)
4. resultado = avanzarSimulacion(simulacion sin apartados, mapa, { instante, momento, rng })
5. estado'  = conResultadoDeSimulacion({ ...estado, tick: tick', estadoMapa: resultado.estadoMapa }, resultado con los apartados AÑADIDOS AL FINAL de ejercitos y de caravanas)
6. eventos de las batallas vencidas en 1: 'batalla.fallida' («no llegó a jugarse: nadie pierde nada»), con el momento del tick
7. conSalidasDelMundo(estado', mapa, instante, bloqueos.heroes)
8. exito(estado'', [eventos de la simulación, eventos de batallas fallidas, eventos de salidas])
```

Consecuencias:

- **Lo bloqueado no se simula**: un ejército o una caravana en una batalla activa no se mueve, no come ni pasa hambre, no reposta, no es perseguido ni atacado por la cadena de este tick (ni por bandidos). Cuando la batalla acaba, la cadena vuelve a verlos. Los comandos tampoco pueden tocarlos (`tocaLoBloqueado`).
- **Un asentamiento asediado sí se simula**: `bloqueos.asentamientos` se calcula pero **no filtra nada** de la cadena (Doc 5.15.1: «su economía sigue produciendo»); solo bloquea comandos y entradas/salidas.
- Los apartados se añaden **al final** de las listas: un ejército que sale de una batalla ya no ocupa su sitio original en el array. Importa solo para los pasos que iteran en el orden del array (no hay empates que dependan de él en las reglas documentadas, pero conviene saberlo).
- Una batalla que vence en este tick **no bloquea** en este mismo tick: se cierra en el paso 1 antes de calcular los bloqueos del paso 2.

### 4.1 Salidas del mundo (`conSalidasDelMundo`)

Un héroe que pidió desconectarse tiene `desconectaEn` (instante en que se le cumple la espera; `PRESENCIA.retardoDesconexionMs` = 2 min 30 s tras pedirlo). En cada tick salen los que cumplen **todo**:

```
h.desconectaEn ≤ instante  ∧  ¬h.fuera  ∧  h no combate (bloqueos.heroes)  ∧  ( instante ≥ h.desconectaEn + (topeAplazamientoMs − retardoDesconexionMs)  ∨  ¬perseguido(h) )
perseguido(h) = su columna está siendo perseguida por otro ejército (persiguiendo = {tipo:'ejercito', id: su columna})
```

Se procesan **por id ascendente**. El aplazamiento máximo es `topeAplazamientoMs` (5 min) desde que pidió desconectarse: quien huye desconectándose no gana nada. `salirDelMundo` (`engine/presencia.ts`):

- **Dentro de una plaza o un campamento**: no se mueve nada; el héroe queda `fuera` (sin carro) y la guarnición defiende como siempre. Evento `jugador.sale_del_mundo`.
- **En su columna, solo**: la columna desaparece; él se lleva escuadras y carro (`ubicacion = desconectado` en el punto de la columna, sus escuadras a `contenedor: fuera`). Las caravanas adjuntas **sin titular** (de fundación) vuelven solas a su origen por una ruta nueva (`comercio.caravana_vuelve`)
  o se pierden si no hay origen o camino (`comercio.caravana_perdida`).
- **En un ejército con más gente**: el mando pasa, si era el Líder, al **conectado de más antigüedad** (`unidoEn`, empate por id; si ninguno está conectado, al más antiguo); el héroe se separa (`desgajar`) con lo suyo y como mucho un carro (ver [`Movimiento_Ejercitos.md`](Movimiento_Ejercitos.md)); el ejército sigue sin él.
- La escolta cedida a una caravana se queda con ella.

`volverAlMundo` (al reconectar): si estaba en plaza o campamento solo recupera el control; si salió con la columna, reaparece en su punto con una columna **personal** nueva, estacionada, con sus escuadras y su carro.

## 5. La cadena `avanzarSimulacion`

### 5.1 Preparación (una vez por tick, con el estado de **principio de tick**)

```
zonas        = computeTodasLasZonas(asentamientos)          ← se calcula UNA vez; los radios que crezcan durante el tick no cambian estas zonas hasta el tick siguiente
capitales    = { faccionId → encontrarCapital }             ← ídem
reclamos     = reclamosDeFuentes(asentamientos)             ← se va actualizando según cada asentamiento compromete obra (no chocan dos asentamientos en un mismo tick)
aedasPorPlaza= residentesPorPlaza(aedasResidentes)
```

### 5.2 Por cada asentamiento, **en el orden del array** `asentamientos`

| # | Paso | Qué hace | Documento |
|---|---|---|---|
| 1 | `avanzarConstruccion` | obras, producción, recetas, mejoras, auto-construcción, obra de recintos; devuelve edificios completados, lo extraído y lo fabricado | [`Economia_Construccion.md`](Economia_Construccion.md) §6 |
| – | XP | si completó obras: `+edificiosCompletados × NIVEL_FACCION.xp.edificioCompletado` a la Facción (se aplica al final del tick, §5.4) | Facciones (pendiente) |
| 2 | `avanzarPoliticas` | expira las políticas vencidas, **después** de que la construcción las usara | Economía §12 |
| 3 | `avanzarAscenso` | termina la obra de ascenso si toca; si `nivel` subió: XP `+ascensoPorNivel × nivel nuevo` (se aplica al final) | Economía §11 |
| 4 | `avanzarNutricionPoblacion` | la población **come antes que las tropas** (los civiles producen; el shock de la escasez lo absorbe antes la parte no productiva) | Economía §9.2 |
| 5 | `avanzarMantenimientoTropas` | ración de la guarnición (`campamentoDe`, las escuadras que viven en los héroes residentes) del almacén; devuelve el campamento con moral/deserción | Combate/Militar (pendiente) |
| 6 | `crecerPoblacion` | crecimiento (consume el generador aleatorio, §6) | Economía §9.1 |
| 7 | recaudación de oro | `almacen.oro ← agregarRecurso(oro, recaudacionOro(población ya crecida))`, con tope de capacidad | Economía §9.3 |
| 8 | `avanzarMantenimiento` | paga, mueve el medidor, baja/sube `nivelActual`, o devuelve `destruido` | Economía §10 |
| 9 | fin de ventanas | si `instante ≥ ocupacionHasta` / `protegidaHasta`: se borra el campo (la guarnición instalada se queda); evento `asentamiento.ocupacion_terminada` | Combate §conquista |

Los eventos del asentamiento se concatenan en este orden: construcción, políticas, **tropas**, nivel, nutrición, población, mantenimiento, fin de ocupación. (Es el orden de los eventos en el flujo, distinto del orden de ejecución de los pasos 4–6.)
Cada evento recibe `momento` y, si no lo traía, el `asentamientoId` del asentamiento que lo produjo.

### 5.3 Cierre global, **en este orden**

```
 1. actualizados = asentamientos no destruidos
 2. por cada destruido (ruina), en orden del array:  reubicarResidentesDeRuina  →  asentamientos y campamentos de mercenarios actualizados
 3. heroes = conEscuadrones(estado.heroes, campamentos de todos los asentamientos)          ← las escuadras de la guarnición vuelven a sus héroes (moral, bajas, deserción del paso 5)
 4. por cada destruido:  facciones = registrarDerrota(facciones, actualizados, faccionId, null)       (si era la última plaza: derrotada sin ganador)
 5. por cada destruido:  cerrarDependientesDeRuina → campamento de bandidos, trueques, caravanas;  heroes = alCampamentoPorIds(heroes, escoltasLiberadas)
 6. avanzarComercio(actualizados, facciones, caravanas, acuerdos, mapa, red, zonas, instante)
        → asentamientos, facciones, caravanas, acuerdos, red, escoltasLiberadas (→ heroes al campamento), eventos
 7. mapa.avanzarRegeneracion(instante)                                    ← yacimientos agotados (Economía §5.2)
 8. avanzarSpawnBandidos(campamentosBandidos, zonas, asentamientos del comercio, mapa, instante, rng, mercenarios, facciones)     [consume rng]
 9. avanzarAtaquesBandidos(campamentos, caravanas del comercio con su escolta puesta, rng, ESTADO.ejercitos con su tropa, zonas, mercenarios)   [consume rng]
        → caravanas (se les quita la escolta), escoltasPerdidas;   heroes = conEscuadrones(heroes, tropa de las caravanas + escoltasPerdidas)
10. avanzarEjercitos(estado.ejercitos, { asentamientos del comercio, caravanas tras bandidos, facciones del comercio, relaciones, mapa, instante, zonas, red, heroes, mercenarios })
        → ejercitos, heroes, asentamientos, facciones, caravanas, campamentosMercenarios, eventos            [consume rng en combates]
11. disolverFormacionesVencidas(ejercitos, heroes, instante)           ← una formación que no llegó a tres a tiempo se deshace
12. caducarOrdenes(ordenes, instante)                                  ← las órdenes de mercado sin tomar se retiran; ya no se liquida nada en el tick
13. liberarVasallosDeSenoresDesarmados(relaciones, asentamientos de 10)  →  relaciones
14. avanzarTributos(relaciones, asentamientos de 10)                    →  asentamientos
15. aplicarExperiencia(facciones de 10, ajustesExperiencia del tick)   ← construcción y ascensos; el XP de comandos y combates ya subió el nivel en su momento
16. facciones finales = sinSolicitudesCaducadas( avanzarReputacion(facciones de 15, relaciones de 13) )
17. reponerMercados(campamentosMercenarios de 10, mercadoMercenario, instante)   ← cita agendada: cada tick solo compara un instante
18. titulos = calcularTitulos(facciones finales, asentamientos de 14, relaciones, heroes, aedasResidentes);  eventos por los cambios respecto a los del principio del tick
19. contadores = sumarContadores(tecnologia, lo extraído y fabricado en el paso 5.2.1);  avanzarTecnologia(contadores, { asentamientos de 14, facciones finales, zonas del principio, mapa, instante })
20. avanzarAedas(aedas, tecnologia de 19, {…, red})   →  tecnologia, aedas        (los itinerantes nacen en el primer tick con asentamientos)
21. avanzarResidentes(aedasResidentes, { asentamientos de 14, facciones finales, tecnologia de 20, instante })
22. avanzarCaravanasDeFundacion(caravanas de 10, ejercitos de 11, { asentamientos de 14, heroes }, facciones finales, campamentosMercenarios de 17, mapa, instante)
        → asentamientos, caravanas, ejercitos, heroes, eventos        ← sin titular vuelve sola, al llegar se desarma, suelta caduca
23. sinPrestamosAjenos(heroes, ejercitos, caravanas, campamentos)      ← la tropa prestada de un campamento donde ya no se reside, fuera
24. resultado: grabarLoVisto(memoria, …)  y  grabarExploracionPersonal(heroes, ejercitos, límites)     ← AL FINAL, con lo que ya se movió: lo que se graba es dónde acabaron las columnas este minuto
```

En el resultado, `acuerdos` se poda con `podarAcuerdosTerminados(acuerdos del comercio, caravanas de 10, instante)` y `historialOrdenes` recibe las órdenes cerradas del paso 12; `estadoMapa = mapa.estadoActual()`.

### 5.4 Qué instantánea lee cada paso

Lo que más se rompe al reimplementar es **de qué foto** lee cada paso:

| Lee el estado de **principio de tick** (`estado.*`) | Lee el resultado **encadenado** |
|---|---|
| `zonas` y `capitales` (todo el tick, aunque los radios crezcan) | `asentamientos`: construcción → … → comercio → ejércitos → tributos → fundación |
| `estado.ejercitos` en el paso 9 (los bandidos ven a las columnas **antes** de moverse) y como entrada del paso 10 | `heroes`: guarnición (3) → liberadas de ruina (5) → comercio (6) → bandidos (9) → ejércitos (10) → formaciones (11) → fundación (22) → préstamos (23) |
| `estado.relaciones` en los pasos 10 y 13 | `caravanas`: ruina (5) → comercio (6) → bandidos (9) → ejércitos (10) → fundación (22) |
| `estado.ordenes`, `estado.mercadoMercenario`, `estado.titulos`, `estado.aedas`, `estado.aedasResidentes`, `estado.memoriaPorFaccion` | `facciones`: derrota (4) → comercio (6) → ejércitos (10) → XP (15) → reputación (16) |
| `tecnologia`: `estado.tecnologia` + contadores del tick (19) | `relaciones`: la liberación (13) alimenta los tributos (14), la reputación (16) y los títulos (18) |

Observaciones concretas: el paso 8 recibe las `facciones` anteriores al comercio (no `trasComercio.facciones`; solo las lee); el paso 10 recibe `facciones` del comercio y **no** las de la ruina; la tecnología (19) usa las zonas del principio del tick: un tick de retraso no cambia ningún hito.

## 6. Orden de consumo del generador aleatorio

Un tick consume el generador de la partida **solo** en estos puntos, y en este orden (cualquier otro orden cambia toda la partida a partir de ahí):

1. `crecerPoblacion`, por asentamiento en el orden del array: Pesants (una tirada siempre), Artesanos (una tirada solo si hay taller activo, hay hueco y ya hay artesanos), Nobleza (ídem) — [`Economia_Construccion.md`](Economia_Construccion.md) §9.1.
2. `avanzarSpawnBandidos` (nivel al azar de un campamento nuevo, punto de un anillo).
3. `avanzarAtaquesBandidos` (varianza de combate).
4. `avanzarEjercitos`: solo al **resolver** un choque, una intercepción, un campamento o un asedio contra una plaza defendida. Una partida sin ejércitos en combate hace exactamente las mismas llamadas, en el mismo orden, que antes de existir la mecánica.

Las funciones que construyen cosas del mundo con una semilla propia (`createRng(mapa.seed ^ sal)`: campamentos de mercenarios, alijos) **no** tocan el generador de la partida.

## 7. Lo que `exito` estampa en todo comando (tick incluido)

1. `version = version + 1`.
2. **Épicas de los Aedas residentes** (`avanzarEpicas`) con los eventos del tick/comando, sobre el estado resultante; sus eventos se añaden a los hechos.
3. **Crónica** (`entradasDeCronica(hechos, estado)`): entradas narradas a partir de los hechos.
4. A cada evento se le asigna la `version` y se **antepone** a `estado.eventosDominio` (más nuevo primero). El log en memoria se recorta a **`INFRA.eventosMaximosEnMemoria` = 5 000** cortando siempre entre dos versiones (el evento más viejo que se conserva es de una versión completa);
   el historial completo vive en el JSONL de la partida.
5. **Logros del servidor** (Doc 6.3): `tecnologia = sumarContadores(tecnologia de las épicas, contadoresDeEventos(eventos))`. Lo que no deja evento (extracción, talleres) ya lo contó el tick en el paso 19 de §5.3.
6. **Mercados de mercenarios**: `mercadoMercenario = sumarComerciado(…, comerciadoDeEventos(eventos))` (lo comerciado entre Facciones alimenta la reposición, Doc 1.9b).

El `momento` de estos pasos es `instanteDeTick(estado.tick)` del estado **ya** avanzado (el tick nuevo).

## 8. Parámetros

<!-- parametros:SIMULACION -->
```json
{
  "epocaInicial": "2026-01-01T00:00:00.000Z",
  "duracionTickMs": 60000
}
```
<!-- /parametros -->

<!-- parametros:PRESENCIA -->
```json
{
  "retardoDesconexionMs": 150000,
  "topeAplazamientoMs": 300000
}
```
<!-- /parametros -->

Cifras de **infraestructura** (viven en `constantsInfra.ts`, no en `constants.ts`: no son reglas del juego y no alteran el resultado de una simulación). `ticksMaximosPorPasada` es el `MAX_TICKS_POR_PASADA` de arriba; `limiteIdempotencia`, las claves que recuerda `RunnerDePartida`; `eventosMaximosEnMemoria`, el recorte del log en memoria.

<!-- parametros:INFRA -->
```json
{
  "ticksMaximosPorPasada": 5,
  "limiteIdempotencia": 500,
  "eventosMaximosEnMemoria": 5000
}
```
<!-- /parametros -->
Las constantes de XP (`NIVEL_FACCION.xp.edificioCompletado`, `ascensoPorNivel`) son de Facciones.

## 9. Invariantes

| Invariante | Dónde se vigila |
|---|---|
| Misma semilla ⇒ mismo estado y mismos eventos tras N ticks; semilla distinta ⇒ resultado distinto | `determinismo.test.ts` |
| Mismo comando + misma semilla con relojes de pared distintos ⇒ snapshot y eventos idénticos; los eventos se fechan con el instante de mundo | `runnerDePartida.test.ts` |
| Un reloj parado o recién arrancado no ejecuta ticks; un atraso grande se descarta y se cuenta | `runnerDePartida.test.ts` |
| Almacén ∈ [0, capacidad], población ≥ 0, medidor ∈ [0, 100], `nivel` monótono, reputación ∈ [−100, 100] a lo largo de 300 ticks | `invariantes_simulacion_larga.test.ts` |
| Un comando rechazado o sin cambios no sube la versión ni mueve el generador aleatorio ni los ids | `gameSession.test.ts` |
| Un ejército o caravana en una batalla activa no cambia durante el tick | `batallas.test.ts`, `entradaEnBatalla.test.ts` |
| Quien combate no sale del mundo hasta que acaba la batalla | `presenciaEnElMundo.test.ts` |

## 10. Casos límite y decisiones que parecen raras

- **El tick se fecha con el instante resultante**, no con el que traía el contexto del comando (que es el del tick anterior).
- **Un tick siempre sube la versión**, aunque no pase nada: es un cambio de estado (`tick`).
- **`bloqueos.asentamientos` y `bloqueos.campamentos` no filtran la cadena**: solo los ejércitos y las caravanas se apartan. Un asedio no congela la economía de la plaza.
- **Una ruina entre el cierre de asentamientos y el comercio**: la plaza destruida ya no existe para el comercio, los bandidos ni los ejércitos de este mismo tick.
- **La guarnición se escribe en el héroe** (paso 3 de §5.3) **antes** de que bandidos y ejércitos lean `heroes`: los combates de este tick ven la moral y las bajas por hambre de la guarnición de este mismo tick.
- **Los bandidos ven a los ejércitos antes de que se muevan** (`estado.ejercitos`), pero a las caravanas ya movidas por el comercio.
- **La memoria de la niebla se graba al final**: una Facción recuerda lo que vio al *terminar* el tick, no en mitad.
- **Atrasos de reloj**: un host dormido o un salto del reloj de pared no se «recupera»; el mundo se queda donde estaba (decisión del autor, 2026-09-05).
- **Reabrir una partida** no ejecuta ni un tick atrasado: `referenciaMs` se ancla a «ahora» al arrancar el reloj.

## 11. Divergencias

Ninguna abierta.
