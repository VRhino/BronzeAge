# Movimiento de ejércitos y Liderazgo — cierre de diseño y plan de ejecución

> **Estado (2026-09-02): diseño cerrado, CERO código escrito.** Mismo criterio que `Murallas_Definicion.md`:
> la especificación se escribe ANTES de tocar el motor. 17 decisiones cerradas con el usuario + revisión por
> consejo incorporada (§9). **Nada bloquea el arranque**; la única decisión abierta (§1.4, la varianza de
> combate) no afecta a los Pasos 1-8.
>
> **Las REGLAS DE JUEGO de esta mecánica no viven aquí: viven en `Docs/Game/`**, que es el canon.
> Concretamente `Docs/Game/5_Sistema_Militar_y_Combate.md` **§5.11 (Liderazgo)**, **§5.12 (Ejércitos y
> movimiento por el mapa)** y **§5.13 (Suministro en campaña)**, más las entradas *Escuadrón/Tropa/Unidad*,
> *Ejército*, *Guarnición* y *Liderazgo* del glosario (`Docs/Game/0_Glosario_de_Entidades_Politicas.md`).
> Doc 5.4 recoge el cambio de propiedad (los escuadrones son del Jugador) y Doc 5.10 aclara que el movimiento
> en el mapa SÍ es Fase 0.
>
> **Este documento cubre solo lo que no es regla de juego**: qué se decidió y cuándo, cómo se representa en
> el motor, qué código hay que tocar, en qué orden, y qué se congela en tests. El enunciado original es
> `Docs/Mecanicas a desarrollar.md` §2.

## 1. Decisiones

### 1.1 Cerradas con el usuario (2026-09-01)

Las siete que estaban abiertas tras el análisis, con su respuesta. La regla resultante está en `Docs/Game/5`;
aquí queda el registro de qué se preguntó y qué se respondió.

1. **El liderazgo limita lo que SALE, no lo que se posee.** La guarnición no tiene tope de liderazgo.
2. **"Dispersión" y "deserción" son lo mismo** — el usuario lo expresó mal la primera vez. **No hay estado
   nuevo**: quedarse sin comida en marcha produce la misma deserción que ya existe. Esto es lo que permite
   que el carro sea un segundo llamador de la función del hambre y no un subsistema paralelo (§2.3).
3. **Carro de suministros**: capacidad fija por jugador, sumada al formar ejército; se carga del asentamiento
   al salir o al unirse; se reabastece en aliado si ese asentamiento tiene la opción activa; el sobrante
   vuelve al asentamiento.
4. **Asentamiento conquistado**: el jugador pierde las tropas que estaban en la ciudad y conserva las que
   lleva encima.
5. **Un jugador SÍ puede unirse a un ejército ya en campo.**
6. **`estacionado` es un estado válido**, con consumo reducido pero nunca 0.
7. **El coste de liderazgo depende del poder**: a mayor poder, mayor coste.

### 1.1b Cerradas tras la revisión por consejo (2026-09-02)

Nueve decisiones más, todas respuesta a hallazgos del consejo (§9) o ampliación del usuario (la 17 va aparte, en §1.3):

8. **Caravanas adjuntas al ejército**, que amplían la carga más allá de la suma de los carros de sus jugadores.
9. **La capacidad de una caravana debe ser igual o mayor que la del carro de un jugador** — "si no, qué sentido tendría la caravana per se". Obliga a rebalancear `CARAVANA_CATALOGO` (§3).
10. **Escolta de caravanas: SÍ.** Era una mecánica pendiente sin implementar (Doc 3.10) y esto la resuelve: una caravana adjunta puede ir cargada y entregar mientras marcha protegida.
11. **Si el ejército es derrotado, se pierden las caravanas adjuntas.** El tren de suministros es objetivo militar.
12. **El carro es de capacidad FIJA e igual para todos los jugadores** — "es un carro". Descarta la versión proporcional a la tropa que se había propuesto para matar el exploit de las mulas; §9 explica por qué con carro fijo la mula deja de ser dominante igualmente.
13. **Velocidad por tropa**: ligera 20 / media 16 / pesada 12, bajo dos reglas del usuario — una caravana inicial no puede ser más rápida que un ejército, y un jugador solo con infantería ligera tiene que poder alcanzar una caravana inicial.
14. **Cancelar una marcha en curso dispara la vuelta** (pasa a `regresando` y desanda la ruta).
15. **El jugador cuyo asentamiento cae estando de campaña queda HUÉRFANO** hasta entrar en una Facción que tenga asentamiento.
16. **Incentivo de conquista**: un asentamiento completo + ampliar los asentamientos de la Facción **por encima del cupo de su nivel**. Cierra el hallazgo del consejo de que la guerra era suma negativa — y resulta que **ya es el comportamiento implementado**: `iniciarAsedio` conserva el nivel del conquistado sin verificar cupo, anotado en el código como "punto abierto #2" a la espera de que alguien decidiera si era intencional. Lo era.

### 1.1c Cerrada al revisar el impacto del TIEMPO REAL (2026-09-02)

18. **Un ejército con todos sus escuadrones a cero se disuelve, y sus identidades vacías vuelven al
    asentamiento de origen**, que es donde se pueden rellenar reclutando.

Salió de una pregunta del usuario: *"el servidor no es del todo tick ya, el avance a tiempo real se ha hecho,
¿esto afecta al plan?"*. Sí — y destapó este hueco. Con 1 tick = 1 minuto real, un ejército cuyo jugador no
vuelve consume durante horas; al vaciarse el carro la deserción lo lleva a cero, pero **el escuadrón persiste
como identidad aunque se quede sin unidades** (Doc 5.4). Sin regla explícita quedaba un ejército fantasma: 0
soldados, 0 ración (ya no pasa hambre), marchando para siempre y atacando con poder 0. Y **no lo atrapaba la
guarda existente**: `resolverCombate` rechaza si el array de atacantes está VACÍO, no si tiene escuadrones a
cero. Regla en Doc 5.13.4.

### 1.1d Cerrada (2026-09-02): visibilidad de ejércitos ajenos

19. **Un jugador ve los ejércitos ajenos que entren en su ZONA DE INFLUENCIA.**

Bloqueaba el Paso 5 (proyección + render del cliente de jugador). Las tres opciones que se plantearon: no ver
nada (coherente con el Slice 1, pero desactiva la tensión — un ejército enemigo tarda ~50 minutos reales en
llegar y no podrías reaccionar), ver por zona de influencia, o ver por radio de visión (que es lo que §12
pide de verdad, pero **necesita un número que sigue sin decidirse**).

Se elige la zona de influencia porque **usa `zonas`/`zonasFusionadas`, que ya existen y ya se proyectan**: da
capacidad de reacción sin inventar ninguna constante, y no compromete §12 — cuando llegue el radio, sustituye
a este criterio sin romper el contrato.

**Consecuencia obligatoria**: el ejército ajeno se proyecta REDACTADO. Los rombos necesitan el número de
participantes, que sale de los escuadrones — pero mandar los escuadrones de un ejército enemigo filtraría su
composición y su poder. Se manda posición, facción y **recuento de participantes**, nunca el detalle de tropas.

### 1.2 Derivadas — no se preguntaron porque las de arriba o los invariantes vigentes ya las obligan

- **El ejército NO cambia de bando al caer su hogar** (de 1.1 punto 4: conserva "las que tiene encima").
- **La guarnición derrotada se PIERDE, no se transfiere al conquistador.** Interpretación de "pierde las
  tropas" bajo el modelo de propiedad por jugador: son tropas personales de otro jugador, no botín. **Esto
  cambia el comportamiento actual**, donde `iniciarAsedio` conserva `defensor.escuadrones` y solo cambia
  `faccionId` — es decir, hoy el conquistador hereda las tropas personales del defensor.
- **Un ejército sin hogar se re-adopta**: si su asentamiento de origen cae o se destruye,
  `origenAsentamientoId` pasa al asentamiento propio más cercano. Sin ninguno, el ejército sigue existiendo
  hasta agotar su suministro — una Facción reducida a un ejército errante es un desenlace legítimo, no un
  estado inválido que haya que impedir.
- **El ejército de jugadores huérfanos no tiene a dónde volver.** Se sigue de 1.1b punto 15: un huérfano no
  tiene residencia (Doc 2.5 exige fundar o comprar casa, y conquistar la ciudad no le da ninguna de las dos),
  así que no basta con que su Facción conserve otros asentamientos. Mientras TODOS los jugadores de un
  ejército estén huérfanos, ese ejército no puede replegarse ni reabastecerse en propio: solo moverse,
  combatir y consumir. Es el mismo desenlace de arriba visto desde el jugador.
- **Unirse en campo exige proximidad** (mismo criterio que el reabastecimiento y que los encuentros). Sin
  esto, unirse sería teletransportar refuerzos.
- **Reabastecerse en asentamiento propio siempre se puede**; la opción activable regula solo a los aliados.
- **Salir solo y salir en ejército son un solo tipo** (§2.2).

### 1.3 Cerrado (2026-09-02): el coste de liderazgo se DERIVA

17. **`costeLiderazgo = poderBase × unidades × 0.2`**, no un campo escrito a mano en el catálogo.

Este punto estuvo abierto un tiempo porque los primeros números que dio el usuario (milicia 10, honderos 15,
lanceros de mimbre 30) contradecían su propia regla de "a mayor poder, mayor coste": con ellos, los lanceros
de mimbre (poderBase 3) costarían el doble que los honderos (poderBase 5), que dominarían estrictamente y
harían que nadie sacara mimbre jamás. **El usuario confirmó que eran ejemplos para explicar la mecánica, no
valores reales.** La regla manda; la tabla de `Docs/Game/5` §5.11.1 es la buena.

Se deriva en vez de escribirse a mano por una razón concreta: `poderBase` sigue siendo PLACEHOLDER pendiente
de calibración (Doc 5.8), y once números escritos a mano se desincronizarían del poder en cuanto se calibre.
Una fórmula no. Lo que se calibra es **el factor**, un solo número.

**Con esto no queda ningún punto bloqueando el arranque.** Lo que sigue abierto (§7) es deliberado y no
impide empezar.

### 1.4 ABIERTA (2026-09-02): la varianza de combate

**Pregunta del usuario: ¿por qué se usa RNG en este desarrollo?** La respuesta corta es que **esta mecánica no
introduce ni una sola llamada nueva**: toda la aleatoriedad es preexistente y solo cambia de sitio (del comando
síncrono al tick, que es lo que hace que el orden importe, §9.1 punto 6).

Consumos de `rng()` en todo el motor — seis, y cinco son la misma línea:

| Sitio | Qué hace |
|---|---|
| `combate.ts:97-98` | jitter ±15% al poder de atacante y defensor |
| `combate.ts:291` | ídem, interceptar caravana |
| `combate.ts:366` | ídem, atacar campamento de bandidos |
| `bandidos.ts:125` | ídem, bandidos atacando una caravana |
| `population.ts:29` | redondeo estocástico del crecimiento de población |

El sexto **se queda y no se discute**: si la población crece 3.4 habitantes, da 3 o 4 con la probabilidad
correcta; sin él, redondear sesgaría el crecimiento sistemáticamente. Los otros cinco son `MILITAR.varianzaCombate`.

**Por qué se replantea ahora.** El código justifica el jitter como "romper empates", pero ±15% por bando
significa que **un atacante necesita un 35% más de poder para tener la victoria garantizada** (Doc 5.2.5). Eso
es varianza real. Y esta mecánica **multiplica el coste de una mala tirada**: antes perdías unos escuadrones
tras pulsar un botón; ahora marchas 30-50 minutos reales, dejas la ciudad indefensa todo ese tiempo, gastas
500 de trigo, quizá llevas la flota comercial enganchada, y las bajas son permanentes. Cuanto mayor el
compromiso, peor sienta el azar — y en Fase 0 el combate **no se ve** (es cálculo y log de texto), así que un
swing del 15% no se lee como tensión sino como "el juego me quitó el ejército".

**El argumento de fondo:** ya hay diseñada una fuente de incertidumbre mejor. La niebla de guerra
(`Mecanicas a desarrollar.md` §12) hace que no sepas qué guarnición hay en casa — incertidumbre **reducible
jugando bien** (explorar, mantener contacto), diegética, y que no cuesta determinismo. El jitter es
incertidumbre que no puedes reducir por mucho que juegues. El contraargumento clásico ("sin azar nadie ataca
salvo que ya haya ganado sobre el papel") **solo aplica con información perfecta**, que es justo lo que la
niebla elimina: el RNG está haciendo un trabajo que la niebla hace mejor.

**Lo que se gana técnicamente al quitarlo:** `avanzarEjercitos` pasaría a ser mudo de RNG **siempre**, no solo
cuando no hay ejércitos. Desaparece el hallazgo del orden canónico (§9.1 punto 6), el guardián de determinismo
queda trivialmente a salvo, y **la calibración por batch se vuelve más barata y nítida** — hoy la varianza
obliga a más corridas para ver señal, y de ahí salen las constantes del juego.

**Tres opciones:**

1. **Dejarlo en 0.15.** Coherente con lo ya calibrado, pero con el coste de compromiso multiplicado.
2. **Bajarlo a ~0.03-0.05.** Sigue rompiendo empates y modelando fricción, pero una ventaja del 20% ya gana
   casi siempre. Los encuentros PvE no se vuelven una tabla de consulta.
3. **Retirarlo del combate entre Facciones** y dejar que la incertidumbre venga de la niebla de guerra.

**Recomendación: la 2 ahora, la 3 cuando la niebla de guerra exista.** Quitar el azar antes de tener la
incertidumbre por información dejaría un hueco donde nadie ataca sin certeza. Aviso honesto sobre la 3:
`interceptarCaravana` y `atacarCampamentoBandidos` comparan contra un umbral fijo, así que sin jitter se
vuelven completamente predecibles — o siempre ganas o siempre pierdes. Para PvE puede estar bien, pero es un
cambio de sensación que hay que querer.

**No bloquea el arranque**: los Pasos 1-8 no tocan la varianza. Conviene cerrarla antes del Paso 10
(encuentros por proximidad), porque si sale la opción 3 ese paso ya no necesita orden canónico de resolución.

## 2. Representación en el motor

### 2.1 `Jugador` — entidad nueva

Hoy **no existe**: el jugador es solo un `jugadorId: string` repartido por `Escuadron`,
`jugadoresFundadoresIds`, `casasCompradas`, `cargos`, `historialJugadores` y `salidasFaccionPorJugador`.
`engine/combate.ts:143` lo dice explícitamente: no se guarda estado de jugador porque "exigiría una entidad
`Jugador` que hoy no existe en el motor; queda pendiente **si llega a necesitar un propósito propio**".

**El liderazgo es ese propósito**, y al crearla se abre parcialmente `Docs/Mecanicas a desarrollar.md` §11
(*progreso jugador y tropa*).

```ts
interface Jugador {
  id: string;
  /** Liderazgo BASE. El efectivo es base + progresión; hoy no hay progresión, así que coinciden. */
  liderazgoBase: number;
}
```

Vive en `GameSessionState.jugadores`. Partidas guardadas: `[]` por defecto, y un jugador sin registro usa
`LIDERAZGO.base` — ninguna partida vieja se rompe ni necesita migración real.

### 2.2 `Ejercito`

```ts
interface Ejercito {
  id: string;
  faccionId: string;
  /** De dónde salió y a dónde vuelve. Se reasigna si cae (§1.2). */
  origenAsentamientoId: string;
  /** Escuadrones MOVIDOS aquí desde el asentamiento. Cada uno lleva su `jugadorId`. */
  escuadrones: Escuadron[];
  /** Los carros de los jugadores, ya sumados. Solo trigo en Fase 0. */
  suministro: Record<string, number>;
  /** Caravanas que marchan con el ejército (§1.1b puntos 8-11): amplían la carga, entran en el `min` de
   * velocidad, pueden ir cargadas de mercancía (escolta) y se pierden si el ejército es derrotado. */
  caravanasAdjuntasIds: string[];
  objetivo: { tipo: 'asentamiento'; id: string } | { tipo: 'punto'; punto: Point };
  ruta: Point[];
  progreso: number;
  posicionActual: Point;
  estado: 'marchando' | 'estacionado' | 'regresando';
}
```

**Un solo tipo para las dos formas de salir** (regla en Doc 5.12.1). Los participantes se derivan con
`new Set(escuadrones.map(e => e.jugadorId))`: tamaño 1 = salió solo, tamaño 4 = ejército de cuatro. No se
guarda lista de participantes, y ese mismo número es el de rombos a dibujar.

**Lo que NO se guarda porque se deriva**: facción y color (`origenAsentamiento.faccionId`, igual que ya hace
el triángulo de caravana en `cliente/src/ui/canvas.ts:554`), nº de rombos, poder (`poderTotal`, ya existe),
capacidad del carro (nº de participantes × constante) y velocidad (`min` sobre los escuadrones).

### 2.3 El hambre se escribe UNA vez

Hoy `avanzarMantenimientoTropas(asentamiento)` (`engine/tropas.ts:137`) mezcla tres cosas: de dónde sale el
trigo, la curva de moral, y qué pasa a moral 0. Se extrae el medio:

```ts
// engine/tropas.ts — núcleo puro: no sabe de asentamientos ni de ejércitos
avanzarRacion(escuadrones, trigoDisponible, factorConsumo = 1)
  -> { escuadrones, trigoConsumido, eventos }
```

- **Guarnición**: envoltorio con `asentamiento.almacen.trigo`, factor 1.
- **Ejército marchando**: envoltorio con `ejercito.suministro.trigo`, factor 1.
- **Ejército estacionado**: lo mismo con `LOGISTICA.factorConsumoEstacionado`.

Como deserción y "dispersión" son lo mismo (§1.1 punto 2), la función **no necesita parámetro de
consecuencia**. Misma curva, mismas constantes `MILITAR`, mismo evento `tropas.desercion`, un solo sitio.

`consumoRacionTropas(asentamiento)` **no se toca**: al irse las tropas de verdad, ya cuenta solo la
guarnición, que es exactamente lo que la regla pide.

### 2.4 Movimiento y combate: la maquinaria ya está

- **Movimiento**: `calcularRuta` (`world/rutas.ts`) + `avanzarPosicionEnRuta` (`engine/movimiento.ts`), las
  mismas dos funciones que ya mueven caravanas. `engine/movimiento.ts` existe precisamente porque
  `trade.ts` y `expansion.ts` duplicaban la interpolación: el precedente del proyecto es **extraer la
  función, no inventar un supertipo**. Un `EntidadMovil` común a caravana y ejército sería abstracción por sí
  misma — comparten tres campos y una llamada, y difieren en todo lo demás.
- **Encuentros por proximidad**: patrón ya probado de `avanzarAtaquesBandidos` (`engine/bandidos.ts:113`),
  que recorre móviles y dispara si `distancia <= radio`.
- **Combate**: `resolverCombate` (`engine/combate.ts:87`) **ya recibe escuadrones planos** y no sabe nada de
  asentamientos. Solo los cuatro envoltorios están atados a `Asentamiento`. Ejército-vs-ejército y
  ejército-vs-ciudad reutilizan el núcleo sin tocarlo; lo que se escribe son envoltorios delgados.

### 2.5 Dónde encaja en el tick

`avanzarEjercitos` va en la fase global de `avanzarSimulacion`, **justo después de `avanzarCaravanasFundacion`
y antes de los bandidos**: misma fase "los móviles se mueven y resuelven", con los asentamientos ya en su
estado post-tick.

> **Esto mueve el combate al tick, y con él el consumo de RNG.** Hoy el combate es un comando síncrono
> (`session/comandos/militar.ts`); con movimiento, la llegada lo resuelve dentro del tick. El tick ya consume
> `rng` (bandidos), así que encaja en el contrato — pero **el orden de consumo del RNG forma parte del
> determinismo**. Hay que actualizar `__tests__/determinismo.test.ts` y asumir que las partidas guardadas
> divergen a partir del cambio.

### 2.5b Qué implica que el servidor ya corra en TIEMPO REAL

Pregunta del usuario (2026-09-02). El reloj de mundo de Fase D ya está: `SIMULACION.duracionTickMs` = 60.000,
o sea **1 tick = 1 minuto real**, y `RunnerDePartida` ejecuta los ticks adeudados en ráfaga
(`MAX_TICKS_RAFAGA` = 10.080, siete días).

**Lo que NO cambia — el modelo de movimiento.** El Doc 2 preveía derivar la posición de "salida + velocidad +
tiempo transcurrido", pero ese objetivo (independencia del tamaño de paso) **se midió imposible y se retiró en
Fase D**. Además, con coste de terreno el movimiento es dependiente del CAMINO: no se puede derivar la
posición del tiempo transcurrido sin recorrer la ruta, porque el coste varía a lo largo de ella. Los ejércitos
acumulan progreso por tick igual que las caravanas, y no hay alternativa mejor.

**Descartado: programar la llegada como evento futuro** en vez de sondearla cada tick, aprovechando que D5
dejó la infraestructura de comandos programados pendiente "para su primera mecánica de Fase 1". El ejército
necesita trabajo por tick de todas formas (comer del carro, moral, encuentros por proximidad), así que
programar la llegada no eliminaría esa pasada: solo añadiría un segundo mecanismo sin ahorrar nada.

**Lo que sí cambia, y está recogido en los pasos y riesgos:**

1. **Rendimiento en ráfaga** — riesgo nuevo en §8, a medir en el Paso 10.
2. **El Paso 5 gana una verificación mucho mejor**: con el reloj de dev a 5 s/tick un ejército cruza el mapa
   en el navegador en un par de minutos, así que el render verifica ruta, velocidad y consumo de verdad y no
   una captura de un rombo quieto.
3. **El Paso 8 (reabastecimiento) pasa de accesorio a load-bearing**: estacionado consume a 0.5×, o sea que un
   ejército aparcado quema su carro en ~100 minutos reales. "Sostener un paso de montaña" no existe como
   posición hasta que se pueda reabastecer.
4. **La crítica del consejo sobre la asincronía deja de ser teórica** (§9.3): ausentarse 8 horas son 480
   ticks, unas 10 veces la autonomía completa de un ejército.

### 2.6 La superficie de comandos se encoge

`combateCampoAbierto` e `interceptarCaravana` **dejan de ser comandos de jugador** y pasan a ser
resoluciones del motor disparadas por geometría (Doc 5.12.3). `movilizarEjercito`, `unirseAEjercito` y
`replegarEjercito` sustituyen a tres comandos de combate.

### 2.7 Terminología: NO se renombra nada (decisión 2026-09-02)

Se evaluó renombrar `Escuadron` → `Tropa` para alinear el código con el vocabulario del usuario, y **se
descartó**. El código actual ya es correcto y consistente:

| Concepto | Palabra | Dónde vive en el código |
|---|---|---|
| El tipo ("Milicia de lanceros" como clase) | **tropa** | `TROPAS_RECLUTABLES`, `Escuadron.tropaId` |
| La instancia que el jugador posee | **escuadrón** | `Escuadron`, `poderEscuadron`, `asentamiento.escuadrones` |
| El soldado individual | **unidad** | `Escuadron.cantidad`, `unidadesPorDefecto` |

Tres razones para no tocarlo:

1. **"Tropa" ya nombra al tipo.** `Escuadron.tropaId` apunta a `TROPAS_RECLUTABLES` y se lee exactamente como
   lo que es. Renombrar la instancia a `Tropa` obligaría a un compuesto (`tipoTropaId`) y dejaría
   `TROPAS_RECLUTABLES` con un nombre engañoso: gastaríamos la palabra buena en el concepto que ya tenía
   nombre.
2. **"Escuadrón" es contable y "tropa" es colectivo.** El Doc 5 está lleno de frases en sentido colectivo
   ("las tropas consumen raciones", "mantenimiento de tropas"); si "tropa" pasara a ser una instancia
   contable, todas se volverían ambiguas.
3. Se pasaría de **tres palabras inequívocas a dos más un compuesto** — un paso atrás en claridad, a cambio
   de 242 cambios en 15 archivos y el riesgo de un rename masivo antes de escribir la mecánica.

El problema real nunca fue el nombre en el código: era que **Doc 5.8 definía "tropa" como el tipo mientras el
glosario no distinguía los tres niveles**. Arreglado en documentación (Doc 0 + Doc 5.8), sin tocar una línea.

El invariante "un jugador tiene muchos escuadrones, **no más de uno de cada tropa**" **ya está implementado**:
`reclutarTropa` empareja por `jugadorId + tropaId` y repone en vez de crear un segundo
(`engine/tropas.ts:52`).

## 3. Lo que hay que tocar

| Sitio | Cambio |
|---|---|
| `engine/tropas.ts` | extraer `avanzarRacion` (§2.3); los envoltorios de guarnición quedan equivalentes |
| `consumoRacionTropas` | **sin cambios** — al irse la tropa de verdad, ya cuenta solo la guarnición |
| `engine/titulos.ts:44` | "Ejército más grande" debe sumar **guarnición + ejércitos en campo**, o el título baila cada vez que alguien marcha y los Aedas narran un `titulo.cambia_manos` falso |
| `session/comandos/autorizacion.ts:133` | `comandaEscuadrones` debe resolver también escuadrones dentro de un ejército, para los comandos sobre ejércitos ya en campo |
| `engine/combate.ts:174` | **rama nueva obligatoria**: `seleccionarEscuadrones` *lanza* si la lista queda vacía, así que una ciudad sin guarnición haría **fallar** el asedio en vez de caer. Sin defensores → conquista automática, sin combate ni bajas |
| `engine/combate.ts` (conquista) | dejar de heredar la guarnición al conquistador (§1.2) |
| `engine/simulation.ts` | `avanzarEjercitos` en la fase global; actualizar `determinismo.test.ts` (§2.5) |
| `domain/types.ts` | `Asentamiento.permiteReabastecerAliados?: boolean` (ausente = false) |
| `session/npcGobernanza.ts` | migrar de ataque instantáneo a marcha — **paso propio con diario de batch** |
| `server/persistenciaPartida.ts` | `ejercitos: []`, `jugadores: []` por defecto |
| `session/proyecciones/jugador.ts` | ejércitos propios completos; ajenos, nada por ahora (§6) |
| `cliente/src/ui/canvas.ts` | rombos + traza, espejo del bloque de caravanas (líneas 530-567) |
| `constants.ts` `TROPAS_RECLUTABLES` | campo `velocidad` por tropa (Doc 5.12.5) |
| `constants.ts` `CARAVANA_CATALOGO` | rebalance de capacidad a >= el carro de un jugador (§1.1b punto 9) — **cambia el oro por entrega**, medir en batch |
| `engine/trade.ts` | una caravana adjunta a un ejército se mueve CON él, no por su cuenta; al entregar, comisión normal (escolta, Doc 5.13.3) |
| `engine/combate.ts` | derrota del ejército ⇒ se pierden las caravanas adjuntas |
| `domain/types.ts` | `Jugador` sin residencia = huérfano (Doc 5.4); el estado tiene que ser representable |

## 4. Constantes nuevas (todas PLACEHOLDER, a calibrar por simulación)

```ts
export const LIDERAZGO = {
  base: 50,                 // liderazgo de un jugador sin progresión (número del usuario)
  factorCoste: 0.2,         // coste = poderBase × unidades × este factor (anclado a milicia = 10)
};

export const LOGISTICA = {
  // Capacidad FIJA e igual para todos (§1.1b punto 12). NO es un número elegido: sale del radio operativo
  // objetivo (Doc 5.13.1) — 70 soldados × 0.15 ración × 50 ticks = 525, redondeado.
  // Si se rebalancea la ración o la producción de trigo, RECALCULAR desde el radio, no ajustar a ojo.
  capacidadCarroPorJugador: 500,
  autonomiaTicksObjetivo: 50,           // el invariante de diseño del que sale lo de arriba
  factorConsumoEstacionado: 0.5,        // reducido, nunca 0
  radioReabastecimiento: 60,            // proximidad para repostar y para unirse en campo
  radioEncuentro: 60,                   // proximidad para chocar en ruta
};

// añadido a TROPAS_RECLUTABLES: `velocidad` por tropa (Doc 5.12.5)
// ligera 20 — milicia_lanceros, honderos, lanceros_mimbre, escaramuzadores_jabalina
// media  16 — espadachines_cobre, espadachines_bronce, hacheros_ligeros, arqueros
// pesada 12 — hacheros_armados, lanceros_pesados, arqueros_compuesto

// REBALANCE de CARAVANA_CATALOGO (§1.1b punto 9): hoy capacidad 20/40/60/150, que está un orden de magnitud
// por debajo del carro de un solo jugador (500). Una caravana debe cargar >= 500. Ver el riesgo en §8:
// la comisión es `valorTotal × tasa × distancia × reputación`, así que multiplicar la capacidad multiplica
// el oro por entrega en la misma proporción — hay que medirlo en batch antes de fijarlo.
```

Sobre la velocidad: la regla del canon es `min` sobre los escuadrones (Doc 5.12.5), y desde el Paso 2 **cada
tropa tiene la suya de verdad** — la idea inicial de una `velocidadEjercitoBase` plana se retiró en cuanto el
usuario dio la tabla real. Calibrarla es un cambio de **datos**, no de código.

## 5. Plan de ejecución

- [x] **Paso 0 — Rebalanceo de producción de trigo (2026-09-02).** Añadido por decisión del usuario tras
      medir §9.4. Producción base de Granja ×2 en todos los niveles (15/22.5/30/45 → 30/45/60/90),
      `BALANCE_VERSION` 3→4, palanca `BATCH_TRIGO_X` añadida a `scripts/run-batch-sim.ts` para poder repetir
      el experimento. Destapó y cerró de paso el bug §E6.16 del trazado (edificios sobre calle), que la suite
      no alcanzaba hasta que las ciudades crecieron. 786/786 tests, `tsc` limpio.

> **Reordenado tras la revisión por consejo (§9).** Tres cambios sobre el orden original: el refactor del
> hambre sube al primer puesto (es riesgo cero y desbloquea todo), el render sube justo detrás del movimiento
> (el lab es el instrumento de medida, no un adorno: sin él los pasos siguientes se depuran a ciegas), y la
> carga del carro desde el almacén pasa a ser **paso propio** en vez de viajar dentro de otro — si se hunde el
> batch, hay que poder saber si fue la logística o la IA.

- [x] **Paso 1 — `avanzarRacion` extraída (2026-09-02).** Refactor puro: la curva de moral/deserción deja de
      estar atada al asentamiento. **Criterio cumplido: cero archivos de test modificados** y el golden de
      `snapshot_baseline` sin moverse — la simulación produce el mismo estado.
      `avanzarRacion(escuadrones, trigoDisponible, factorConsumo = 1) -> { escuadrones, trigoConsumido, eventos }`;
      `avanzarMantenimientoTropas` queda como envoltorio sobre el almacén. `consumoRacionTropas` tampoco
      cambia de contrato: ahora delega en `consumoRacionDeEscuadrones`, que es el que reutilizará el carro.
      Añadido `avanzarRacion.test.ts` (7 casos) para congelar lo que el envoltorio de guarnición NO ejercita
      y de lo que el Paso 5 va a depender: `trigoConsumido` como retorno y `factorConsumo` (estacionado).
      Retirada de paso `poblacionTotalConTropas`, código muerto (§10). 786 → 793 tests, `tsc` limpio.
- [x] **Paso 2 — Entidad `Jugador` + Liderazgo + entidad `Ejercito` + persistencia (2026-09-02).**
      `Jugador` y `Ejercito` en `domain/types.ts`; `LIDERAZGO` y `LOGISTICA` en `constants.ts`; campo
      `velocidad` por tropa con la tabla de Doc 5.12.5. `engine/liderazgo.ts` puro (`costeLiderazgo` derivado,
      `liderazgoDe`, `liderazgoComprometido`, `puedeLlevar`, `liderazgoDisponible`).
      `GameSessionState.jugadores`/`.ejercitos` + **migración de snapshot v5 → v6** (ambas a `[]`).
      **Sin comportamiento**: el tick no las toca, así que NO entran en `EstadoSimulacion` hasta el Paso 4.
      Tests: la tabla de Doc 5.11.1 congelada tropa a tropa —es el sitio donde documento y código se miran a
      la cara, ya que el coste se deriva y no se ve en `constants.ts`—, el gate de élite, que no hay tope
      agregado, y la migración. 793 → 815 tests, `tsc` limpio en motor, lab y cliente.
      Sigue sin verificarse EN VIVO, como estaba previsto: eso llega en el Paso 3.
- [x] **Paso 3 — Comandos de ejército (2026-09-02).** `engine/ejercitos.ts` (dominio) +
      `session/comandos/ejercitos.ts` (capa de partida), con esquema, autorización y registro.
      **`replegar` y `cancelarMarcha` resultaron ser la MISMA operación desde estados distintos**, así que son
      un solo comando y no dos: marchando da media vuelta (invierte ruta Y progreso, la posición no se mueve
      ni un punto), estacionado calcula ruta nueva a casa. Se añadió `estacionarEjercito`, que el plan no
      listaba pero que la regla de Doc 5.12.3 exigía para poder llegar al estado `estacionado`.
      Códigos de error nuevos: `movilizacion.invalida`, `ejercito.no_existe`.
      El carro nace VACÍO a propósito — cargarlo del almacén es el Paso 6, aislado para poder medir su
      impacto económico por separado.
      **VERIFICADO EN VIVO** sobre un servidor que escucha de verdad (fetch, no `inject`), pasando además por
      disco para ejercitar el round-trip del snapshot v6: movilizar con 10 ≤ 10 pasa, con 25 > 10 devuelve
      `movilizacion.invalida`, el escuadrón desaparece de la guarnición, la ruta sale con 10 puntos y replegar
      deja el ejército en `regresando`. 815 → 826 tests, `tsc` limpio en motor, lab y cliente.
- [x] **Paso 4 — `avanzarEjercitos` en el tick (2026-09-02).** Comer del carro (con factor de estacionado),
      moverse a la velocidad del escuadrón más lento, disolverse si no queda nadie, y llegar. Colocado al
      final de la cadena, tras los bandidos.
      **Resultó ser mudo de RNG SIEMPRE, no solo cuando no hay ejércitos**: ni el hambre, ni el movimiento, ni
      la disolución necesitan aleatoriedad. Eso es más fuerte de lo que preveía el plan — el guardián de
      determinismo pasó **sin tocarlo**, y hay un test que lo congela contando las llamadas al RNG con y sin
      ejércitos (mismo número). El RNG entrará en el Paso 7, con el combate.
      Incluye la disolución del **ejército fantasma** (§1.1c / Doc 5.13.4) y el retorno: al llegar a casa
      reintegra la tropa **y devuelve el sobrante del carro** al almacén.
      `calcularTitulos` pasa a contar guarnición + campo (invariante 8): si no, "Ejército más grande" cambiaría
      de manos cada vez que alguien marcha y los Aedas narrarían un cambio que no ocurrió.
      826 → 838 tests, `tsc` limpio en motor, lab y cliente.

      > **Hallazgo de calibración, medido al verificar en vivo.** Trazando un ejército tick a tick, los cuatro
      > primeros avanzan a 0.006 de progreso y los siguientes a 0.086: **14× de diferencia**, porque la ruta
      > bordea agua (`COSTE_MOVIMIENTO.agua` = 15, `cima` = 12) al salir del asentamiento. En esa corrida, 13
      > ticks cubrieron ~161 unidades en vez de las 260 que darían en llano — **un 62% de la velocidad
      > nominal**. El cálculo de `capacidadCarroPorJugador` (Doc 5.13.1) asumió coste 1, así que **el radio
      > operativo real es menor que el objetivo**: hay que meter un factor de terreno medio en la calibración
      > del Paso 13. Es una ruta en una seed, así que es indicación, no calibración.
- [x] **Paso 5 — Render de ejércitos (2026-09-04).** Eran TRES piezas con destinos distintos, no una
      (corrección del usuario: el lab NO pinta mapa general —es vista de asentamiento— y hay DOS clientes).
  - [x] **5a — Cliente de administración** (`cliente/src/ui/canvas.ts`, 2026-09-02). Sin trabajo de backend:
        `EstadoAdmin` es un `Omit<GameSessionState,'mapa'>`, así que `ejercitos` viaja desde el Paso 2.
        Traza de ruta + **rombos apilados medio superpuestos, uno por jugador distinto**, en color de la
        Facción de origen. El recuento se DERIVA de `escuadrones`; no se guarda nada nuevo.
        **Verificado en vivo** con el reloj de mundo a 5 s/tick: dos ejércitos inyectados (uno de 3 jugadores,
        otro de 1) se mueven, gastan trigo en proporción 3:1 a sus soldados, y el log canta
        `El ejército ejercito-solitario llega a su destino y acampa`. Medido en píxeles sobre el canvas: el
        racimo de 3 jugadores ocupa **16 px de ancho frente a 6 px el de 1** — cada participante añade
        exactamente media anchura de rombo, que es el solape del 50% especificado.
  - [x] **5b — Proyección al jugador (2026-09-03).** Dos campos, no uno: `ejercitos` (los propios, completos)
        y `ejercitosAvistados` (los ajenos, redactados a `id` + `faccionId` + `posicionActual` +
        `participantes`). Van en arrays SEPARADOS a propósito — la diferencia entre "lo veo entero" y "solo
        lo avisto" queda en el tipo, no en un campo opcional que el cliente pueda olvidarse de mirar.
        Visibilidad = zona de influencia propia ∪ `LOGISTICA.radioVisionEjercito` (150) alrededor de los
        ejércitos propios, sin memoria: responde "¿se ve AHORA?" y nada más. Un jugador huérfano sigue viendo
        la columna en la que va su propia tropa, aunque no tenga Facción.

        **Además hubo que tapar una fuga que habría dejado la redacción en decoración**: los eventos de
        `avanzarEjercitos` salían SIN `asentamientoId`, y un evento sin atribuir es GLOBAL — o sea que el log
        le contaba a toda la partida que a un rival se le desertaban los hombres o que su columna acababa de
        llegar. Se atribuyen ya a su `origenAsentamientoId`, para lo que `EventoCrudo` acepta ahora un
        `asentamientoId` propio (un subsistema global que itera sobre ejércitos no cabe en la atribución por
        lotes de `simulation.ts`, que va asentamiento a asentamiento).

        **Verificado en vivo** contra `GET /v1/jugador/partidas/local` con cuatro ejércitos en el mundo: el
        admin ve los cuatro; la jugadora ve el suyo completo, dos rivales redactados a exactamente esas
        cuatro claves (uno dentro de su zona, otro visto solo por su ejército en marcha, con
        `participantes: 2` derivado de 2 escuadrones de 2 jugadores) y el cuarto **no aparece en un solo byte
        del payload**. En el log: de 17 eventos del tramo, 8 eran del ejército rival y a ella le llegaron 9,
        ninguno de ellos del rival.
  - [x] **5c — Cliente de jugador (2026-09-04).** `BronzeAgeClient`, **repo aparte** en
        `C:/Users/VRINO/Desarrollo/BronzeAgeClient`, rama `ejercitos-en-el-mapa`. Espejo de 5a con el MISMO
        glifo y las mismas medidas (`dibujarRacimoDeRombos`), alimentado por los dos arrays de 5b: a los
        propios se les pinta además el rastro de la ruta, como a las caravanas; a los avistados NO, y no por
        simplificar — su ruta no viaja, así que no hay estela que dibujar, y esa ausencia ES lo que se sabe
        de ellos. Copias locales de `Ejercito` y `EjercitoAvistado` en `src/tiposDominio.ts`, porque ese
        cliente no importa del motor por diseño.

        **Verificado en vivo**, medido en píxeles sobre el canvas: el racimo mide **7, 13 y 18 px** de
        relleno para 1, 2 y 3 jugadores —cada participante añade ~5, media anchura de rombo, el solape
        especificado— y queda centrado sobre la posición del ejército con menos de 1 px de desviación. Dos
        Facciones, dos colores. El cuarto ejército, fuera de vista, ni llega en la proyección ni pinta un
        píxel de su Facción donde debería estar. De propina, el radio de visión se vio funcionar solo: un
        rival dejó de proyectarse en cuanto la columna propia se alejó más de 150 de él.

        **Se arreglaron dos cosas rotas de antes** que salieron al montar la verificación: la entrada
        `bronze-age-cliente-jugador` de `.claude/launch.json` seguía apuntando a `cliente-jugador/`, un
        directorio que dejó de existir cuando el cliente se movió a su repo (`2dfe9e7`), con dos variables de
        entorno que nadie lee; y la barra de estado del cliente imprimía literalmente `Tick: undefined` desde
        que la Fase D retiró el `tick` del contrato (lo que viaja es `instante`).

- [x] **Paso 6 — Carga del carro desde el almacén (2026-09-04).** Al movilizar y al unirse, cada Jugador
      carga trigo hasta el menor de dos topes: el espacio libre del carro y lo que el asentamiento puede
      soltar sin bajar de su reserva. Si no llega, **se sale con menos autonomía y punto** — el diseño dice
      explícitamente que no se bloquea la salida.

      **La reserva no es una constante nueva.** Es la que YA protegía a la ciudad de la auto-construcción y
      del reclutamiento: `(consumo de población + guarnición) × RESERVA_CONSTRUCCION.horizonteMinutosComida`.
      Estaba copiada en dos sitios y esta habría sido la tercera copia, así que se extrajo a
      `reservaDeTrigo` (`engine/tropas.ts`) y los tres llamadores miden con la misma vara. El refactor se
      midió en batch: **cero diferencias en 422 líneas de informe.**

      Dos detalles que sí son decisiones:
      - La reserva se calcula sobre el asentamiento **ya sin los escuadrones que salen**: dejan de comer ahí
        en el mismo acto, y seguir contándolos protegería bocas que ya no están.
      - El tope al unirse va contra la capacidad TOTAL de la columna (participantes × carro), no contra "un
        carro más". Sin eso, un jugador que ya iba dentro podía unirse otra vez, y otra, sacando 500 de trigo
        cada vez: una bomba de trigo infinita desde el almacén.

- [~] **Paso 6b (sale de medir el Paso 6) — la economía no puede pagar el carro.** El usuario eligió palanca el 2026-09-04: Granero + doblar otra vez el trigo. Resuelto en una ciudad sana; en el batch sigue sin verse porque esas ciudades no construyen almacenaje NINGUNO. Ver §10.1.

- [x] **Paso 7 — Llegada → asedio (2026-09-04).** Un ejército que termina su ruta sobre un asentamiento de otra
      Facción resuelve el asedio ahí mismo, una sola vez. Sin defensores cae sin combate (Doc 5.12.4) **y sin
      tocar el RNG**, que es lo que mantiene verde el guardián de determinismo sin modificarlo: una partida sin
      asedios contra plazas defendidas hace exactamente las mismas llamadas que antes. Como el RNG ya entra
      aquí, los ejércitos pasan a recorrerse en **orden canónico por id** (lo que el §9 pedía para el Paso 10).

      **La conquista se escribió una vez** (`aplicarConquista`) y la usan los dos caminos, así que el comando
      viejo `iniciarAsedio` deja de contradecir al canon: hasta ahora **heredaba la guarnición del vencido**,
      justo lo que Doc 5.4 prohíbe, y no lo cubría ningún test. Ahora la guarnición se pierde, la residencia y
      los cargos se vacían —de ahí sale el huérfano, derivado y no almacenado— y la ciudad se entrega intacta.

      **Decisión que el canon no cerraba**: qué pasa con los residentes que estaban EN CASA. Se les retira la
      residencia igual que a los de campaña; la alternativa era dejarlos como residentes de una ciudad enemiga.
      Anotado en Doc 5.4 pendiente de confirmación.

      **Verificado en vivo** sobre el servidor real: ana moviliza 25 milicianos contra una plaza rival de 5
      defensores; a los 5 ticks la plaza pasa a su Facción con la **guarnición vacía**, residentes y cargos a
      cero, y sus **33 edificios, 210 habitantes y 3.379 de trigo intactos**. Su columna queda acampada FUERA
      con 23 de 25 supervivientes. El carro había salido con 500 de trigo — el Paso 6 y el Granero funcionando
      en el mismo trayecto.

      **Audiencia de los eventos**: un choque entre dos se narra al hogar del atacante y, si la plaza resistió,
      también a la plaza. Si cae, solo al atacante — atribuírselo también a la ciudad recién conquistada se lo
      enseñaba dos veces al mismo jugador (medido en vivo antes de corregirlo). Queda el hueco de que **al
      vencido no le llega la noticia de su derrota**, porque pierde el asentamiento por el que la vería: pide
      una audiencia por Facción que el modelo de eventos no tiene, y es el mismo agujero del Paso 11.
- [ ] **Paso 8 — Reabastecimiento en ruta** (propio siempre, aliado con la opción activa) + campo
      `permiteReabastecerAliados`. **Más importante de lo que parecía**: en tiempo real un ejército
      estacionado quema su carro en ~100 minutos, así que hasta este paso "sostener un paso de montaña" no
      existe como posición, solo como jugada corta (§2.5b).
- [ ] **Paso 9 — Caravanas adjuntas**: capacidad, entrada en el `min` de velocidad, pérdida al ser derrotado,
      y escolta (la caravana entrega mientras marcha). Incluye el rebalance de `CARAVANA_CATALOGO` **medido en
      batch por el efecto sobre el oro**.
- [ ] **Paso 10 — Encuentros por proximidad** (ejército↔ejército, ejército↔caravana), con **orden canónico de
      resolución por id** (§9, hallazgo de la revisión cruzada: sin él el RNG deja de ser determinista aunque
      la secuencia global sea correcta).
- [ ] **Paso 11 — Retirar `combateCampoAbierto` e `interceptarCaravana` como comandos.** Demolición separada
      del paso que la habilita: primero funciona lo nuevo, después se borra lo viejo.
      **Se llevan por delante una fuga de visibilidad** detectada al hacer el 5b: los dos emiten sus eventos
      SIN `asentamientoId` a propósito (`session/comandos/militar.ts`, "el choque es entre DOS asentamientos,
      atribuirlo a uno sería arbitrario"), y un evento sin atribuir es global — o sea que hoy toda la partida
      lee la resolución completa de cualquier combate entre dos rivales, con los ids de ambos bandos en el
      `payload`. No se arregla ahora porque este paso los borra; si por lo que sea sobrevivieran, hay que
      decidir a quién se atribuye un choque entre dos (probablemente a los DOS: un evento por bando).
- [ ] **Paso 12 — NPC** migrado a marchar, con diario de batch contra la línea base.
- [ ] **Paso 13 — Calibración** por simulación: `LIDERAZGO.factorCoste`, `capacidadCarroPorJugador` (recalculado
      desde el radio si cambió la ración), velocidades y capacidad de caravana.

## 6. Invariantes a congelar en tests

1. Un jugador nunca saca escuadrones cuyo coste de liderazgo sumado exceda su liderazgo.
2. Un escuadrón está en **exactamente un sitio**: la guarnición de un asentamiento o un ejército. Nunca en dos,
   nunca en ninguno.
3. La guarnición y el ejército aplican **la misma** curva de moral/deserción (mismo `avanzarRacion`).
4. Un ejército estacionado consume **menos que marchando, pero más que 0**.
5. El trigo se conserva: lo que sale del almacén al carro, más lo devuelto al volver, más lo comido, cuadra.
6. Un asentamiento sin guarnición es conquistado, **no** rechaza el asedio.
7. La conquista **no** transfiere escuadrones al conquistador.
8. "Ejército más grande" cuenta guarnición + campo (no baila al marchar).
9. Determinismo: dos partidas con la misma seed y los mismos comandos producen los mismos ejércitos.
10. **Con varios encuentros en un mismo tick, el orden de resolución es canónico por id** — no el de
    iteración de una estructura (§9.1 punto 6).
11. Un ejército va a la velocidad de su integrante MÁS LENTO, contando también las caravanas adjuntas.
12. Cancelar una marcha deja el ejército en `regresando`, nunca lo desvanece ni lo teletransporta.
13. Un ejército derrotado pierde sus caravanas adjuntas.
14. El carro NO se descarga en un asentamiento distinto del de origen (si no, es transporte gratis).
15. Un jugador sin residencia (huérfano) no puede reclutar, reabastecer en propio ni replegarse.
16. Un ejército con todos sus escuadrones a cero **se disuelve**, y esas identidades vacías reaparecen en el
    asentamiento de origen (nunca se pierden por hambre: lo que murió son las unidades, no el squad).

## 7. Abierto — lo que este documento deja sin cerrar a propósito

- **Los números de liderazgo** (§1.3) — pendiente de confirmación del usuario.
- **Visibilidad de ejércitos ajenos.** Fase 0 proyecta los propios completos y los ajenos **nada** — slice
  conservador coherente con el Slice 1 actual de `proyectarParaJugador`. El radio de visión depende de
  `Docs/Mecanicas a desarrollar.md` §12 (niebla de guerra), que sigue sin parámetros. **No bloquea.**
- **Progresión del liderazgo** — es §11 (*progreso jugador y tropa*), sin diseñar.
- **Convergencia con el revamp de caravanas (§8)** — el carro de suministros y los carros/animales de tiro
  son el mismo concepto físico; Fase 0 lo resuelve como un número.
- **El escuadrón mermado paga liderazgo completo** (§9.3): uno a 3/20 cuesta los mismos puntos que uno lleno.
  Sin resolver a propósito — resolverlo obliga a decidir si el coste mira `cantidad` actual, y eso reabre el
  exploit de la élite parcial.
- **La varianza de combate** (§1.4) — decisión abierta con recomendación; conviene cerrarla antes del Paso 10.
- **Órdenes condicionales** (§9.3): reglas de enfrentamiento y retirada automática por umbral de bajas, para
  que un jugador que vuelve tras ocho horas no encuentre solo un log. Es la crítica más profunda del consejo
  y probablemente el siguiente diseño después de este, pero el movimiento es su prerrequisito.
- **Capacidad de caravana**: el rebalance a >= 500 multiplica el oro por entrega (§8). Converge con el revamp
  de caravanas (§8 de `Mecanicas a desarrollar.md`), donde carros y animales de tiro redefinen la carga.
- **¿Aprovechan los ejércitos los caminos comerciales?** `COSTE_MOVIMIENTO.factorCamino` está disponible pero
  no se aplica; aplazado porque toca el balance de la guerra relámpago.

## 8. Riesgos

| Riesgo | Coste si pasa | Mitigación |
|---|---|---|
| El NPC marchando hunde los números del batch | alto — ya pasó con otras mecánicas militares | Paso 10 aislado, con diario de batch antes/después |
| Los ejércitos vuelven la guerra demasiado lenta y nadie ataca | medio | la `velocidad` de cada tropa es un dato del catálogo; calibrable en el Paso 13 sin tocar código |
| El determinismo se rompe al mover el combate al tick | medio | Paso 6 aislado; `determinismo.test.ts` se actualiza en el mismo commit |
| Sacar el ejército deja al asentamiento sin trigo y colapsa | medio | es coste deseado, pero hay que medirlo: la reserva de `reclutarTropa` protege el reclutamiento, no la salida. **Paso 6 propio** con reserva mínima intocable |
| Subir la capacidad de caravana multiplica el oro por entrega | **alto** | `comision = valorTotal × tasa × distancia × reputación` (`trade.ts:234`) escala con la carga: ×8 de capacidad es ×8 de oro por viaje. Medir en batch en el Paso 9 ANTES de fijar el número |
| **Una ráfaga de catch-up con muchos ejércitos bloquea la cola serial** | **alto, sin medir** | `MAX_TICKS_RAFAGA` son 10.080 ticks en UNA entrada de la cola (`sincronizarConReloj`, D5). Hoy los bandidos pagan `campamentos × caravanas` por tick; el Paso 10 añade `ejércitos × caravanas + ejércitos²`. Con 50 ejércitos y 100 caravanas son ~7.500 distancias/tick × 10.080 = ~75M en una ráfaga. **Medir el coste de la ráfaga en el Paso 10**, no solo la corrección |
| El determinismo se rompe con varios encuentros en un tick | medio | orden canónico de resolución por id (§9.1 punto 6), congelado en test |

## 9. Revisión por consejo (2026-09-02)

Cinco asesores independientes (Contrarian, First Principles, Expansionist, Outsider, Executor) + cinco
revisiones cruzadas anónimas, mismo procedimiento que `Murallas_Definicion.md` §19.1.

### 9.1 Hallazgos que cambiaron el diseño

| # | Hallazgo | Estado |
|---|---|---|
| 1 | **Exploit de las "mulas" de suministro.** Con carro de 300 FIJO por jugador y aditivo, un aliado que se une con una milicia aporta un carro entero: la autonomía sube más que el consumo. Y como el sobrante volvía al almacén, era además **transporte de trigo gratis que canibalizaba las caravanas**. Lo encontraron dos asesores por separado. | **RESUELTO**, pero no como proponía el consejo (capacidad proporcional a la tropa): el usuario mantiene el carro fijo (§1.1b punto 12) y el exploit muere por otra vía — ver §9.2. |
| 2 | **El encuentro ejército↔caravana era letra muerta.** Con velocidad plana de 10 y caravanas a 16-24, un ejército no alcanzaba jamás a una caravana. Verificado contra `CARAVANA_CATALOGO`. | **RESUELTO** por la tabla de velocidades por tropa (§1.1b punto 13): la ligera (20) caza a la comercial (16). |
| 3 | **La guerra era suma negativa.** Conquistar no daba botín (la guarnición se pierde) y el defensor podía evacuar sus tropas para no perderlas ⇒ el meta óptimo era no atacar nunca. | **RESUELTO** por el incentivo de conquista (§1.1b punto 16). |
| 4 | **No había forma de cancelar ni retirar una marcha.** Fire-and-forget durante horas reales. | **RESUELTO** (§1.1b punto 14). |
| 5 | **El jugador cuya ciudad cae estando de campaña quedaba sin definir.** El diseño resolvía la re-adopción del ejército pero no la residencia del jugador. | **RESUELTO** por el estado huérfano (§1.1b punto 15). |
| 6 | **Orden canónico de resolución entre ejércitos.** Solo apareció en la revisión cruzada: `resolverCombate` consume RNG, así que con N encuentros en un tick el orden de iteración determina el consumo. Sin ordenar por id, el determinismo se rompe aunque la secuencia global del tick sea correcta. | **INCORPORADO** al Paso 10. |
| 7 | **El determinismo no necesitaba romperse.** Colocando `avanzarEjercitos` al final de la cadena y haciéndolo mudo de RNG cuando no hay ejércitos, el test guardián sigue verde sin tocarlo. Verificado contra `simulation.ts` (`crecerPoblacion` l.190, `avanzarAtaquesBandidos` l.232). | **INCORPORADO** al Paso 4. Sustituye al plan anterior, que asumía actualizar el test. |
| 8 | **Faltaba un paso: la carga del carro desde el almacén.** Iba implícita dentro de otros pasos; si el batch se hunde, no se podría saber si fue la logística o la IA. | **INCORPORADO** como Paso 6. |
| 9 | **El render estaba demasiado tarde.** El lab es el instrumento de medida; sin él los pasos de asedio, reabastecimiento y encuentros se depuran a ciegas. | **INCORPORADO**: sube al Paso 5. |

### 9.2 Por qué la "mula" deja de ser un exploit con el carro FIJO

El consejo pedía capacidad proporcional a la tropa. El usuario mantiene el carro fijo por coherencia física
("es un carro"). Medido, con 70 soldados / 500 de carro / ración 0.15:

| Se une… | Soldados | Trigo | Autonomía | Poder |
|---|---|---|---|---|
| nadie | 70 | 500 | 48 ticks | ×1 |
| un combatiente (70 sold.) | 140 | 1000 | **48 ticks** | **×2** |
| una "mula" (25 sold.) | 95 | 1000 | **70 ticks** | ×1.36 |

El combatiente duplica el poder sin perder alcance; la mula da +47% de alcance por +36% de poder. **Ninguno
domina** — la mula solo gana si el alcance es la restricción, y entonces no es un exploit sino un
**intendente**, rol legítimo. Lo que sí había que cortar es el transporte gratis, y se corta con la regla de
que **el carro solo se descarga en el asentamiento de origen** (Doc 5.13): mover mercancía se hace con
caravanas adjuntas, que es explícito y tiene coste.

### 9.3 Lo que el consejo señaló y NO se acepta

- **"El coste de liderazgo es lineal sobre el mismo escalar que el jugador maximiza, luego no hay decisión,
  solo aritmética"** (First Principles). Es cierto *en aislamiento*, pero ignora el acoplamiento con el
  suministro: la ración va por SOLDADO y el coste por PODER, así que a igual liderazgo las tropas baratas
  traen muchas más bocas y menos alcance. La decisión existe, solo que el eje no es el poder sino el alcance.
- **"El exploit de sacar un escuadrón de élite parcial y pagar menos"** (Contrarian). No aplica: la fórmula
  usa `unidadesPorDefecto` del catálogo, constante por tropa, no la `cantidad` actual. **Pero destapó el
  problema inverso y real**: un escuadrón mermado a 3/20 paga los mismos puntos que uno completo. Queda
  anotado como punto abierto.
- **"Ramificar ya el Liderazgo en un árbol de progresión"** (Expansionist). Prematuro — tres de los cinco
  revisores lo señalaron como el mayor punto ciego del consejo: construye sobre números todavía sin calibrar.
- **"El diseño real no es movimiento sino órdenes condicionales"** (First Principles). Es la crítica más
  profunda y probablemente tenga razón a medio plazo: con 1 tick = 1 minuto real, una marcha de un cuarto de
  mapa son ~50 minutos y una campaña larga cruza la noche. Pero es un **pivote, no un arreglo**, y el
  movimiento es el prerrequisito de cualquier sistema de órdenes. Queda anotado en §7.

### 9.4 El dato que salió de la revisión y no es del consejo

Al medir para responder al consejo apareció que **el ejército no es el problema del trigo**:

| | consumo | |
|---|---|---|
| Civil | 0.1 trigo/tick | `POBLACION.consumoComidaPorHabitante` |
| Soldado | 0.15 trigo/tick | `MILITAR.racionPorSoldadoPorMinuto` |

Un soldado come 1.5× un civil, proporción sana. Pero un asentamiento **nivel 1 a tope de población (300)
come 30/tick y una Granja nivel 1 produce 15**: nace en déficit estructural. En nivel 3 (techo 6.000) harían
falta ~13 Granjas nivel 4. **Ese es el cuello de botella de nivel 3 ya documentado en los diarios de batch**,
y es anterior a los ejércitos: un ejército de 70 soldados come 10.5/tick, un tercio de lo que comen los
civiles de un nivel 1 con el 23% de las bocas.

**Consecuencia para esta mecánica:** no bajar la ración del soldado para que cuadren las campañas — sería
tapar un problema de producción con un parche militar. La capacidad del carro se deja expresada como
consecuencia del radio operativo (Doc 5.13.1), de modo que al rebalancear la producción el radio se mantenga
en vez de romperse en silencio.

**RESUELTO (2026-09-02): producción base de Granja ×2, adoptado tras experimento A/B/C.** Se añadió un Paso 0
al plan por decisión del usuario ("vamos con la opción 2"), y se midió 1x / 2x / 3x con la misma seed. Los
números y el detalle están en `Docs/Game/4` §4.2.1; el resumen: a 1x la nutrición **caía** (28.98 al tick 600)
y **el mundo entero sostenía 15 tropas**; a 2x la nutrición satura en 100, los asentamientos en nivel 2 pasan
de 7 a 11 de 13, y hay ~777 tropas. **3x no mejora nada sobre 2x** salvo sostener más ejército.

Esto importa para esta mecánica más allá del trigo: **con 1x el movimiento de ejércitos habría sido
intestable en batch** — no había ejércitos que mover. El Paso 0 no era solo higiene económica, era el
prerrequisito para poder medir la mecánica.

Dos cosas quedaron abiertas y tienen issue propio: el bloqueo de nivel 3 **no es alimentario**
(`issues/npc_no_alcanzan_nivel_3.md`, causa sospechada `artesanos = 0`) y el 2x **esquia** el problema de
escalado de Granjas en vez de resolverlo (`issues/granjas_no_escalan_con_poblacion.md`).

## 10. Higiene detectada de paso (no es de esta mecánica, pero toca el mismo código)

- ~~**`poblacionTotalConTropas`** (`engine/tropas.ts`) es código muerto~~ — **retirada en el Paso 1**
  (2026-09-02), junto con el import de `poblacionTotal` que solo ella usaba.
- **`distancia` está duplicada en cuatro módulos de `engine/`** (`bandidos`, `construction`, `mantenimiento`,
  `trade`) teniendo `world/geometria.ts` exportándola. Los imports `engine/` → `world/` ya son práctica
  establecida (`calcularRuta`, `pointInPolygon`, `unirPoligonos`), así que el módulo nuevo debe importarla y
  no hacer la quinta copia.

## 10. Medición del Paso 6: la economía actual no puede pagar el carro (2026-09-04)

Al instrumentar el batch con la logística de campaña (`sobranteParaCarroMedia`, `sinCarroCompleto`,
`sinNadaQueCargar` en `scripts/run-batch-sim.ts`), la línea base es demoledora. Corrida de 600 ticks, 30
Facciones, 28 asentamientos vivos:

| tick | sobrante medio para el carro | sin carro completo | sin NADA que cargar |
|---|---|---|---|
| 100 | 41,3 | 28/28 | 2/28 |
| 200 | 10,2 | 28/28 | 26/28 |
| 600 | **7,7** | **28/28** | **26/28** |

**Ni un solo asentamiento puede llenar un carro, y 26 de 28 no pueden aportar ni un grano.** El sobrante
medio es un **1,5 %** de `LOGISTICA.capacidadCarroPorJugador` (500).

La cuenta explica por qué. Un asentamiento nivel 1 a tope tiene ~300 habitantes (30 de trigo/minuto) y ~73
soldados (11/minuto): su reserva son `41 × 8 ≈ 330`. Para sacar a UN jugador con el carro lleno haría falta
tener **~830 de trigo almacenado**, y estas ciudades viven rozando su reserva.

Esto **no invalida la implementación** —hace exactamente lo que el diseño pide— sino que revela que la
capacidad del carro se derivó del **radio operativo** (§5.13.1: "un cuarto del mapa ida y vuelta") sin que
nadie comprobara que la economía puede pagarlo. Las dos mitades del diseño nunca se habían tocado.

Es la misma raíz que el cuello de botella ya conocido (§9.4): el trigo va justo en todas partes. El Paso 0 lo
dobló y sigue sin llegar para sostener además campañas.

**No se toca ninguna constante todavía** — la medición es el entregable; elegir la palanca es decisión del
usuario. Las opciones, sin recomendación cerrada:

1. **Bajar la capacidad del carro** y aceptar un radio operativo menor (revisa §5.13.1, que es canon).
2. **Subir la producción de trigo** otra vez, o bajar el consumo — arregla también §9.4, pero mueve toda la
   economía y exige rebalance completo.
3. **Bajar `horizonteMinutosComida`** solo para el carro, desacoplándolo de la auto-construcción: sacar un
   ejército pasaría a ser legítimamente más arriesgado que construir.
4. **Aceptarlo como está**: sacar un ejército con autonomía exige haber acumulado excedente a propósito, y hoy
   nadie lo hace porque el NPC recluta a saco. Es defendible, pero significa que en Fase 0 las campañas largas
   no existen hasta que alguien juegue para ello.

Nota sobre la métrica: es un **suelo**. Se mide con la guarnición entera dentro, y llevarse tropa reduce la
propia reserva (48 de margen por cada 40 soldados que salen). A la escala del carro no cambia la conclusión.

### 10.1 La palanca elegida, y qué destapó (2026-09-04)

De las cuatro opciones de §10 el usuario eligió una combinación de la 1 y la 2, con una pieza nueva:

- **Granero**: almacén especializado en grano, uno por asentamiento, 4 niveles internos, de 2.000 a 6.000 de
  capacidad de trigo (Doc 4.2/4.3). Sin él, doblar la producción no habría servido de nada: el trigo de más se
  perdía contra el techo del almacén.
- **Producción base de trigo doblada otra vez**: 30 → 60 por Granja de nivel 1 (60/90/120/180 por nivel).

**Medido en una ciudad sana** (fixture de un asentamiento, seed 20, 300 ticks): el Granero se construye en el
tick ~50, sube a nivel 3 hacia el 150, y el excedente disponible para el carro pasa de **7,7 a 5.881** —de un
1,5% de un carro a casi doce carros llenos—. El Paso 6b queda resuelto **para una ciudad que funciona**.

**Medido en batch (30 Facciones, 600 ticks): CERO Graneros construidos.** No es culpa del Granero: la sonda
`SIN FONDOS` disparó en las 1.406 propuestas, y el contador `almacenesActivos` sale **0** — esas ciudades
tampoco construyen ni un Almacén. Se proponen, tienen sitio y tienen cupo, pero no pueden pagar 50 de madera
sin bajar de su reserva de mantenimiento. **La madera, no el trigo, es lo que las tiene atascadas**, y eso es
anterior a todo esto (es el mismo cuello de botella de §9.4 visto desde otro lado).

Consecuencia práctica: el batch **no puede medir todavía** el efecto económico de la logística de campaña, y
cualquier lectura sobre excedentes ahí está midiendo la pobreza de la madera. Queda como entrada del Paso 13.

### 10.2 Un bug latente que salió al añadir el Granero

El array `Asentamiento.edificios` es el **historial de crecimiento**, y su orden es load-bearing: `redDeCalles`
lo replaya de principio a fin y una calle solo puede nacer donde el suelo estaba libre cuando le tocó a ese
edificio. `avanzarConstruccion` lo **permutaba** al final de cada tick, reordenando los `en_cola` por
`prioridad` — un arreglo de PRESENTACIÓN (que la posición mostrada coincidiera con el orden de arranque)
implementado permutando el historial.

Eso rompe §E6.12 ("ningún edificio encima de una calle"): un proyecto encolado con score alto salta por delante
de edificios YA CONSTRUIDOS y, en el replay, se procesa antes que ellos —cuando su suelo aún consta libre— y
les tiende una calle por debajo. El guardián de §E6.16 (`pisaCalleComprometida`) no podía verlo: valida contra
el prefijo real en el momento de pagar, y la permutación ocurre después.

Reproducido: seed 42, perfil `nucleos`, tick 29 — el Granero pasa a la posición 14 y deja a la Vivienda 15 con
la celda (1,-6) convertida en calle bajo sus cimientos. Latente desde que existe la reordenación; salió ahora
porque el Granero se encola con urgencia máxima y salta muy arriba.

**Arreglado retirando la permutación**: el array vuelve a ser historial y el orden de cola se deriva de
`prioridad` donde se necesita — que es lo que ya hacían `avanzarConstruccion` y `moverEnCola`, y ahora también
el cliente al pintar la cola.
