# Auditoría del tick: qué se evalúa cada minuto y qué debería dispararse por hecho

**Abierto el 2026-10-02**, sobre `8091638` (`origin/main`). Sale de la corrección de la residencia en campamentos:
`acogerHeroesSinCasa` recorría todos los héroes en cada tick para reubicar a quien se había quedado sin casa, y era
una **consecuencia de un hecho puntual** (cae una plaza, se arruina, alguien deja su casa) que se resolvía tarde,
barriendo. Aquí se pregunta lo mismo del resto del tick.

Es un **informe para decidir, no un plan**. Solo lectura: no se ha tocado código ni se han ejecutado tests (el
árbol no tiene `node_modules`); todo lo que sigue sale de leer `src/` y de las medidas ya escritas en
`Docs/Arquitectura/6_Sincronizacion_Visibilidad_y_Escala.md`.

> **Aviso de alcance.** El commit `9f9a098` (campamentos de mercenarios, `desalojarResidentes` de ruina,
> `reubicarResidentesDeRuina`, `dejarResidencia`) **no está en el árbol auditado** — ni en `origin/main` ni en
> ninguna de las ramas remotas. En este árbol `desalojarResidentes` solo existe para la conquista
> (`engine/combate.ts:226`) y la ruina (`engine/simulation.ts:224-227`) no reubica a nadie. Lo de residencia se da
> por resuelto tal como lo describe la petición y no se audita.

---

## 1. Conclusiones en seis líneas

1. **La gran mayoría del tick es legítima.** De las 45 filas del inventario, más de 30 son dinámica continua,
   caducidades por fecha o hechos ya disparados donde ocurren (§3). El tick ya está bien orientado: casi todo lo que caduca lleva una **fecha absoluta
   en la entidad** (`completaEn`, `expiraEn`, `preparaHasta`…), que es la forma de "evento" barata de este motor.
2. **Hay pocas mal asignadas y casi todas son pequeñas**: tres en categoría 2 (§4) y siete en la 3 (§5). La más
   clara es el **nivel de Facción**, que se recalcula al final de cada tick aunque la XP se gane en un comando.
3. **El coste no es el argumento.** Un tick a 100 asentamientos son ~62 ms sobre 60 000 ms de intervalo (0,10 %,
   Doc 6 §1). Lo que justifica mover algo es **corrección** —ventana entre hecho y tick, mezcla de casos, evento
   que sale en el minuto equivocado—, no ahorrar CPU. Donde sí hay coste oculto probable está en §5 (D) y (G), y
   se resuelve midiendo y memoizando, no con eventos.
4. **No conviene mover** las caducidades por fecha, el NPC por umbrales, la memoria de niebla ni los barridos que
   sostienen un invariante con muchos caminos de ruptura (§6). Mover lo equivocado cambia el orden de evaluación
   del tick y el batch deja de ser comparable.
5. **Dos bugs claros** y tres observaciones aparte (§7). Ninguno depende de mover nada: se arreglan solos.
6. **Orden recomendado** en §8: bugs → nivel de Facción → medir perfil por etapa → cierre de una ruina → resto.

---

## 2. Cómo se ha clasificado

Tres categorías, las de la petición:

- **1 · Legítima del tick.** Dinámica continua o periódica por naturaleza: producción, consumo, crecimiento,
  mantenimiento, movimiento, caducidad por fecha, reposición programada.
- **2 · Mal asignada.** Es la consecuencia de un hecho discreto (comando, conquista, obra terminada, llegada) y se
  detecta recalculando o barriendo en vez de dispararse cuando ocurre.
- **3 · Dudosa.** Depende de cuántas veces se mire, de una ventana poco clara, o mezcla un caso real con otros.

Cinco preguntas para decidir si algo de la 2 o la 3 **debe** moverse. Si alguna sale mal, no se mueve:

1. **¿Hay un hecho identificable y enumerable?** Si la consecuencia puede venir de cuatro caminos distintos y mañana
   de un quinto, el barrido es el invariante; un evento por camino se deja uno atrás.
2. **¿La consecuencia es escribir estado/evento o solo leer algo derivado?** Si es lectura derivada, la herramienta
   correcta es la **memoización con clave** (como las tres de Doc 6 E3) o derivarlo al leer (como `estaHerido`,
   `estaOcupado`, la proyección), no un evento.
3. **¿Qué ventana abre el barrido?** Hasta un tick (1 min de mundo) es casi siempre aceptable; lo que no lo es es
   que el *comando* devuelva un estado que contradice su propio efecto.
4. **¿Qué orden de evaluación cambia?** El pipeline de `avanzarSimulacion` es un orden *load-bearing* (comentado
   en `simulation.ts:172-199`: población come antes que tropas, recaudación antes que mantenimiento…).
5. **¿El hecho ya tiene un único sitio de escritura?** Si sí, el coste de mover es bajo; si está repartido por 6
   funciones, el coste lo pone el sitio, no el principio.

Lo que protegería un cambio (referencia para "qué se rompería" en cada ficha):

| Guardián | Qué fija | A qué es sensible |
|---|---|---|
| `engine/__tests__/snapshot_baseline.test.ts` | 1 Facción, 1 asentamiento, semilla 99, cortes 1/10/25/50/100: nivel, población, medidor, nº de escuadras, edificios por tipo/estado, 7 recursos, nivel y reputación de la Facción | **orden del pipeline de construcción/población/mantenimiento**. No ve NPC, combate, conquista, ruina ni comercio |
| `engine/__tests__/determinismo.test.ts` | 80 ticks, `toEqual` del estado final **y de los eventos por tick**, misma semilla | cualquier cambio de **qué evento sale en qué tick**, y todo consumo de RNG |
| `engine/__tests__/invariantes_simulacion_larga.test.ts` | 3 asentamientos, 300 ticks, invariantes por tick | recursos negativos, NaN, medidor fuera de rango |
| Batch (`scripts/run-batch-sim.ts`, sellos SHA-256 de `bench-batch-checkpoint.ts`) | estado completo idéntico byte a byte | **todo**, y en particular el NPC, que sí corre en batch |
| RNG | una tirada por asentamiento y tick en `crecerPoblacion`; `avanzarAtaquesBandidos`; asedios y encuentros | mover algo que consume RNG cambia la secuencia de todo lo posterior |

---

## 3. Inventario por responsabilidad

Orden de `avanzarSimulacion` (`engine/simulation.ts:122`), más lo que corre alrededor. **1** legítima · **2** mal
asignada · **3** dudosa · ✔ ya está disparada por el hecho.

### 3.1 Motor (`engine/`)

| # | Responsabilidad | Ubicación | Cat. | Nota |
|---|---|---|---|---|
| 1 | Zonas, capitales y reclamos de fuentes de todo el mundo | `simulation.ts:127,130,137` | 3 | derivado que se recalcula entero; ver **(G)** |
| 2 | Obra de edificio termina | `construction.ts:1255-1284` | 1 | fecha `completaEn`; los efectos (capacidad, puestos, XP, radio) cuelgan inline del hecho ✔ |
| 3 | Producción de extractores/granjas/leñeras | `construction.ts:1288-1326` | 1 | tasa continua |
| 4 | Arranque de obras en cola cuando hay cuadrilla | `construction.ts:1339-1376` | 1 | limitado por cupo y stock, no por un hecho único |
| 5 | Recetas de transformación | `construction.ts:1184,1381` | 1 | continua; `factorLineaProduccion` ver **(G)** |
| 6 | Mejoras de nivel interno (terminar / empezar) | `construction.ts:1067-1108` | 1 | fecha + stock |
| 7 | Crecimiento de `radioPotencial` por edificio completado | `construction.ts:1401-1404` | ✔ | ya va por el hecho, no por tiempo |
| 8 | Auto-construcción (`evaluarNecesidades`) | `construction.ts:533,1419` | 1 | umbrales de variables continuas; memoizada (Doc 6 E3) |
| 9 | Obra de recintos celda a celda | `muralla.ts:772-815` | 1 | stock + `siguienteCeldaEn` |
| 10 | XP de Facción por edificio completado | `simulation.ts:164-170` | ✔ | por el hecho |
| 11 | Caducidad de políticas | `politicas.ts:74-91` | 1 | `expiraEn` |
| 12 | Fin de la obra de ascenso | `ascenso.ts:191-209` | 1 | `completaEn`; el cupo y los gates ya se comprobaron **al pedirla** ✔ |
| 13 | Nutrición, hambruna | `population.ts:178-228` | 1 | tasa |
| 14 | Ración y deserción de la guarnición | `tropas.ts:269-282` | 1 | tasa |
| 15 | Crecimiento de población | `population.ts:48-155` | 1 | tasa + RNG |
| 16 | Recaudación de oro | `simulation.ts:192-196` | 1 | tasa |
| 17 | Mantenimiento (pago, medidor, degradación, recuperación por racha) | `mantenimiento.ts:184-288` | 1 | racha = acumulador real, no deadline (Doc 10 §6) |
| 18 | Ruina del asentamiento | `simulation.ts:224-227`, `mantenimiento.ts:260-275` | ✔/2 | el hecho se detecta al ocurrir, pero **sus dependientes se descubren barriendo**: **(B)** |
| 19 | Fin de la ocupación | `simulation.ts:203-205,215` | 1/3 | la lógica ya es perezosa (`estaOcupado`); el barrido solo limpia el campo y narra |
| 20 | Movimiento de caravanas, `preparaHasta` | `trade.ts:506-640` | 1 | |
| 21 | Caducidad de trueques | `trade.ts:876-905` | 1 | `expiraEn` |
| 22 | Asignación automática de caravanas a trueque | `trade.ts:860-991` | 3 | tres condiciones mezcladas (acuerdo aceptado, caravana libre, stock) y una es continua; **no mover** |
| 23 | Caravanas de fundación: avanzar y fundar al llegar | `expansion.ts:168-230` | ✔ | la fundación ocurre al cruzar el final de la ruta |
| 24 | Regeneración de yacimientos | `simulation.ts:241` | 1 | reposición por fecha `regeneraEn` |
| 25 | Spawn/respawn de bandidos | `bandidos.ts:70-95` | 3 | **(D)** |
| 26 | Ataques de bandidos a caravanas | `bandidos.ts:115-189` | 1 | proximidad geométrica + RNG |
| 27 | Ejércitos: comer, mover, repostar, llegar | `ejercitos.ts:1413-1680` | 1 | continuo; la llegada que asedia es **one-shot** al cruzar `progreso>=1` ✔ |
| 28 | Disolver columna sin nadie dentro | `ejercitos.ts:1499-1530` | 3 | invariante con varios caminos de ruptura; ya está dentro del bucle de ejércitos; **no mover** |
| 29 | Columna esperando a la puerta (plaza en batalla o héroes heridos) | `ejercitos.ts:1582-1590` | 3 | **(J)** |
| 30 | Encuentros por persecución | `ejercitos.ts:1683+` | 1 | geometría + orden `atacar` |
| 31 | Caducidad de órdenes de mercado | `market.ts:89-104` | 1 | `expiraEn` |
| 32 | Tributo de vasallaje | `diplomacia.ts:173-209` | 1 | tasa por minuto |
| 33 | **Nivel de Facción a partir de la XP** | `simulation.ts:298-300`, `faccion.ts:65-79` | **2** | **(A)** |
| 34 | Reputación: decaimiento y bono de alianza | `reputacion.ts:26-43` | 1 | tasa |
| 35 | **Títulos dinámicos** | `simulation.ts:304-306`, `titulos.ts:26-94` | 3 | **(E)** |
| 36 | Memoria de niebla (`grabarLoVisto`) y exploración personal | `simulation.ts:322-330`, `memoria.ts:96-142`, `ubicacion.ts:269-292` | 3 | **(H)** |

### 3.2 Sesión (`session/`) y servidor (`server/`)

| # | Responsabilidad | Ubicación | Cat. | Nota |
|---|---|---|---|---|
| 37 | Cerrar batallas vencidas | `comandos/avanzarTick.ts:37`, `batallas.ts:509` | 1 | `expiraEn`; candidata natural del scheduler **si llega** (Doc 10 §8, D5 aplazado) |
| 38 | Bloqueos de la batalla activa | `avanzarTick.ts:38`, `batallas.ts:110` | 1 | derivado barato del propio estado |
| 39 | Abrir combates que el tick no resolvió | `avanzarTick.ts:70` | ✔ | sale del hecho (un combate con humano) |
| 40 | Auto-comercio de simulación | `comandos/avanzarAutoComercio.ts` | — | solo batch, palanca apagada |
| 41 | Turno del NPC de gobernanza | `comandos/avanzarFaccionesNpc.ts`, `npcGobernanza.ts:1498` | mixto | tabla propia, §3.3 |
| 42 | Reloj de mundo y catch-up | `runnerDePartida.ts:383-432` | 1 | infra, es la fuente del tick |
| 43 | Precios de referencia (TTL perezoso) y geometría (`cacheGeometria` por identidad) | `runnerDePartida.ts:240-267` | ✔ | **ya derivan al leer**; es el patrón a copiar |
| 44 | Proyección por jugador y niebla "viéndolo ahora" | `proyecciones/jugador.ts:457-519` | ✔ | derivada en cada lectura; no se guarda lo derivable |
| 45 | Mantenimiento de disco (respaldos, poda) | `server/mantenimiento.ts` | 1 | periódico por naturaleza, opt-in, fuera de la partida |

### 3.3 NPC de gobernanza (`npcGobernanza.ts:1498`)

La pregunta de la petición era *¿decide cada tick lo que podría reaccionar a eventos?* **Casi todo lo que decide es
política por umbrales de variables continuas** (stock, déficit, solvencia): un bot que mira cada minuto lo que un
jugador miraría de vez en cuando. Eso no es mal asignado; es lo que hace un agente. Los pasos con forma de
*reacción a un hecho* son pocos:

| Paso | Ubicación | Hecho al que reacciona | Cat. |
|---|---|---|---|
| Fundar el asentamiento inicial de una Facción NPC sin ninguno | `1514-1535`; `fundarAsentamientosIniciales` `1421` | crear la Facción NPC (ya lo hace `crearFaccionNpc`) **o perder la última plaza** | **2 → (C)** |
| `asegurarGobernanzaBase` (cargos + reserva) | `232`, llamada `1540` | plaza fundada o conquistada; cargo vacío; `nivelActual` llega a 2 | 3 — invariante multi-camino |
| `materializarFundadoresNpc` | `1306`, llamada `1543` | una caravana de fundación fundó una plaza NPC | ✔/no mover (§6) |
| `responderPropuestasNpc` | `513`, llamada `1601` | llega una propuesta de trueque | 3 → **(I)** |
| `guarnecerNpc` / `prepararDefensaNpc` | `689`, `713`, llamada `1644-1647` | escuadra nueva, héroe vuelve a casa, cambia la residencia | 3 → **(F)** |
| `replegarLosQueYaTerminaron` | `1198`, llamada `1702` | una columna acampa (`ejercito.llega`) | 3 — también repliega las que quedan `estacionado` por otros caminos |
| Granjas mínimas, infraestructura comercial, núcleo militar, muralla | `299-429`, `1551-1575` | umbral de recursos / nivel | 1 |
| Ascenso | `1580-1592` | stock + solvencia (cambia con la producción) | 1 |
| Trueque de supervivencia | `553`, `1595` | déficit continuo de un recurso | 1 |
| Reclutar, atacar campamentos, lanzar campañas, órdenes de mercado, persecuciones, expandir | `644`, `750`, `1093`, `1043`, `976`, `847` | umbrales / geometría | 1 |

El NPC además cumple el **principio rector** de `NPC_Gobernanza_Facciones_Controladas.md`: decide después del tick,
con las funciones públicas, como un jugador. Eso es una razón de capa para **no** disparar sus reacciones desde
dentro de los comandos (§6).

---

## 4. Categoría 2 — mal asignadas

Orden de ficha: ubicación · hecho · dónde se produce hoy · qué cambia al moverlo · riesgo · esfuerzo.

### (A) El nivel de Facción se recalcula al final de cada tick, aunque la XP se gane en un comando

- **Ubicación.** `engine/simulation.ts:298-300` (`aplicarAjustesExperiencia` + `avanzarNivelesFaccion`),
  `engine/faccion.ts:54-79`.
- **Hecho que debería dispararlo.** Ganar experiencia de Facción. El nivel es función pura de `experiencia`
  (`calcularNivelFaccion`, `faccion.ts:25`); solo puede cambiar cuando la XP sube.
- **Dónde se produce el hecho.** Seis funciones de `engine/combate.ts` que llaman a `aplicarAjustesExperiencia`
  directamente: `iniciarAsedio:383`, `atacarCampamentoBandidos:457`, `atacarCampamentoConColumna:515`,
  `asediarConEjercito:608,650`, `encuentroEntreEjercitos:696` — todas invocadas desde **comandos** (`militar.ts`,
  `interaccion.ts`, `resultadoBatalla.ts`) o desde el turno del NPC. Y el XP por edificio completado
  (`simulation.ts:164-170`), que sí nace dentro del tick.
- **El defecto concreto.** Un comando que concede XP devuelve una Facción con la XP nueva y el **nivel viejo**; el
  nivel sube en el tick siguiente (≤1 min) y el evento `faccion.nivel_subio` sale entonces, no en el comando que
  lo causó. Consumidores del nivel en ese hueco: cap de fundación (`settlement.ts:235`, `expansion.ts:97`), slots
  de política (`politicas.ts:58`) y cupo de nivel de asentamiento (`ascenso.ts:84`). Es la ventana (b) del caso de
  residencia.
- **Qué se rompería al moverlo.**
  - *Orden de evaluación:* nada dentro del tick consume `faccion.nivel` después de `avanzarEjercitos`
    (tributos, reputación y títulos no lo leen), así que subirlo antes no altera el pipeline.
  - *Eventos:* `faccion.nivel_subio` pasa a salir en el comando (y con su `momento`). Los tests
    `eventosDominioFaccion.test.ts` llaman a `avanzarNivelesFaccion` directamente y habría que repuntarlos.
  - *Batch:* **sí cambia** si el NPC gana XP por combate en su turno (corre después del tick): el nivel subiría en
    ese mismo turno y no en el tick siguiente. Los sellos del batch con combate NPC dejan de coincidir; son
    equivalentes en distribución, no byte a byte.
  - *`snapshot_baseline` y `determinismo`:* **no se mueven** — solo hay XP de construcción, que se concede dentro
    del tick y se recalcula antes de salir, igual que hoy.
- **Punto único de escritura:** `aplicarAjustesExperiencia` (`faccion.ts:54`) — los seis sitios de combate ya pasan
  por ahí. Es lo que hace barato el movimiento.
- **Riesgo:** bajo. **Esfuerzo:** bajo (S). **Impacto:** medio-bajo (ventana de ≤1 min, pero visible al jugador).

### (B) Una ruina deja a sus dependientes para que el tick los descubra

- **Ubicación.** El hecho: `simulation.ts:224-227` (`destruido` de `avanzarMantenimiento`,
  `mantenimiento.ts:260-275`). Los dependientes, repartidos por otros módulos.
- **Hecho que debería dispararlo.** `asentamiento.ruinas`: hay un único sitio donde se decide que un asentamiento
  deja de existir.
- **Qué depende de él y cómo se entera hoy.**

  | Dependiente | Cómo se entera | Resultado hoy |
  |---|---|---|
  | Caravana comercial que sale de o va hacia la plaza | `trade.ts:551-556`, barrido por caravana | se pierde **sin evento**; libera la escolta |
  | Caravana de fundación que sale de ella | `expansion.ts:187-193` | se pierde con `expansion.caravana_perdida` |
  | **Campamento de bandidos asignado a ella** | **nunca** | queda huérfano para siempre y bloquea el spawn de otros — ver bug **B1** |
  | **Trueque `activo` con ella** | solo al llegar `expiraEn` (`trade.ts:887`) | la otra parte no puede cumplir y **cobra la penalización de reputación** por incumplir — ver **B2** |
  | Ejército con ella de origen | al llegar a casa (`reintegrar`, `ejercitos.ts:1457`) | se resuelve, aunque tarde |
  | Residentes | en `9f9a098` (fuera de este árbol) | — |

- **Qué se rompería al moverlo.** Cerrar los dependientes en el sitio de la ruina cambia **cuándo** se pierde cada
  caravana (hoy: en el paso de comercio del mismo tick, que va *después*; `escoltasLiberadas` se aplica ahí,
  `simulation.ts:231`). El orden es el mismo tick pero distinto punto de la cadena: eventos reordenados (tocan
  `determinismo`) y las caravanas desaparecen antes de que `avanzarComercio` las mueva. Los trueques pasarían a
  `expirado` sin penalizar a nadie, lo cual **es el cambio de comportamiento buscado**.
- **Riesgo:** medio — toca cuatro módulos y el orden del tick, y el batch (donde las ruinas son comunes, son la
  causa de muerte que miden los diarios) deja de ser byte-idéntico. **Esfuerzo:** medio (M).
  **Impacto:** medio (arregla B1 y B2 de raíz y deja de narrar tarde).

### (C) El NPC re-funda su asentamiento inicial mirando cada tick si "tiene alguno"

- **Ubicación.** `npcGobernanza.ts:1514-1535` y `fundarAsentamientosIniciales` (`1421-1465`).
- **Hecho que debería dispararlo.** Dos casos distintos que hoy comparten el mismo `if`: (1) crear una Facción NPC
  — **ya resuelto** en `crearFaccionNpc.ts` al crearla — y (2) **perder la última plaza** por conquista
  (`militar.ts:127`, `resultadoBatalla.ts:225`, `ejercitos.ts:1239`) o ruina (`simulation.ts:226`).
- **Mezcla de casos.** El caso (2) es una *resurrección inmediata* de una Facción derrotada, que el comentario del
  propio código admite ("los que ya tenga (se quedó sin asentamientos)") pero no está declarada como regla de
  juego en ningún `Docs/Game`. Y si no encuentra sitio (`buscarPosicion` devuelve `undefined`) **reintenta cada
  tick**, con el barrido en rejilla de `buscarPosicionFundacionInicialPorDefecto`.
- **Qué se rompería.** Poco: Paso 0 solo corre con `config.faccionesIds` (partida real); **en batch no se ejecuta**
  (`npcGobernanza.ts:1514`), así que ni `snapshot_baseline`, ni `determinismo`, ni los sellos del batch lo ven.
  Hay que decidir de diseño si la resurrección es inmediata, con retardo, o no existe.
- **Riesgo:** bajo. **Esfuerzo:** bajo-medio (S/M, el esfuerzo es la decisión de diseño). **Impacto:** bajo.

---

## 5. Categoría 3 — dudosas

### (D) Spawn de bandidos: reintento perpetuo y guarda que no mide lo que dice

- **Ubicación.** `engine/bandidos.ts:69-93`, llamado en `simulation.ts:247`.
- **Qué hace.** Cada tick: `campamentos.length >= asentamientos.length || instante < proximoSpawnEn` → si no, busca
  el asentamiento sin campamento y `bosqueNoReclamadoMasCercano`, que recorre **todos los bosques** del mundo
  contra **todas las zonas** (`bandidos.ts:51-56`). Si no queda bosque libre, no spawnea y **vuelve a hacerlo el
  tick siguiente, y el siguiente**, mientras el asentamiento siga sin campamento.
- **Hecho que lo desbloquea.** Un bosque queda libre (se arruina una plaza, se achica una zona), se funda una plaza,
  se destruye un campamento (`interaccion.ts:214`, `resultadoBatalla.ts:205` ya ponen `bandidosProximoSpawnEn`).
- **Por qué es dudoso y no 2.** El cooldown de reposición es legítimo (cat. 1); lo dudoso es el **reintento**, cuyo
  coste crece con zonas × bosques justo en madurez, cuando la mayoría de bosques ya están reclamados. Doc 6 perfiló
  el tick sobre trazado y colocación y no mide esta rama. La guarda por conteo además es frágil (**B1**).
- **Qué se rompería.** Sin RNG (`avanzarSpawnBandidos` no recibe `rng`), así que es seguro para la secuencia
  aleatoria. Cuidado con *atascar un spawn*: si el trigger omite un caso, un asentamiento se queda sin campamento
  para siempre.
- **Acción.** **Medir primero** (ver §8). La salida barata no es un evento: es recordar "sin bosque libre para el
  mundo tal" con una clave (la lista de zonas) y no repetir la búsqueda mientras no cambie.
- **Riesgo:** bajo. **Esfuerzo:** bajo (S). **Impacto:** desconocido hasta medir.

### (E) Títulos: cadencia arbitraria de un minuto sobre magnitudes continuas

- **Ubicación.** `simulation.ts:304-306`, `titulos.ts:26-94`.
- **Qué hace.** Recalcula los cuatro títulos cada tick sobre oro total, tropa total, nº de asentamientos y ligas, y
  narra cada cambio de manos (`titulo.cambia_manos`). El propio comentario dice "recalculados periódicamente […]
  no en tiempo real".
- **Por qué es dudoso.** Dos de los títulos (oro, ejército) dependen de magnitudes que **cambian cada minuto**: no
  hay hecho que dispare nada, así que *periódico* es la forma correcta. Lo dudoso es la cadencia: con dos
  Facciones de oro o ejército parecidos el título puede **oscilar entre ellas** y el log de Aedas narrar cada
  cambio. La cadencia no es una decisión, es un accidente de "1 tick = 1 min".
- **Qué se rompería.** Dar a los títulos una cadencia propia (cada N minutos de mundo) o una histéresis cambia
  *qué evento sale en qué tick* (`determinismo` lo detecta; `snapshot_baseline` no, no lee títulos) y los tests de
  `eventosDominioTitulos.test.ts`.
- **Acción.** Declarar la cadencia como constante nombrada. **No** convertirlo en evento.
- **Riesgo:** bajo. **Esfuerzo:** bajo (S). **Impacto:** bajo (ruido de log).

### (F) NPC: guarnición y loadout reescritos cada tick para todos los héroes bot

- **Ubicación.** `npcGobernanza.ts:689-735` (`guarnecerNpc`, `prepararDefensaNpc`), bucle en `1644-1647`.
- **Qué hace.** Para cada plaza NPC y cada héroe bot residente, reasigna guarnición y **reescribe el loadout
  activo**. `guardarLoadout` (`heroe.ts:83`) devuelve siempre un héroe y unos `loadouts` nuevos, aunque no cambie
  nada; `asignarGuarnicion` rechaza con `HeroeInvalidoError` y se captura **como flujo de control** cada vez que no
  cabe.
- **Hechos que lo disparan.** Escuadra nueva (`reclutarTropa`), héroe vuelve a casa (`reintegrar`, dentro del
  motor), cambio de residencia, sanar. **Cuatro caminos, uno de ellos dentro del tick del motor**: no hay un único
  sitio donde colgarlo (pregunta 1).
- **Acción.** No mover. Hacerlo **idempotente**: si el resultado coincide con lo que ya hay, devolver el mismo
  objeto. Es churn de estado y excepciones, no un fallo de orden.
- **Riesgo:** bajo. **Esfuerzo:** bajo (S). **Impacto:** bajo.

### (G) Derivados que se recomputan enteros cada tick

No es el principio de eventos —son **lecturas**, no consecuencias— pero es el mismo síntoma que Doc 6 E3 ya midió
("el motor recalculaba en cada tick cosas que solo cambian al construir").

- `computeTodasLasZonas`, `reclamosDeFuentes`, `encontrarCapital` por Facción: `simulation.ts:127,130,137` (y otra
  vez en `npcGobernanza.ts:1551-1553`). Solo cambian al fundar, arruinar, conquistar o crecer `radioPotencial`.
- `factorLineaProduccion` / `fuentesDeRecurso`: `construction.ts:1122,1159`, O(edificios²) por receta y tick;
  solo cambia al completarse un edificio.
- `campamentoDe(asentamiento, heroes)` por plaza: O(asentamientos × héroes).
- **Qué se rompería.** Nada de comportamiento si la clave es completa; la lección de Doc 6 es que *"un campo de más
  en la clave solo cuesta aciertos; uno de menos da una respuesta equivocada"*, y que la primera memoización de la
  red de calles falló por omitir `avance`. Aquí la clave de zonas es (id, posición, `radioPotencial`, `faccionId`).
- **Acción.** **Medir con un perfil por etapa antes de tocar** (hoy no existe: `medicion-escala.ts` mide el tick
  entero). Si no pesa, no se hace.
- **Riesgo:** medio (claves). **Esfuerzo:** bajo-medio. **Impacto:** desconocido.

### (H) Memoria de niebla: grabar lo visto cada tick

- **Ubicación.** `simulation.ts:322-330`, `memoria.ts:96-142`, `ubicacion.ts:269-292`.
- **Qué hace.** Para cada Facción, para cada "ojo" (plaza o columna), `marcarVisto` decodifica la máscara hex y
  la vuelve a marcar; para cada plaza ajena a la vista **reescribe su ficha con `conocidoEn = instante`**, de modo
  que `if (exploracion === previa && asentamientos === previa)` (la guarda "un tick tranquilo no ensucia el
  estado") **no se cumple nunca mientras haya una plaza ajena a la vista**.
- **Hechos que lo disparan.** Moverse una columna (continuo en marcha), crecer el radio de una plaza, fundar o
  conquistar. Para una columna parada o una plaza quieta el resultado es idempotente.
- **Por qué NO moverlo a eventos.** (1) `conocidoEn` es "la última vez que la vi": mientras está a la vista, esa
  fecha **es** ahora, y la única forma de tener la última fecha exacta es escribirla mientras se mira —o saber
  cuándo dejó de verse, que es otro barrido—. (2) Habría que enganchar **cada** mutación de posición (movimiento
  del tick, `salirAlMundo`, `marcharA`, desalojo, fundación…). (3) La ventana ya la cubre a propósito la proyección:
  `nieblaDe` calcula `visibles` en vivo y los une con la memoria (`jugador.ts:495-529`, comentario de "la ventana de
  un tick"). Está bien resuelto.
- **Acción.** Como mucho, una micro-optimización medible: no re-decodificar un ojo cuya posición y alcance no han
  cambiado desde el tick anterior. **Medir antes.**
- **Riesgo:** bajo. **Esfuerzo:** bajo. **Impacto:** bajo.

### (I) NPC: responder propuestas de trueque en su turno

- **Ubicación.** `npcGobernanza.ts:513-551`, llamada en `1601`.
- **Hecho.** `proponerTrueque` (comando de un jugador con destino a una plaza NPC).
- **Por qué es 3 y no 2.** El NPC responde a las propuestas pendientes en su turno, o sea con **≤1 tick de
  latencia**. Es una decisión de comportamiento ("tarda un poco en contestar") que además mantiene el principio
  rector: el NPC juega después del tick, como un jugador. Responder dentro del comando del jugador metería la
  decisión del NPC (y su RNG, si lo hubiera) en un comando ajeno.
- **Acción.** Dejarlo. Si algún día se quiere que conteste "al instante", será una decisión de diseño con
  `Docs/Game` delante, no una corrección.

### (J) Columna esperando a la puerta

- **Ubicación.** `ejercitos.ts:1582-1590`.
- **Qué hace.** Una columna NPC que llega a una plaza enemiga que está en batalla (Doc 5.15.1) o cuyos héroes están
  todos heridos se queda `marchando` con la ruta acabada y **"la llegada se vuelve a mirar cada tick"**.
- **Hechos que la liberan.** Cierra la batalla (`resultadoBatalla.ts`, `vencerBatallas`) o sana un héroe
  (`heridoHasta`, que **no** es un evento: caduca perezosamente por `estaHerido`).
- **Acción.** No mover. Sanar no tiene punto de disparo —solo un scheduler lo tendría—, el bucle de ejércitos ya
  pasa por cada columna y el coste añadido es una comparación. Se anota por si llega el scheduler.

---

## 6. Lo que NO conviene mover, y por qué

Para no sobre-aplicar el principio. Cada punto con la pregunta de §2 que lo descarta.

| Qué | Por qué se queda |
|---|---|
| **Caducidades por fecha** (políticas, trueques, órdenes de mercado, obras, mejoras, ascenso, `preparaHasta`, ocupación, batallas, regeneración, respawn de bandidos) | Ya son "eventos baratos": la fecha vive **en la entidad** (Doc 10 §6, "Instantes"; D3 sustituyó los contadores por `completaEn`). Cada una son comparaciones sobre listas pequeñas. Un scheduler añadiría una **segunda fuente de verdad** (la fecha en la entidad y en la cola) y su orden de disparo, y sacaría los efectos del punto exacto del pipeline donde hoy se encadenan (`avanzarAscenso` va **antes** de la nutrición y el crecimiento, así que el techo de población del nivel nuevo aplica ese mismo tick; terminar un edificio sube `radioPotencial` y XP en el mismo paso). Solo revisar cuando llegue el scheduler de comandos programados (D5, aplazado), y entonces migrar de uno en uno. (Preguntas 3 y 4) |
| **Producción, consumo, crecimiento, mantenimiento, ración, tributo, reputación** | Tasas continuas por minuto. No hay hecho. |
| **Auto-construcción, mejoras automáticas, recetas, obra de muralla** | Decisiones por **umbral de stock**: el "hecho" es cruzar una cantidad que cambia cada minuto. Ya están memoizadas y con guardián de impagables (Doc 6 E3). |
| **El NPC por umbrales** (granjas, comercio, núcleo militar, ascenso, trueque de supervivencia, reclutar, atacar, campañas, órdenes, persecuciones, expandir) | Es el agente mirando variables continuas, igual que lo haría un jugador. Moverlas a los comandos rompería el principio rector del NPC y su orden de evaluación, y el batch (donde sí corre) dejaría de ser comparable con los diarios. |
| **`asegurarGobernanzaBase` y la disolución de columnas vacías** | Sostienen un **invariante con varios caminos de ruptura** (cargo vacío, plaza conquistada, héroe que se va, `nivelActual`; participantes que salen). Un evento por camino deja un caso atrás; el barrido barato es la garantía. Es la excepción a "resolver en el momento del hecho": aplica cuando los hechos son enumerables (como la residencia), no cuando son abiertos. (Pregunta 1) |
| **`materializarFundadoresNpc`** | Crear el héroe bot **dentro** de la fundación obligaría a que el motor construyese `Heroe` con `controlador: 'bot'` (`heroeBot` vive en `session/npcGobernanza.ts`): cruza la separación `engine`/`session`. Y la ventana es nula: el turno del NPC va en el **mismo** `aplicarYPersistir` que el tick (`runnerDePartida.ts:346-352`). |
| **Fichas de la memoria de niebla** | Ver (H): el barrido es la forma de mantener "última vez visto". |
| **Proyección, precios de referencia, geometría por frame** | Ya derivan **al leer** (`runnerDePartida.ts:240-267`, `jugador.ts`). Es el patrón correcto; añadir un evento sería guardar lo derivable. |
| **`heridoHasta` y `ocupacionHasta` (lógica)** | Ya perezosos: `estaHerido`, `estaOcupado` comparan con `instante` al preguntar. El único barrido que queda en ocupación es limpiar el campo y narrar (fila 19); no hay lógica que mover. |
| **Reloj de mundo, mantenimiento de disco, catch-up** | Infraestructura periódica por naturaleza, fuera del estado de partida. |
| **Evento `ejercito.llega` y asedio al llegar** | Ya son one-shot al cruzar `progreso >= 1` (`98f4e4c`). |

**Dónde el principio ya está bien aplicado** (útil como ejemplos de cómo se ve): cupo y gates del ascenso al
**pedirlo** (`ascenso.ts:119-190`), asedio como orden (`98f4e4c`), la conquista en `aplicarConquista` +
`desalojarResidentes` en el momento, `radioPotencial` por edificio completado, combates de Unity abiertos desde el
hecho (`avanzarTick.ts:70`).

---

## 7. Bugs y observaciones aparte

### Bugs claros

**B1. Campamentos de bandidos huérfanos.** Una ruina no retira el campamento asignado (`simulation.ts:226` filtra
asentamientos; nadie toca `campamentosBandidos`), y `avanzarSpawnBandidos` decide con
`campamentos.length >= asentamientos.length` (`bandidos.ts:79`). Con 3 plazas y 3 campamentos, si una cae hay 2
plazas y 3 campamentos: `3 >= 2` y **la plaza nueva que se funde después nunca recibe campamento** mientras el
huérfano siga vivo — y el huérfano sigue atacando caravanas (`avanzarAtaquesBandidos` recorre todos). Solo se
retira si alguien lo destruye; el NPC lo excluye expresamente ("si el asentamiento asignado sigue vivo",
`npcGobernanza.ts:732`). Hallazgo **por lectura, no reproducido**: no existe test que ejercite ruina + spawn
(`bandidos.test.ts` no menciona ruinas). La guarda correcta es "¿hay algún asentamiento sin campamento?", que ya
calcula `asentamientoSinCampamento` (`bandidos.ts:42`).

**B2. Penalización de reputación por una ruina ajena.** Un trueque `activo` con una plaza que cae en ruinas no se
cierra: la otra parte ya no puede lanzar caravanas (`trade.ts:936` descarta los lados cuyo destino ya no existe) y al vencer
`expiraEn` se le aplica `penalizacionTruequeIncumplido` (`trade.ts:898-901`) porque `cantidadEntregada <
cantidadTotal`. La Facción superviviente pierde reputación por incumplir un pacto que no pudo cumplir. Mismo
origen que **(B)**; hallazgo por lectura, no reproducido.

### Observaciones (a confirmar si son deliberadas)

**O1. Los ticks del reloj de mundo no se difunden por WebSocket.** `HubDeDifusion.difundir` solo se llama desde
`rutas/comandos.ts:151`, `rutas/batallas.ts:156` y `rutas/admin.ts:418` (tick manual de administración). El camino
`iniciarRelojDeMundo → sincronizarConReloj → unTickCompleto` (`runnerDePartida.ts:383-432`) persiste y anexa
eventos al JSONL, pero no avisa a nadie: los suscriptores solo ven eventos de comandos. Además `ResultadoComando`
de un tick solo trae los eventos de la primera mutación (`runnerDePartida.ts:517-521` lo documenta): ni siquiera el
tick manual difunde lo que haga el NPC. Fuera del alcance de esta auditoría, pero es exactamente el problema
inverso: un hecho (el tick) sin su consecuencia (avisar).

**O2. Lo que sobrevive a una conquista.** `aplicarConquista` (`combate.ts:156-209`) reinicia cargos, fundadores,
población, edificios, recintos, medidor y `ocupacionHasta`, pero **conserva** `politicasActivas`,
`reservaManual` y `autoConstruccionPausada` del Gobernador/Tesorero derrotado. Las políticas siguen ocupando
slots del cargo que ya no existe hasta que caduquen, y la reserva manual del antiguo Tesorero sigue bloqueando
gasto del nuevo dueño. Puede ser deliberado ("la ciudad cambia de dueño entera"); el documento
`Ocupacion_Post_Conquista_Definicion.md` no lo trata.

**O3. Tecnología y Eras.** No hay ninguna mecánica de tecnología por tick en el árbol (BA-006 sigue en diseño).
Cuando llegue, conviene diseñarla **por hecho** desde el principio (completar un edificio o cumplir un hito que
la desbloquea) y no como condición re-derivada cada minuto, que es el patrón de (A).

---

## 8. Tabla priorizada y orden recomendado

Impacto = lo que gana el usuario o el motor. Riesgo = probabilidad de cambiar el comportamiento observable o los
guardianes de §2. Esfuerzo: S = horas, M = un día, L = varios.

| # | Qué | Cat. | Impacto | Riesgo | Esfuerzo | Mueve el batch | Toca `snapshot_baseline` / `determinismo` |
|---|---|---|---|---|---|---|---|
| **B1** | Guarda de spawn de bandidos por "asentamiento sin campamento" y limpieza de huérfanos | bug | medio | bajo | S | solo si hay ruinas con campamento | no / no (ninguno cubre ruinas) |
| **B2** | Cerrar trueques con una plaza arruinada sin penalizar | bug | medio | bajo | S | sí (ruinas) | no / no |
| **A** | Nivel de Facción al ganar la XP | 2 | medio-bajo | bajo | S | **sí**, solo con combate NPC | no / no |
| **—** | Perfil por etapa del tick en madurez (no cambia nada) | medir | decide G, D, H | nulo | S | no | no |
| **D** | Spawn de bandidos: no repetir la búsqueda sin bosque libre | 3 | ¿medio? | bajo | S | no (sin RNG) | no / no |
| **B** | Cerrar los dependientes de una ruina en el sitio de la ruina | 2 | medio | medio | M | **sí** (ruinas comunes) | no / sí si cambia el orden de eventos |
| **C** | NPC: re-fundación al perder la última plaza y no cada tick | 2 | bajo | bajo | S-M (decisión) | no (no corre en batch) | no / no |
| **E** | Cadencia/histéresis declarada de los títulos | 3 | bajo | bajo | S | sí (eventos) | no / **sí** |
| **F** | NPC: `guarnecer`/`prepararDefensa` idempotentes | 3 | bajo | bajo | S | no (mismo resultado) | no / no |
| **G** | Memoizar zonas, reclamos y `factorLineaProduccion` | 3 | ¿?, solo si el perfil lo pide | medio (claves) | S-M | no (mismo resultado) | no / no |
| **H** | No re-decodificar ojos sin cambios | 3 | bajo | bajo | S | no | no / no |
| **I, J** | Latencia del NPC, columna a la puerta | 3 | — | — | — | — | no tocar |

**Orden recomendado.**

1. **B1 y B2 primero**, aunque no sean "mover nada": son bugs, tienen un arreglo local y no esperan a una decisión.
2. **(A) nivel de Facción.** El caso más limpio del principio: un hecho, un punto único de escritura
   (`aplicarAjustesExperiencia`), una ventana visible. Sirve de **ensayo** de "mover una consecuencia al hecho" con
   riesgo bajo, antes de tocar la ruina.
3. **Medir** un perfil por etapa del tick con una partida madura (muchas zonas, pocos bosques libres, columnas
   paradas). Decide si (D), (G) y (H) son trabajo real o solo parecen. Doc 6 E3 ya enseñó que el coste de este motor
   estaba donde nadie lo esperaba.
4. **(B) la ruina como hecho**, que de paso fija B1 y B2 de raíz. Es el más grande de la lista y el único que reordena
   el pipeline: conviene hacerlo con los sellos del batch delante y aceptando que cambian.
5. **(C), (E), (F)** cuando haya hueco; ninguno bloquea nada.
6. **Dejar (I) y (J)** como están y la lista de §6 como está. Si llega el scheduler de comandos programados (D5),
   revisar entonces las caducidades una por una; no antes.

Un criterio que conviene dejar escrito junto al caso de la residencia: **mover al hecho cuando los hechos son pocos
y enumerables y cada uno ya tiene un sitio de escritura** (residencia: conquista, ruina, dejar casa; nivel de Facción:
`aplicarAjustesExperiencia`; ruina: un único `destruido`). **Mantener el barrido cuando sostiene un invariante que
puede romperse por caminos abiertos**, y **memoizar con clave cuando lo que se recalcula es una lectura**. Tres
herramientas distintas para tres problemas que se parecen desde fuera.
