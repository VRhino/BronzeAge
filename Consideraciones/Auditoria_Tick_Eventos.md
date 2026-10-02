# Auditoría del tick: qué se evalúa cada minuto y qué debería dispararse por hecho

**Abierto el 2026-10-02. Cuarta pasada, sobre `b452315`** (`origin/main`, 2026-10-02 13:57, campamentos de mercenarios, paso 5
de 5). La §0 resume qué cambia respecto a la tercera pasada (`70d3f0e`); el resto del documento ya está al día salvo donde
la §0 dice lo contrario.
Las dos primeras se hicieron sobre `8091638` (2026-09-26): una solo leyendo código y otra midiendo. Entre ambos
commits entraron 62 commits que tocan 136 ficheros de `src/`: tecnología y Eras, red de caminos, guerra declarada,
capital, protección post-conquista, `dejarResidencia`… Y varios arreglan cosas que señalaba la pasada anterior. Este
documento describe **el estado actual**; la §4 dice qué se arregló y qué no.

Sale de la corrección de la residencia en campamentos: un barrido que recorría todos los héroes en cada tick para
reubicar a quien se había quedado sin casa, cuando era la **consecuencia de un hecho puntual** (cae una plaza, se
arruina, alguien deja su casa) y debía resolverse en el acto. Aquí se hace la misma pregunta sobre el resto del tick.

Es un **informe para decidir, no un plan**. Las medidas y reproducciones son scripts aparte que importan `src/` tal
cual. Tras el informe, el usuario decidió implementar parte de lo que recoge; la sección siguiente dice qué.

## Estado de la implementación (2026-10-02)

Lo descrito en el resto del documento es el estado **antes** de estos cambios; cada fila dice dónde se resolvió.

| Qué | Resuelto en | Cómo |
|---|---|---|
| **R, B5** | `6b15716` | `acogerHeroesNpc` desaparece. La derrota de la última plaza se resuelve donde ocurre (`session/derrotas.ts`: `derrotasEntre`, `resolverDerrotasNpc`, `conDerrotasResueltas`): el tick, `iniciarAsedio`, el asedio por columna y el resultado de una batalla; el batch la llama entre motor y NPC. Anexión por NPC ganador (los bots pasan a la ganadora, que los reparte por la plaza con menos residentes contando la conquistada); disolución en los demás casos. El Paso 0 saca de los campamentos a los fundadores. |
| **B4** | `6b15716` | `iniciarAsedio` registra la derrota. Con test que falla si se quita. |
| **O2** | `1827f06` | `aplicarConquista` limpia `politicasActivas`, `reservaManual`, `autoConstruccionPausada` y `recetasPausadas` del dueño derrotado. |
| **B1, B2, ficha B** | `d4c1390` | `engine/ruina.ts`, `cerrarDependientesDeRuina`: dispersa el campamento de bandidos, cancela sin penalizar los trueques en pie, y las caravanas vuelven a casa con su carga (o se pierden si su origen es la ruina o no hay camino). |
| **B3** | `74e76e3` | El id lleva el minuto de fundación (`asentamiento-2-120-1440-t29455201`). Cambia el trazado de las ciudades que se funden desde ahora; las ya fundadas conservan su id. |
| **N2b** | `9e25811` | `ejercito.reabastecido` solo se narra si el carro estaba por debajo de `LOGISTICA.umbralNarrarReposte` (50 %). El reposte no cambia. A los 4 500 ticks: 36 928 eventos en memoria frente a 59 240 (−38 %). |
| **A** | `43f4375` | `aplicarExperiencia` suma la experiencia, recalcula el nivel y emite `faccion.nivel_subio` donde se da: combate (5 sitios), comercio, entrega desde caravana adjunta, fundación y el propio tick. |

**Sin hacer:** **N2a** (el log en memoria se sigue copiando entero en `exito`: con N2b son ~1,6 ms por copia a los 4 500 ticks,
unos 5 ms por tick; el arreglo de verdad cambia el orden del log, que leen la persistencia JSONL, los cursores y muchos
tests, así que se decide aparte), reclutamiento NPC, trueques terminales al historial, títulos, idempotencia del NPC y
**O1** (difusión por WebSocket de los ticks del reloj).

**Hallazgos de paso:** `iniciarAsedio` no puede asediar una plaza sin defensores (devuelve "No hay escuadrones válidos
seleccionados"), y `typecheck:lab` falla en `main` por `lab/src/main.ts:209`.

> **Alcance.** `9f9a098` ("reubicar a quien pierde la casa en el momento del hecho") **ya está en `main`** y se audita aquí.

## 0. Cuarta pasada: qué cambia con `b452315`

Entre `70d3f0e` y `b452315` entran 6 commits: los cinco pasos de los campamentos de mercenarios y `9f9a098`.

**Lo que `9f9a098` resuelve en el hecho, y está bien:**
- **Ruina:** sus residentes se reubican en el acto (`reubicarResidentesDeRuina`, `simulation.ts:256`; `mercenarios.ts:231`), en
  la plaza propia más cercana o, si no queda ninguna, en el campamento de mercenarios más cercano.
- **Conquista:** `desalojarResidentes` lleva a quien se queda sin plaza propia al campamento más cercano en los tres caminos
  (asedio con ejército, comando `iniciarAsedio`, batalla de Unity).
- **`dejarResidencia`:** lleva al campamento en el acto.
- **Fundar con caravana:** saca a los fundadores del campamento en el mismo tick (`simulation.ts:276`).
- **De paso:** la población de un campamento se calcula al mirar, no cada tick (`poblacionActual`); la reposición del mercado
  es una fecha (`reponerMercados`); la aparición de campamentos se mira cada `MERCENARIOS.cadaMinutos`. Todo correcto.

**Lo que no resuelve, y lo empeora: el barrido viejo sigue y ahora choca con el nuevo.** `acogerHeroesNpc`
(`npcGobernanza.ts:1530`, llamado en `2075`) sigue corriendo cada turno NPC y no sabe que existen los campamentos de
mercenarios: el NPC no toca `campamentosMercenarios` en ningún sitio. Hay así **dos mecanismos para la misma
consecuencia**, uno en el hecho y otro barriendo, y se pisan. Es el bug **B5**, reproducido con la sesión real:

| Escenario | Tras el tick (mecanismo nuevo) | Tras el turno NPC (barrido viejo) |
|---|---|---|
| La última plaza de una Facción NPC cae en ruinas | sus bots residen en `mercenarios-0` ✔ | la Facción se disuelve y **sus héroes se borran**, pero `mercenarios-0` **los sigue listando como residentes** |
| Una Facción NPC pierde su última plaza contra otra NPC | sus bots residen en `mercenarios-0` ✔ | se anexiona y los bots pasan a residir en una plaza de la ganadora **sin salir del campamento**: **residen en dos sitios** |
| Una Facción NPC la pierde contra un jugador y el Paso 0 la refunda | sus bots residen en el campamento | `fundarAsentamientosIniciales` los hace fundadores de la plaza nueva sin sacarlos del campamento: **doble residencia** (por lectura, no reproducido) |

Con esto, **R** deja de ser solo "mal asignado" y pasa a ser la causa de un bug. Lo que hay que hacer es terminar el trabajo
de `9f9a098` en el NPC:
- La **anexión** y la **disolución** van donde se registra la derrota (`registrarDerrota`), y tienen que sacar o reasignar
  a los residentes de los campamentos.
- La rama "dar casa a un bot sin residencia" sobra, porque ya nadie se queda sin casa, y además ignora la residencia en
  campamento.
- El Paso 0 tiene que sacar a sus fundadores del campamento, como ya hace la fundación con caravana.

**Sin cambios respecto a la tercera pasada** (vuelto a comprobar sobre `b452315`):
- **B1** (campamento de bandidos huérfano, heredable con B3), **B2** (penalización por la ruina del socio) y **B3** (id de
  asentamiento que renace): reproducidos igual.
- **B4:** `iniciarAsedio` (`comandos/militar.ts:108`) sigue sin `registrarDerrota`. Ahora sí lleva a los residentes al
  campamento, pero la Facción no queda marcada como derrotada.
- **(A)** nivel de Facción y **N2** log de eventos: igual. Medido sobre `b452315`, con el mismo método: motor ~9,4 ms, NPC
  4,2 → 5,9 ms, 59 240 eventos a los 3 días. Los mercenarios no añaden coste apreciable.

---

## 1. Conclusiones

1. **El tick completo ha bajado de ~45 a ~15 ms** con 20 Facciones NPC (§3). El turno NPC pasa de 22-38 ms a 4-6 ms,
   gracias a que el reclutamiento ya no prueba tropas sin tecnología y a que las órdenes cerradas salen de la lista
   viva. El motor sigue en ~9 ms.
2. **El sitio del principio que más importa ahora es `acogerHeroesNpc`** (§6, **R**). Hace en `main` lo mismo que se
   rehízo para la residencia: tres consecuencias de hechos discretos detectadas barriendo cada turno. No pesa
   (~0,05 ms), pero mezcla casos, tiene ventana mientras la Facción está en batalla, y depende de un campo
   (`derrotadaPor`) que **uno de los cuatro caminos de conquista no escribe** (bug **B4**).
3. **Lo único que sigue creciendo con la edad de la partida es el log de eventos en memoria** (§6, **N2**).
   `exito()` copia el log entero en cada mutación: 2,0 → 4,0 ms/tick entre los ticks 1 500 y 4 500, ya ~25 % del
   tick. A los 3 días son 59 000 eventos y 16,7 MB. El **39 %** es `ejercito.reabastecido`: un proceso continuo
   narrado **cada minuto** por cada columna parada, que es el patrón inverso al de esta auditoría.
4. **Siguen abiertos** el nivel de Facción re-derivado al final del tick (**A**) y los dependientes de una ruina
   (**B**). Siguen reproduciéndose **B1** (campamento huérfano, ahora además heredable), **B2** (penalización por la
   ruina del socio) y **B3** (id de asentamiento reutilizable).
5. **La tecnología ya nació bien**: los contadores se suman en el hecho, también desde los comandos vía `exito`. Lo que
   re-evalúa cada tick son los hitos de aparición, y medido no cuesta nada (§7).
6. **No conviene mover** las caducidades por fecha, el NPC por umbrales, la memoria de niebla, la red de caminos ni la
   regeneración de yacimientos (§8).

---

## 2. Cómo se ha clasificado

- **1 · Legítima del tick.** Dinámica continua o periódica por naturaleza: producción, consumo, crecimiento,
  mantenimiento, movimiento, caducidad por fecha, reposición programada.
- **2 · Mal asignada.** Consecuencia de un hecho discreto (comando, conquista, obra terminada, llegada, paso a estado
  terminal) que se detecta recalculando o barriendo en vez de resolverse cuando ocurre.
- **3 · Dudosa.** Depende de cuántas veces se mire, de una ventana poco clara, o mezcla un caso real con otros.

Cinco preguntas para decidir si algo de la 2 o la 3 **debe** moverse. Si alguna sale mal, no se mueve:

1. **¿Hay un hecho identificable y enumerable?** Si la consecuencia puede venir de muchos caminos y mañana de uno más,
   el barrido es el invariante; un evento por camino deja uno atrás (y **B4** es un ejemplo real de eso).
2. **¿Escribe estado/evento o solo lee algo derivado?** Para lecturas, **comprobar barato antes** o **memoizar con
   clave** (Doc 6 E3), no un evento.
3. **¿Qué ventana abre el barrido?** Hasta un tick suele ser aceptable; no lo es que un *comando* devuelva un estado que
   contradice su propio efecto, ni una ventana que dura lo que una batalla.
4. **¿Qué orden de evaluación cambia?** El pipeline de `avanzarSimulacion` es *load-bearing*.
5. **¿El hecho tiene un único sitio de escritura?** Si sí, mover es barato.

Lo que protegería cada cambio:

| Guardián | Qué fija | A qué es sensible |
|---|---|---|
| `engine/__tests__/snapshot_baseline.test.ts` | 1 Facción, 1 plaza, semilla 99, cortes 1/10/25/50/100 | orden del pipeline de construcción, población y mantenimiento. No ve NPC, combate, conquista, ruina ni comercio |
| `engine/__tests__/determinismo.test.ts` | 80 ticks; `toEqual` del estado **y de los eventos de cada tick** | qué evento sale en qué tick; todo consumo de RNG |
| `engine/__tests__/invariantes_simulacion_larga.test.ts` | 3 plazas, 300 ticks | recursos negativos, NaN, medidor fuera de rango |
| Batch (`scripts/run-batch-sim.ts`, sello SHA-256) | estado completo byte a byte | todo, incluidos el NPC y los **ids**. El reclutamiento NPC ya consume el contador también al saltar tropas sin tecnología, para no mover ids |
| RNG | `crecerPoblacion`, bandidos, asedios, encuentros | mover algo que lo consume cambia la secuencia de todo lo posterior |

---

## 3. Medidas sobre `70d3f0e`

**Método.** El mismo que en la pasada anterior, para poder comparar. `GameSession.crear` con semilla 7, 20 Facciones
creadas con el comando real `crearFaccionNpc`, y por tick la secuencia del runner: `avanzarTick` →
`avanzarAutoComercio` → `avanzarFaccionesNpc`. 4 500 ticks (~3 días de mundo), con perfil de CPU en dos ventanas de
300 ticks (desde el 1 500 y desde el 4 500). Límites: la máquina del contenedor y un mundo solo de NPC, todo en nivel
1. Esta vez **sí hay ejércitos** (8-13 columnas) y trueques. Dice qué pesa y qué crece, no cifras de una partida
madura con jugadores.

**Evolución del tick completo:**

| | `8091638` (26-sep), t1 500 → t4 500 | `70d3f0e` (hoy), t1 500 → t4 500 |
|---|---|---|
| Motor | 8,0 → 7,6 ms | 9,6 → 9,9 ms |
| Turno NPC | 21,6 → **37,8** ms | 5,5 → 7,3 ms |
| **Total** | ~30 → ~45 ms | **~15 → ~17 ms** |

**Reparto actual (ms/tick, tiempo inclusivo):**

| | desde t1 500 | desde t4 500 | Nota |
|---|---|---|---|
| **`exito()`: copia del log de eventos** | 2,0 | **4,0** | lo único que crece de verdad (**N2**) |
| `avanzarConstruccion` | 4,7 | 4,0 | el grueso del motor; ya memoizado |
| `reclutarParaTodos` (NPC) | 2,4 | 2,6 | antes 19-20 |
| `grabarLoVisto` | 1,0 | 1,0 | |
| `calcularTitulos` | 0,4 | 0,4 | |
| `avanzarEjercitos` | 0,4 | 0,4 | |
| `publicarOrdenesNpc` | 0,1 | 0,3 | antes 0,4 → 12,5 |
| `prepararDefensaNpc` + `guarnecerNpc` | 0,4 | 0,4 | |
| `avanzarSpawnBandidos` | 0,2 | 0,2 | |
| `computeTodasLasZonas` | 0,1 | 0,1 | |
| `avanzarTecnologia` | 0,05 | 0,09 | nueva |
| `acogerHeroesNpc`, `ocuparConquistas`, Paso 0 | ≤0,05 | ≤0,05 | nuevas o cambiadas; sin coste |

**Colecciones** (estado guardado en t1 500 → t4 500):

| | t1 500 | t4 500 | |
|---|---|---|---|
| Órdenes vivas | 99 (todas activas) | 520 (todas activas) | ✔ las cerradas ya no están |
| `historialOrdenes` | 302 | 6 350 | crece; se copia entero cuando cierra alguna |
| Trueques en la lista viva | 21 (19 activos) | 23 (**19 expirados**, 4 cumplidos) | los terminados no salen |
| Eventos en memoria | 14 955 | **59 235** (~13 por tick; 16,7 MB en JSON) | **N2** |

**De qué está hecho el log a los 3 días:** `ejercito.reabastecido` 39 %, `npc.accion` 17 %,
`construccion.yacimiento_agotado` 12 %, `mapa.yacimiento_regenerado` 12 %, `mercado.orden_expirada` 11 %. El resto,
menos del 2 % cada uno.

**Reclutamiento NPC hoy**, sobre el estado del tick 4 500: se omiten sin intentarlo 2 100 tropas sin tecnología. De
los 400 intentos que quedan, 100 tienen éxito; fallan 100 por falta de Galería de tiro, 92 por escuadra al tope y 100
por falta de artesanos.

---

## 4. Qué cambió desde la pasada anterior

| Punto de la pasada anterior | Estado en `70d3f0e` |
|---|---|
| **B0** · todos los campamentos con id `campamento-0` | ✅ **Arreglado.** El id es `campamento-${asentamiento.id}` (`bandidos.ts:75`); uno por plaza |
| **B1** · el campamento huérfano bloqueaba el spawn de plazas nuevas | ✅ **El bloqueo, arreglado**: la guarda es ahora por asentamiento (`bandidos.ts:69`). 🔶 **El huérfano sigue en el mundo**, y con B3 se reengancha (§9) |
| **N1** · el NPC intentaba ~1 050 reclutamientos por tick y fallaba el 99,5 % | ✅ **Casi resuelto**: salta las tropas sin tecnología (`npcGobernanza.ts:872`), sin mover ids. Queda un residuo (§7) |
| **N2** · órdenes expiradas recorridas cada tick | ✅ **Arreglado para órdenes**: salen a `historialOrdenes` (`market.ts:90`, `simulation.ts:355`) y `publicarOrdenesNpc` usa un `Set` (`npcGobernanza.ts:1309`). 🔶 Los trueques terminados y el **log de eventos** siguen igual |
| **O4** · "la tecnología, que nazca por hecho" | ✅ **Así se hizo**: contadores en el hecho, también desde los comandos (`exito`, `tipos.ts:103`) |
| **C** · el NPC re-fundaba cada tick a la Facción que perdía su última plaza | 🔶 **Cambió**: si la derrota otra NPC, se anexiona; si colapsa sola, se disuelve (`acogerHeroesNpc`). Solo se refunda si la derrotó un jugador. **Sigue siendo un barrido** (§6, **R**) |
| **A** · nivel de Facción re-derivado al final del tick | ❌ Sin cambios |
| **B** · dependientes de una ruina descubiertos barriendo | ❌ Sin cambios de fondo. Nuevo: la ruina ya registra la derrota en el acto (`simulation.ts:250`) |
| **B2** · penalización por la ruina del socio | ❌ Se reproduce igual |
| **B3** · id de asentamiento reutilizable | ❌ Se reproduce igual |
| **O1** · los ticks del reloj no se difunden por WebSocket | ❌ Sin cambios |
| **O2** · lo que sobrevive a una conquista | ❌ Sin cambios |

---

## 5. Inventario por responsabilidad

Orden de `avanzarSimulacion` (`engine/simulation.ts:126`), más lo que corre alrededor en cada tick. **1** legítima ·
**2** mal asignada · **3** dudosa · ✔ ya disparada por el hecho.

### 5.1 Motor (`engine/`)

| # | Responsabilidad | Ubicación | Cat. | Nota |
|---|---|---|---|---|
| 1 | Zonas, capitales y reclamos del mundo | `simulation.ts:131,134,141` | 3 | derivado; ~0,1 ms, no se toca |
| 2 | Construcción: obras que terminan, producción, cola, recetas, mejoras | `construction.ts:1270` (`avanzarConstruccion`), `1099` (`avanzarMejoras`) | 1 | fechas, tasas y stock |
| 3 | Auto-construcción | `construction.ts:541` (`evaluarNecesidades`) | 1 | umbrales; memoizada |
| 4 | Contadores de producción para la tecnología | `simulation.ts:170` | ✔ | se suman al producir |
| 5 | XP por edificio completado y por ascenso | `simulation.ts:171-186` | ✔ | |
| 6 | Caducidad de políticas | `politicas.ts:76` | 1 | `expiraEn` |
| 7 | Fin de la obra de ascenso | `ascenso.ts:203` | 1 | gates y cupo **al pedirla** ✔ |
| 8 | Nutrición, ración de la guarnición, crecimiento, recaudación | `population.ts:181,49`, `tropas.ts:281`, `simulation.ts:202` | 1 | tasas |
| 9 | Mantenimiento | `mantenimiento.ts:202` | 1 | |
| 10 | Fin de la ocupación y de la protección | `simulation.ts:214-225` | 1 | lógica perezosa (`estaOcupado`, `estaProtegida`); el barrido limpia y narra |
| 11 | Ruina | `simulation.ts:246-250` | ✔/2 | derrota registrada en el acto ✔; **dependientes por barrido**: **(B)** |
| 12 | Comercio: movimiento de caravanas, caducidad de trueques, asignación automática | `trade.ts:504`, `902`, `1040` | 1/3 | trueques terminados siguen en la lista viva: **N2** |
| 13 | Red de caminos: registrar ruta al lanzar, podar caducadas | `redCaminos.ts:53,72` | ✔ | registrada en el hecho, derivados cacheados por identidad |
| 14 | Caravanas de fundación | `expansion.ts:169` | ✔ | |
| 15 | Regeneración de yacimientos | `simulation.ts:264`, `world/mapa.ts:422` | 1 | fecha `regeneraEn`; los eventos son ciclos reales, no ruido |
| 16 | Spawn de bandidos | `bandidos.ts:59` | 1/3 | plazo por plaza ✔; el huérfano: **B1** |
| 17 | Ataques de bandidos | `bandidos.ts:104` | 1 | |
| 18 | Ejércitos: comer, mover, **repostar**, llegar, encuentros | `ejercitos.ts:1436` | 1 | reponer cada tick es legítimo; **narrarlo cada tick no** (**N2b**) |
| 19 | Disolver columna sin nadie dentro | `ejercitos.ts:1523` | 3 | invariante multi-camino; no mover |
| 20 | Columna esperando a la puerta | `ejercitos.ts:1606` | 3 | **(J)** |
| 21 | Caducidad de órdenes, que pasan al historial | `market.ts:90`, `simulation.ts:355` | 1 ✔ | |
| 22 | Tributo, reputación | `diplomacia.ts:224`, `reputacion.ts:26` | 1 | tasas |
| 23 | **Nivel de Facción a partir de la XP** | `simulation.ts:324-326` | **2** | **(A)** |
| 24 | Títulos | `simulation.ts:330-332`, `titulos.ts:26` | 3 | **(E)** |
| 25 | Tecnología: récords de plazas, logros, Era, aparición | `simulation.ts:337-346`, `tecnologia.ts:233` | 1/3 | §7 |
| 26 | Memoria de niebla y exploración personal | `simulation.ts:362-370`, `memoria.ts:103` | 3 | **(H)**, no mover |

### 5.2 Sesión (`session/`) y servidor (`server/`)

| # | Responsabilidad | Ubicación | Cat. | Nota |
|---|---|---|---|---|
| 27 | Cerrar batallas vencidas; bloqueos | `comandos/avanzarTick.ts:37-38`, `batallas.ts:509,73` | 1 | `expiraEn`; filtra también las terminadas (sin medir) |
| 28 | Abrir combates del tick | `avanzarTick.ts:70` | ✔ | |
| 29 | **Anteponer eventos al log en memoria** | `comandos/tipos.ts:100` (`exito`) | **2** | **N2**; además cuenta logros (✔) |
| 30 | Turno NPC | `npcGobernanza.ts:2056` | mixto | §5.3 |
| 31 | Reloj de mundo, precios, geometría, proyección, mantenimiento de disco | `server/` | 1/✔ | derivan al leer o son infra |

### 5.3 NPC de gobernanza (`npcGobernanza.ts:2056`)

**Casi todo lo que decide es política por umbrales de variables continuas**: un bot que mira cada minuto lo que un
jugador miraría de vez en cuando, y eso no está mal asignado. Las excepciones con forma de *reacción a un hecho* son
estas:

| Paso | Ubicación | Hecho al que reacciona | Cat. |
|---|---|---|---|
| **`acogerHeroesNpc`** (anexionar, disolver, dar casa) | `1530`, llamada en `2075` | derrota de la última plaza; héroe que se queda sin casa | **2 → R** |
| Paso 0: fundar el asentamiento inicial | `2079`, `fundarAsentamientosIniciales` `1979` | Facción NPC derrotada por un jugador | 2 → **C** (ya estrecho) |
| `ocuparConquistas` | `1591` | una columna NPC conquistó una plaza | 3: detectado por geometría ("plaza propia sin residentes con columna a la puerta"); reacción de agente, aceptable |
| Reclutar | `850` | obra terminada, escuadra con bajas, población | 1, con residuo de 2 (§7) |
| `asegurarGobernanzaBase` | `279` | plaza fundada o conquistada, cargo vacío | 3, invariante multi-camino; no mover |
| `materializarFundadoresNpc` | `1863` | fundación NPC | ✔/no mover (capas) |
| `responderPropuestasNpc` | `632` | propuesta recibida | 3 → **(I)** |
| `guarnecerNpc` / `prepararDefensaNpc` | `909`, `933` | escuadra nueva, vuelta a casa, residencia | 3 → **(F)** |
| `replegarLosQueYaTerminaron` | `1756` | columna acampada | 3 |
| Granjas, comercio, núcleo militar, muralla, ascenso, trueques, órdenes, bandidos, campañas, persecuciones, expandir | varios | umbrales o geometría | 1 |

---

## 6. Categoría 2 — mal asignadas

Formato de cada ficha: ubicación · hecho · dónde se produce · qué cambia al moverlo · riesgo · esfuerzo.

### (R) `acogerHeroesNpc`: el patrón de la residencia, vivo en `main`

> **Actualizado en la cuarta pasada (§0):** con `9f9a098` en `main`, este barrido ya no solo está mal asignado; choca
> con la residencia en campamentos y produce el bug **B5**. Lo que sigue describe el barrido tal como era antes de `9f9a098`.

- **Ubicación.** `npcGobernanza.ts:1530-1588`, al principio de cada turno NPC (`2075`).
- **Qué hace cada turno**, con tres consecuencias de hechos distintos:
  1. **Anexionar.** Toda Facción NPC con `derrotadaPor` = otra NPC y sin plazas se une a la ganadora (`anexionar`,
     `fusion.ts:36`); sus columnas cambian de bandera y sus relaciones se disuelven.
  2. **Disolver.** Toda Facción NPC con `derrotadaPor: null` (colapso) y sin plazas desaparece, **con sus héroes bot y
     sus columnas**.
  3. **Dar casa.** Todo héroe bot sin residencia pasa a residir en la plaza más cercana de su Facción, y su columna
     cambia de origen. Recorre todos los héroes contra todas las plazas (`esResidente`).
- **Dónde se producen esos hechos.** La derrota de la última plaza ya tiene sitio:
  - `registrarDerrota` (`faccion.ts:119`), llamada desde la ruina (`simulation.ts:250`), el asedio de un ejército
    (`ejercitos.ts:1257`) y la batalla de Unity (`resultadoBatalla.ts:256`). Pero **no** desde el comando
    `iniciarAsedio` (`comandos/militar.ts:108-148`): ver **B4**.
  - El héroe que se queda sin casa sale de `desalojarResidentes` (`combate.ts:271`, que ya lo reubica en el acto), de la
    anexión del punto 1 y de la ruina.
- **Por qué está mal asignado**, con los tres síntomas de la petición:
  - *Ventana:* `avanzarFaccionesNpc` **salta a las Facciones con una batalla activa** (`comandos/avanzarFaccionesNpc.ts:26-27`).
    Mientras dura la batalla de Unity, sus héroes sin casa siguen sin casa y su Facción vacía sigue en pie: la ventana
    no es un tick, es lo que dure la batalla.
  - *Mezcla de casos:* el punto 3 "recoge a los que quedaban fuera, incluidos los recién absorbidos", según su propio
    comentario. Es decir, el barrido existe para cubrir caminos que nadie enumeró.
  - *Dependencia frágil:* los puntos 1 y 2 solo funcionan si `derrotadaPor` se escribió, y un camino no lo escribe
    (**B4**). Por ese camino la Facción NPC no se anexiona ni se disuelve: el Paso 0 la refunda gratis.
- **Hacia dónde moverlo.** Los tres hechos son enumerables y ya tienen sitio. Anexionar o disolver va donde se
  llama a `registrarDerrota` (o dentro de ella, para la parte de motor). Dar casa va en `desalojarResidentes`, que ya lo
  hace, y en la anexión. Es exactamente lo que se hizo con la residencia.
- **Qué se rompería.**
  - *Capas:* anexionar y disolver son decisiones del NPC (`session/`). Dispararlas desde `engine/` exige que el motor
    devuelva el hecho ("Facción X derrotada por Y") y que la sesión lo aplique, como ya hace con `combatesPorAbrir`.
  - *Batch:* el NPC corre en batch sin `faccionesIds`, así que el momento de la anexión se mueve. Hoy ocurre en el
    turno NPC del mismo tick, así que el estado final por tick suele coincidir; el orden de eventos no.
  - `snapshot_baseline` y `determinismo` no lo ven.
- **Riesgo:** medio (toca capas y tres caminos). **Esfuerzo:** M. **Impacto:** medio. No es coste: es corrección, y es
  el caso que originó la petición.

### (N2) El log de eventos en memoria: se copia entero en cada mutación, y crece sobre todo por ruido

- **Ubicación.** `exito()` (`comandos/tipos.ts:92-106`) hace `[...nuevos, ...estado.eventosDominio]` en cada mutación
  (tick y turno NPC; el auto-comercio apagado no muta). El log en memoria no tiene cota; el persistido va aparte, al
  JSONL.
- **Medido.** 2,0 → 4,0 ms/tick entre t1 500 y t4 500, ya ~25 % del tick completo y lineal con la edad. 59 235 eventos
  y 16,7 MB a los 3 días con 20 plazas. Extrapolando, solo orientativo: a 30 días serían ~560 000 eventos y unos
  40 ms/tick, solo en copiar.
- **Dos hechos distintos que resolver:**
  - **N2a · retención.** El hecho es "este evento ya lo leyó todo el que lo necesitaba". El log en memoria sirve de
    cursor incremental a los clientes (`eventosDesde`), y el histórico completo ya está en el JSONL. Basta con una cota
    o una estructura que no se copie entera: anteponer a un array de 60 000 elementos es O(n) por mutación.
  - **N2b · ruido.** El **39 %** del log es `ejercito.reabastecido`. Repostar cada tick es legítimo (Doc 5.13, "sostener
    un paso de montaña"), pero **narrarlo cada minuto** convierte un proceso continuo en un hecho falso. El hecho real
    es "empieza a reabastecerse aquí" o "repone una cantidad que importa". `npc.accion` es otro 17 %, en texto libre.
- **Qué se rompería.** Con una cota, un cliente con un cursor más viejo que la cota lee del JSONL. Si se dejan de emitir
  eventos de reabastecimiento, `determinismo` (eventos por tick) y los tests de eventos de ejércitos lo detectan, y
  los diarios de batch que los cuenten dejan de cuadrar.
- **Riesgo:** bajo-medio. **Esfuerzo:** S para el ruido; S-M para la retención. **Impacto:** **alto a medio plazo**:
  es el único coste que crece sin cota.

### (A) El nivel de Facción se recalcula al final del tick, aunque la XP se gane en un comando

- **Ubicación.** `simulation.ts:324-326`; `faccion.ts:56` (`aplicarAjustesExperiencia`) y `67`
  (`avanzarNivelesFaccion`).
- **Hecho.** Ganar experiencia; el nivel es función pura de ella (`calcularNivelFaccion`, `faccion.ts:27`).
- **Dónde se produce.** `combate.ts:430, 498, 592, 635, 681` (asedio desde plaza, campamentos de bandidos, asedio con
  ejército, encuentros) y ahora también `trade.ts:1086` (comercio). Los de combate se invocan desde comandos o el NPC;
  ningún comando llama a `avanzarNivelesFaccion`.
- **El defecto.** El comando devuelve la XP nueva con el **nivel viejo**; el nivel y `faccion.nivel_subio` llegan en
  el tick siguiente. En ese hueco leen `faccion.nivel` el cap de fundación (`settlement.ts:263`, `expansion.ts:98`), los
  slots de política (`politicas.ts:60`), el cupo de nivel (`ascenso.ts:85`) y la Gran Fundición
  (`construction.ts:1686`).
- **Qué se rompería.** Nada del pipeline. `eventosDominioFaccion.test.ts` habría que repuntarlo. El batch no queda
  byte-idéntico si hay combate NPC.
- **Riesgo:** bajo. **Esfuerzo:** S. **Impacto:** medio-bajo.

### (B) Una ruina deja a sus dependientes para que el tick los descubra

- **Ubicación.** `simulation.ts:246-250`: un único sitio, que ya registra la derrota en el acto.
- **Dependientes y cómo se enteran hoy:**

  | Dependiente | Cómo se entera | Resultado |
  |---|---|---|
  | Caravanas que salen de o van a la plaza | `trade.ts:552`, barrido por caravana | se pierden sin evento |
  | Caravana de fundación | `expansion.ts:169+` | se pierde con evento |
  | Rutas de la red de caminos | `podarRutas` (`redCaminos.ts:72`), cada tick | ✔ correcto y barato |
  | **Campamento de bandidos asignado** | **nunca** | huérfano: **B1** |
  | **Trueque activo con ella** | al vencer (`trade.ts:931`) | penaliza a la otra parte: **B2** |
  | Ejército con ella de origen | al volver (`reintegrar`, `ejercitos.ts:1481`) | se resuelve tarde |
  | **Referencias por id** | **nunca** | se reenganchan si el id renace: **B3** |
  | Héroes de una Facción NPC que colapsa | `acogerHeroesNpc` en el turno NPC | **R** |

- **Qué se rompería.** Adelanta en el tick cosas que hoy pasan en el paso de comercio (va después, `simulation.ts:252`).
  Reordena eventos y el batch deja de ser byte-idéntico, porque las ruinas son frecuentes en los diarios.
- **Riesgo:** medio. **Esfuerzo:** M. **Impacto:** medio (arregla B1-B3 de raíz).

### (C) Re-fundar la Facción NPC derrotada por un jugador

- **Ubicación.** `npcGobernanza.ts:2079` (Paso 0), `fundarAsentamientosIniciales` (`1979`).
- **Estado.** Ya es estrecho: solo afecta a Facciones NPC sin plazas que no se hayan anexionado ni disuelto, es decir,
  derrotadas por un jugador o **por el camino de B4**. Sigue mirándose cada tick, pero es una resurrección deliberada.
  Lo que queda es decidir si es inmediata o con retardo, y tapar B4.
- **Riesgo:** bajo. **Esfuerzo:** S. **Impacto:** bajo.

---

## 7. Categoría 3 — dudosas

| | Qué | Medido | Veredicto |
|---|---|---|---|
| **Tecnología** | `avanzarTecnologia` (`tecnologia.ts:233`) recalcula cada tick el récord de plazas en nivel 2/3, comprueba los logros y evalúa los **hitos de aparición** de cada Facción. Casi todos los hitos son discretos (edificio, tecnología adoptada, capital en nivel, yacimiento en territorio); uno es continuo (recurso en la capital) | 0,05-0,09 ms | **dejar**. El propio código acepta un tick de retraso a propósito ("un tick de retraso no cambia ningún hito", `simulation.ts:341`), y al haber un hito continuo, el barrido es la forma sencilla de no perder ninguno |
| **Reclutamiento NPC, residuo** | De 400 intentos por tick, 100 fallan por falta de Galería de tiro (hecho discreto: obra terminada) y 92 por escuadra al tope | ~2,5 ms | descartar también esas antes de intentar, igual que ya se hace con la tecnología. Menor |
| **Trueques terminados en la lista viva** | 19 de 23 expirados en t4 500; se recorren cada tick en comercio y en el NPC | sin coste medible hoy | mismo trato que las órdenes cuando crezca |
| **`historialOrdenes`** | Se copia entero (`simulation.ts:355`) cuando cierra alguna orden; 6 350 en t4 500 y viaja en cada snapshot | despreciable en CPU | vigilar el tamaño del snapshot, no el tick |
| **(E) Títulos** | Recalculados cada minuto sobre oro y tropa, y narrados en cada cambio de manos | 0,4 ms | dudoso por ruido (pueden oscilar); cadencia o histéresis declarada. No es un evento |
| **(F)** | NPC: guarnición y loadout reescritos cada tick | 0,4 ms | hacerlo idempotente; no mover |
| **(H)** | Memoria de niebla | 1,0 ms | no mover (§8) |
| **(I)** | NPC responde propuestas en su turno | ~0 | decisión de comportamiento |
| **(J)** | Columna esperando a la puerta a que sane un héroe o acabe una batalla | ~0 | sanar no tiene punto de disparo; dejar |

---

## 8. Lo que NO conviene mover, y por qué

| Qué | Por qué se queda |
|---|---|
| **Caducidades por fecha** (políticas, trueques, órdenes, obras, mejoras, ascenso, ocupación, protección, batallas, regeneración, respawn por plaza, rutas de la red) | Ya son eventos baratos: la fecha vive en la entidad (Doc 10 §6). Un scheduler sería una segunda fuente de verdad y sacaría efectos del punto exacto del pipeline. Revisar solo si llega el scheduler de comandos programados (D5) |
| **Tasas** (producción, consumo, crecimiento, mantenimiento, ración, tributo, reputación, **repostar**) | Continuas. Lo que sí sobra es **narrar** el reabastecimiento cada minuto (N2b) |
| **Auto-construcción, mejoras automáticas, recetas, muralla** | Umbral de stock; memoizadas |
| **El NPC por umbrales** | Es el agente mirando variables continuas. Moverlo rompe su principio rector y el batch. Medido, ya no pesa |
| **`asegurarGobernanzaBase`, disolver columnas vacías** | Sostienen un invariante con caminos de ruptura abiertos. La diferencia con **R** es que allí los caminos sí son enumerables |
| **`materializarFundadoresNpc`** | Crear el héroe bot en la fundación obligaría al motor a crear héroes bot (cruza capas); va en el mismo persist que el tick |
| **Memoria de niebla** | `conocidoEn` es "la última vez que la vi"; mantenerla exige escribir mientras se mira. La proyección ya cubre la ventana en vivo |
| **Red de caminos** | Ya está bien: se registra al lanzar la caravana, los derivados se cachean por identidad y `podarRutas` devuelve la misma red si nada cambia |
| **Regeneración de yacimientos** | Fecha `regeneraEn`; los eventos `agotado`/`regenerado` son ciclos reales (10 min metales, 3 min ganado), no ruido |
| **Proyección, precios, geometría, `heridoHasta`, `ocupacionHasta`, `protegidaHasta`** | Ya derivan al leer o son perezosos |

**Dónde el principio ya está bien aplicado** (útiles como ejemplo): el ascenso se valida y reserva **al pedirlo**; el
asedio es una orden; la conquista con `aplicarConquista` + `desalojarResidentes` + `registrarDerrota` en el acto; la
ruina registra la derrota en el acto; los contadores de tecnología se suman en el hecho, también desde comandos; el
plazo de bandidos se fija al destruir el campamento (`agendarReaparicionBandidos`, `bandidos.ts:92`); la red de
caminos se registra al lanzar.

---

## 9. Bugs y observaciones aparte

Los bugs marcados *reproducido* se han comprobado con scripts sobre el motor o la sesión reales de `70d3f0e`.

**B1 · El campamento de una plaza arruinada no se retira nunca, y lo hereda quien refunde en el mismo sitio.**
*Reproducido:* 3 plazas con 3 campamentos; cae `asentamiento-2-120-1440`; 60 ticks después
`campamento-asentamiento-2-120-1440` sigue en el mundo (y atacando caravanas, que `avanzarAtaquesBandidos` recorre todos).
Si se refunda en el mismo punto, la plaza nueva recibe el mismo id (B3), y como el id del campamento sale del de la
plaza, **el huérfano pasa a ser su campamento** y le bloquea el spawn propio. El NPC no lo ataca porque su plaza no
existe.

**B2 · Penalización de reputación por la ruina del socio.** *Reproducido con control:* trueque de 50 de madera contra
10 de piedra, bandidos desactivados. **Sin ruina**, A entrega 50/50 y su reputación queda en 0. **Con la ruina del
socio**, el trueque sigue `activo`, A entrega 0/50 y al vencer cae a −8 (−3,6 tras el decaimiento),
`trade.ts:931-945`.

**B3 · Un id de asentamiento puede renacer.** `asentamiento-${asentamientosExistentes.length}-${x}-${y}`
(`settlement.ts:280`). *Reproducido:* arruinada `asentamiento-2-120-1440` y refundada en el mismo punto da el mismo id.
Se reenganchan en silencio el campamento (B1), los trueques aún activos, las rutas `origen>destino` de la red si no se
han podado, el origen de ejércitos y caravanas, la ficha de memoria y el historial. La búsqueda de posición del NPC es
determinista, así que repetir sitio es probable.

**B5 · Dos mecanismos para la misma consecuencia: `acogerHeroesNpc` pisa la residencia en campamentos de `9f9a098`.**
*Reproducido* en la sesión real de `b452315` (detalle en §0). Con la ruina de la última plaza de una Facción NPC, sus héroes
se borran pero quedan como residentes fantasma de un campamento de mercenarios. Con la anexión NPC contra NPC, sus bots
residen a la vez en una plaza y en un campamento. Por lectura, lo mismo pasa al refundar en el Paso 0. Es el riesgo que
motivó toda la auditoría, materializado: arreglar una consecuencia en el hecho sin retirar el barrido que la cubría antes.

**B4 · El comando `iniciarAsedio` conquista sin registrar la derrota.** `comandos/militar.ts:108-148` llama a
`desalojarResidentes` pero no a `registrarDerrota`, a diferencia del asedio con ejército (`ejercitos.ts:1257`) y de la
batalla de Unity (`resultadoBatalla.ts:256`). Si por ese camino cae la **última** plaza de una Facción, queda sin
`derrotadaPor`. Si es NPC, `acogerHeroesNpc` no la anexiona ni la disuelve y el Paso 0 la refunda gratis; si no lo es,
nada la marca como derrotada. Hallazgo **por lectura, no reproducido**. Es el ejemplo concreto de por qué un barrido
que depende de un campo escrito "en cada camino" es frágil.

**O1 · Los ticks del reloj de mundo no se difunden por WebSocket.** `HubDeDifusion.difundir` solo se llama desde
`rutas/comandos.ts:151`, `rutas/batallas.ts:156` y `rutas/admin.ts:418`; el camino del reloj
(`runnerDePartida.ts`, `sincronizarConReloj` → `unTickCompleto`) persiste y anexa al JSONL, pero no avisa a nadie. Un
hecho sin su consecuencia.

**O2 · Lo que sobrevive a una conquista.** `aplicarConquista` (`combate.ts:199-253`) conserva `politicasActivas`,
`reservaManual` y `autoConstruccionPausada` del dueño derrotado. `Ocupacion_Post_Conquista_Definicion.md` no lo trata.

**O3 · Excepciones como control de flujo en el NPC.** Siguen en reclutar (residuo), `anadirEdificioManualmente`,
`comprometerRecintoManualmente` y `asignarGuarnicion`. Hoy suman <1 ms; conviene no añadir más pasos así.

---

## 10. Tabla priorizada y orden recomendado

| # | Qué | Cat. | Impacto | Riesgo | Esfuerzo | Batch byte-idéntico | `snapshot_baseline` / `determinismo` |
|---|---|---|---|---|---|---|---|
| **B5 + R** | Terminar `9f9a098` en el NPC: anexión y disolución en el hecho, conscientes de los campamentos; quitar "dar casa"; el Paso 0 saca a sus fundadores del campamento | bug + 2 | **alto** (estado inconsistente: fantasmas y doble residencia) | medio | M | no (orden de eventos del NPC) | no / no |
| **B4** | `iniciarAsedio` registra la derrota | bug | medio | bajo | S | sí (el batch no usa ese comando) | no / no |
| **N2b** | No narrar el reabastecimiento cada minuto | 3→2 | **alto** (39 % del log) | bajo | S | no (eventos) | no / **sí** |
| **N2a** | Log en memoria con cota o sin copia O(n) | 2 | **alto** a medio plazo | bajo-medio | S-M | sí | no / no |
| **R** | `acogerHeroesNpc` en los hechos: derrota y desalojo | 2 | medio (corrección; el caso origen) | medio | M | no (orden de eventos) | no / no |
| **B1 + B3** | Retirar el campamento al arruinarse; ids de plaza que no renazcan | bug | medio | medio (formato de id) | S | no (ids) | según dónde salga el id |
| **B2** | Cerrar trueques con una plaza arruinada sin penalizar | bug | medio | bajo | S | solo con ruinas | no / no |
| **A** | Nivel de Facción al ganar la XP | 2 | medio-bajo | bajo | S | no con combate NPC | no / no |
| **B** | Cerrar los dependientes de la ruina en el sitio de la ruina (engloba B1-B2) | 2 | medio | medio | M | no | no / según orden |
| Reclutamiento, residuo | Descartar antes falta de edificio y escuadra al tope | 3 | bajo (~2 ms) | bajo | S | no (si no conserva ids) | no / no |
| **C, E, F** | Re-fundación, títulos, idempotencia del NPC | 2/3 | bajo | bajo | S | — | E toca `determinismo` |
| Tecnología, H, I, J, §8 | — | — | despreciable o por diseño | — | — | — | no tocar |

**Orden recomendado.**

1. **B4 y B5 + R, juntos.** B4 es una línea y hace que los cuatro caminos de conquista escriban la derrota; con eso, la
   anexión y la disolución pueden colgar de `registrarDerrota` y `acogerHeroesNpc` puede desaparecer. Es terminar `9f9a098`.
2. **N2b y N2a.** Es lo único que crece sin cota, y la mayor parte es ruido de un proceso continuo narrado por minuto.
3. *(antes "R", ahora dentro del punto 1)*
4. **B1 + B3 y B2**, o directamente **B** si se quiere arreglar de raíz.
5. **A.** El más limpio del principio y de riesgo bajo; puede ir antes si se quiere un ensayo pequeño.
6. **El resto** cuando haya hueco. Tecnología, memoria de niebla, red de caminos, regeneración y todo §8: dejar.

**El criterio, como queda tras esta tercera pasada:**

- **Mover al hecho** cuando los hechos son pocos y enumerables y cada uno ya tiene un sitio de escritura: residencia,
  derrota de la última plaza, nivel de Facción, ruina, paso a estado terminal. Cuando se mueva, **todos** los caminos
  tienen que escribir el hecho: B4 enseña lo que pasa si uno se queda atrás.
- **Mantener el barrido** cuando sostiene un invariante con caminos abiertos, o cuando alguna de sus condiciones es
  continua (hitos de tecnología con recurso en la capital).
- **Comprobar barato antes, o memoizar con clave**, cuando lo que se recalcula es una lectura.
- **No narrar como hecho un proceso continuo**: si algo ocurre cada minuto, no es un evento (N2b).
