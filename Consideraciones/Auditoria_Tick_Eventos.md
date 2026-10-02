# Auditoría del tick: qué se evalúa cada minuto y qué debería dispararse por hecho

**Abierto el 2026-10-02**, sobre `8091638` (`origin/main`). **Revisado el mismo día (segunda pasada)** con medidas
de CPU de una partida real y reproducciones de cada bug; la §10 resume qué cambió respecto a la primera versión,
que se escribió solo leyendo código.

Sale de la corrección de la residencia en campamentos: `acogerHeroesSinCasa` recorría todos los héroes en cada tick
para reubicar a quien se había quedado sin casa, y era una **consecuencia de un hecho puntual** (cae una plaza, se
arruina, alguien deja su casa) que se resolvía tarde, barriendo. Aquí se hace la misma pregunta sobre el resto del
tick.

Es un **informe para decidir, no un plan**. No se ha tocado código del repositorio: las medidas y reproducciones
son scripts aparte que importan `src/` tal cual.

> **Aviso de alcance.** El commit `9f9a098` (campamentos de mercenarios, `reubicarResidentesDeRuina`,
> `dejarResidencia`) **no está en el árbol auditado**: ni en `origin/main` ni en ninguna rama remota. En este árbol
> `desalojarResidentes` solo existe para la conquista (`engine/combate.ts:226`) y la ruina
> (`engine/simulation.ts:224-227`) no reubica a nadie. La residencia se da por resuelta tal como la describe la
> petición y no se audita.

---

## 1. Conclusiones

1. **El motor está bien orientado; el turno del NPC no.** Con 20 Facciones NPC, el tick completo del servidor
   (`RunnerDePartida.unTickCompleto`) gasta **~7,5 ms en el motor y 22-38 ms en el NPC** (§3). A 50 Facciones, 27 ms
   contra 54. El NPC corre cada tick, dentro de la misma entrada de la cola serial, y es lo que más pesa.
2. **El mayor coste del tick entero es una consecuencia mal asignada.** Cada tick el NPC hace **~1 050 intentos de
   reclutar, de los que fallan el 99,5 %**. El 90 % falla porque la plaza no tiene Barracón o Galería de tiro, algo
   que solo cambia cuando termina una obra. Son **~20 ms/tick, 2,5 veces el motor entero**, y más de la mitad se va en
   construir excepciones (§5, **N1**).
3. **Lo segundo que más pesa es barrer historia que solo crece.** Las órdenes de mercado expiradas nunca salen del
   estado (de 252 a 5 550 en 3 000 ticks, el 93 % ya expiradas), y `publicarOrdenesNpc` copia y recorre la lista
   entera por cada plaza y recurso: **de 0,4 a 12,5 ms/tick en esos 3 000 ticks**. Al log de eventos en memoria le
   pasa lo mismo a menor escala (§5, **N2**). El hecho que debería moverlas fuera es su paso a estado terminal.
4. **Lo que la primera versión mandaba "medir" no pesa.** Zonas ~1 ms, memoria de niebla ~0,6 ms, títulos ~0,3 ms,
   spawn de bandidos ~0,1 ms por tick. Siguen siendo dudosos por **corrección** en algún caso, pero no por coste.
5. **Las caducidades por fecha, el NPC por umbrales y la memoria de niebla deben quedarse como están** (§7).
6. **Cuatro bugs reproducidos**, uno grave: todos los campamentos de bandidos nacen con el **mismo id**
   `campamento-0`. Solo se puede atacar el primero, y destruirlo los borra todos (§8, **B0**).

---

## 2. Cómo se ha clasificado

Tres categorías, las de la petición:

- **1 · Legítima del tick.** Dinámica continua o periódica por naturaleza: producción, consumo, crecimiento,
  mantenimiento, movimiento, caducidad por fecha, reposición programada.
- **2 · Mal asignada.** Consecuencia de un hecho discreto (comando, conquista, obra terminada, llegada, paso a
  estado terminal) que se detecta recalculando o barriendo en vez de resolverse cuando ocurre.
- **3 · Dudosa.** Depende de cuántas veces se mire, de una ventana poco clara, o mezcla un caso real con otros.

Cinco preguntas para decidir si algo de la 2 o la 3 **debe** moverse. Si alguna sale mal, no se mueve:

1. **¿Hay un hecho identificable y enumerable?** Si la consecuencia puede venir de cuatro caminos y mañana de un
   quinto, el barrido es el invariante; un evento por camino deja uno atrás.
2. **¿La consecuencia escribe estado/evento o solo lee algo derivado?** Para lecturas, la herramienta es
   **comprobarlo barato antes** o **memoizar con clave** (como Doc 6 E3), no un evento.
3. **¿Qué ventana abre el barrido?** Hasta un tick (1 min de mundo) suele ser aceptable; no lo es que un *comando*
   devuelva un estado que contradice su propio efecto.
4. **¿Qué orden de evaluación cambia?** El pipeline de `avanzarSimulacion` es *load-bearing* (comentado en
   `simulation.ts:172-199`).
5. **¿El hecho tiene un único sitio de escritura?** Si sí, mover es barato; si está repartido, el coste lo pone el
   sitio, no el principio.

Lo que protegería cada cambio (la referencia para "qué se rompería" en las fichas):

| Guardián | Qué fija | A qué es sensible |
|---|---|---|
| `engine/__tests__/snapshot_baseline.test.ts` | 1 Facción, 1 plaza, semilla 99, cortes 1/10/25/50/100: nivel, población, medidor, nº de escuadras, edificios, 7 recursos, nivel y reputación de la Facción | orden del pipeline de construcción/población/mantenimiento. No ve NPC, combate, conquista, ruina ni comercio |
| `engine/__tests__/determinismo.test.ts` | 80 ticks; `toEqual` del estado final **y de los eventos de cada tick** | qué evento sale en qué tick, y todo consumo de RNG |
| `engine/__tests__/invariantes_simulacion_larga.test.ts` | 3 plazas, 300 ticks, invariantes tick a tick | recursos negativos, NaN, medidor fuera de rango |
| Batch (`scripts/run-batch-sim.ts`, sello SHA-256 de `bench-batch-checkpoint.ts`) | estado completo byte a byte | **todo**, incluidos el NPC y los **ids**: el contador de ids del NPC avanza por intento, no por éxito |
| RNG | una tirada por plaza y tick en `crecerPoblacion`; `avanzarAtaquesBandidos`; asedios y encuentros | mover algo que consume RNG cambia la secuencia de todo lo posterior |

Estado de partida: **la suite completa pasa (1 350 tests) sobre `8091638`**.

---

## 3. Medidas

**Método.** `GameSession.crear` con semilla 7; 20 Facciones creadas con el comando real `crearFaccionNpc`
(`postura` por defecto, agresiva); y por cada tick, la misma secuencia que el runner: `avanzarTick` →
`avanzarAutoComercio` → `avanzarFaccionesNpc`. Se corrieron 4 500 ticks (~3 días de mundo), con perfil de CPU
(`node --cpu-prof`) en dos ventanas de 300 ticks: desde el tick 1 500 y desde el 4 500. Una tercera corrida, de 50
Facciones y 400 ticks, mide cómo escala.

**Límites, para no sobreleer los números.** Es la máquina de desarrollo del contenedor. Es un mundo solo de NPC,
temprano: las 20 plazas siguen en nivel 1 y no ha salido ningún ejército. Sirve para ver **qué pesa y qué crece**;
no da cifras de una partida madura con jugadores.

**Reparto por tick (ms/tick, tiempo inclusivo):**

| | desde el tick 1 500 | desde el tick 4 500 | Nota |
|---|---|---|---|
| **Turno NPC (`avanzarFaccionesNpc`)** | **21,6** | **37,8** | crece con el tiempo con el mundo constante |
| ↳ `reclutarParaTodos` | 19,0 | 20,0 | plano, y aun así el mayor |
| ↳ `publicarOrdenesNpc` | 0,4 | **12,5** | ×34: recorre todas las órdenes de la historia |
| ↳ `prepararDefensaNpc` + `guarnecerNpc` | 0,4 | 0,4 | |
| ↳ `asegurar*` (granjas, comercio, militar, muralla) | 0,6 | 0,4 | |
| **Motor (`avanzarTick`)** | **8,0** | **7,6** | estable |
| ↳ `avanzarConstruccion` | 4,7 | 3,7 | el grueso del motor; ya memoizado (Doc 6 E3) |
| ↳ `computeTodasLasZonas` | 1,1 | 0,7 | |
| ↳ `grabarLoVisto` | 0,7 | 0,6 | |
| ↳ `calcularTitulos` | 0,4 | 0,2 | |
| ↳ `avanzarSpawnBandidos` | 0,2 | 0,1 | |
| ↳ `caducarOrdenes` | 0,01 | 0,12 | crece, por la misma razón que `publicarOrdenesNpc` |
| **`exito()` (copia del log de eventos)** | 0,05 | **1,9** | crece con la edad de la partida |

**Escala:** con 50 Facciones, a los 400 ticks, motor 27,5 ms y NPC 53,7 ms.

**Perfil propio (sin hijos) de toda la corrida 1 500 → 4 500:** el 29 % del tiempo total está **dentro del
constructor de `ReclutamientoInvalidoError`**, otro 13 % en el propio `reclutarTropa` y el 10,7 % en
`publicarOrdenesNpc`.

**Qué falla al reclutar** (todos los intentos de un tick sobre el estado del tick 4 500): 1 050 intentos y 5 éxitos.

| Motivo | Intentos |
|---|---|
| `Se necesita barracon activo en nivel interno N` | 570 |
| `Se necesita galeriaDeTiro activo en nivel interno N` | 380 |
| `Este escuadrón ya está al tope de unidades` | 95 |

**Colecciones que solo crecen** (estado guardado en el tick 1 500 y en el 4 500):

| | tick 1 500 | tick 4 500 |
|---|---|---|
| Órdenes de mercado (expiradas / activas) | 252 (156 / 96) | **5 550 (5 150 / 400)** |
| Eventos en memoria (`eventosDominio`) | ~5 000 | **28 184** (~9 por tick) |

Ni órdenes, ni trueques, ni batallas terminadas se retiran del estado en ningún sitio (no hay `filter` de poda en
`engine/` ni en `session/`). En este mundo no llegó a cerrarse ningún trueque, así que su crecimiento no se midió,
pero el patrón de código es el mismo (§5, **N2**).

**Extrapolación, solo orientativa:** con el ritmo medido (~1,8 órdenes y ~9 eventos por tick con 20 plazas), a los
30 días de mundo (43 200 ticks) habría ~78 000 órdenes y ~390 000 eventos. Como `publicarOrdenesNpc` es
plazas × recursos × órdenes, y las órdenes crecen con las plazas, esa línea escala con el **cuadrado** de las plazas.

---

## 4. Inventario por responsabilidad

Orden de `avanzarSimulacion` (`engine/simulation.ts:122`), más lo que corre alrededor en cada tick. **1** legítima ·
**2** mal asignada · **3** dudosa · ✔ ya está disparada por el hecho. La columna ms es la medida de §3, si la hay.

### 4.1 Motor (`engine/`)

| # | Responsabilidad | Ubicación | Cat. | ms | Nota |
|---|---|---|---|---|---|
| 1 | Zonas, capitales y reclamos de fuentes del mundo | `simulation.ts:127,130,137` | 3 | ~1 | derivado recalculado entero; no merece tocarse por coste |
| 2 | Obra de edificio termina | `construction.ts:1255-1284` | 1 | | fecha `completaEn`; capacidad, puestos, XP y radio cuelgan del hecho ✔ |
| 3 | Producción de extractores, granjas y leñeras | `construction.ts:1288-1326` | 1 | | tasa |
| 4 | Arranque de obras en cola cuando hay cuadrilla | `construction.ts:1339-1376` | 1 | | cupo + stock |
| 5 | Recetas | `construction.ts:1184,1381` | 1 | | tasa |
| 6 | Mejoras (terminar y empezar) | `construction.ts:1067-1108` | 1 | | fecha + stock |
| 7 | `radioPotencial` crece por edificio completado | `construction.ts:1401-1404` | ✔ | | ya por el hecho |
| 8 | Auto-construcción (`evaluarNecesidades`) | `construction.ts:533,1419` | 1 | parte de 3,7-4,7 | umbrales continuos; memoizada y con guardián de impagables |
| 9 | Obra de recintos celda a celda | `muralla.ts:772-815` | 1 | | stock + `siguienteCeldaEn` |
| 10 | XP por edificio completado | `simulation.ts:164-170` | ✔ | | |
| 11 | Caducidad de políticas | `politicas.ts:74-91` | 1 | | `expiraEn` |
| 12 | Fin de la obra de ascenso | `ascenso.ts:191-209` | 1 | | `completaEn`; gates y cupo **al pedirla** ✔ |
| 13 | Nutrición y hambruna | `population.ts:178-228` | 1 | | tasa |
| 14 | Ración y deserción de la guarnición | `tropas.ts:269-282` | 1 | | tasa |
| 15 | Crecimiento de población | `population.ts:48-155` | 1 | | tasa + RNG |
| 16 | Recaudación de oro | `simulation.ts:192-196` | 1 | | tasa |
| 17 | Mantenimiento | `mantenimiento.ts:184-288` | 1 | | la racha es un acumulador real (Doc 10 §6) |
| 18 | Ruina del asentamiento | `simulation.ts:224-227` | ✔/2 | | hecho detectado al ocurrir, **dependientes descubiertos barriendo**: **(B)** |
| 19 | Fin de la ocupación | `simulation.ts:203-205` | 1 | | la lógica ya es perezosa (`estaOcupado`); el barrido solo limpia y narra |
| 20 | Movimiento de caravanas, `preparaHasta` | `trade.ts:506-640` | 1 | | |
| 21 | Caducidad de trueques | `trade.ts:876-905` | 1 | | `expiraEn`, pero recorre también los terminales: **N2** |
| 22 | Asignación automática de caravanas a trueques | `trade.ts:860-991` | 3 | ~0 | mezcla acuerdo aceptado, caravana libre y stock (continuo); no mover |
| 23 | Caravanas de fundación: avanzar y fundar al llegar | `expansion.ts:168-230` | ✔ | | |
| 24 | Regeneración de yacimientos | `simulation.ts:241` | 1 | | fecha `regeneraEn` |
| 25 | Spawn de bandidos | `bandidos.ts:70-95` | 3 | 0,1-0,2 | guarda frágil (**B1**) e id fijo (**B0**) |
| 26 | Ataques de bandidos | `bandidos.ts:115-189` | 1 | | proximidad + RNG |
| 27 | Ejércitos: comer, mover, repostar, llegar | `ejercitos.ts:1413-1680` | 1 | | la llegada que asedia es one-shot ✔ |
| 28 | Disolver columna sin nadie dentro | `ejercitos.ts:1499-1530` | 3 | | invariante multi-camino; no mover |
| 29 | Columna esperando a la puerta | `ejercitos.ts:1582-1590` | 3 | | **(J)** |
| 30 | Encuentros por persecución | `ejercitos.ts:1617+` | 1 | | |
| 31 | Caducidad de órdenes de mercado | `market.ts:89-104` | 1 | 0,01→0,12 | `expiraEn`, pero mapea **todas** las órdenes de la historia: **N2** |
| 32 | Tributo de vasallaje | `diplomacia.ts:173-209` | 1 | | tasa |
| 33 | **Nivel de Facción a partir de la XP** | `simulation.ts:298-300` | **2** | ~0 | **(A)** |
| 34 | Reputación | `reputacion.ts:26-43` | 1 | | tasa |
| 35 | Títulos | `simulation.ts:304-306`, `titulos.ts:26-94` | 3 | 0,2-0,4 | **(E)** |
| 36 | Memoria de niebla y exploración personal | `simulation.ts:322-330`, `memoria.ts:103-142` | 3 | ~0,6 | **(H)**, no mover |

### 4.2 Sesión (`session/`) y servidor (`server/`)

| # | Responsabilidad | Ubicación | Cat. | ms | Nota |
|---|---|---|---|---|---|
| 37 | Cerrar batallas vencidas | `comandos/avanzarTick.ts:37`, `batallas.ts:509` | 1 | | `expiraEn`; también recorre las terminadas: **N2** |
| 38 | Bloqueos de la batalla activa | `avanzarTick.ts:38`, `batallas.ts:73,110` | 1 | | derivado |
| 39 | Abrir combates que el tick no resolvió | `avanzarTick.ts:70` | ✔ | | |
| 40 | **Anteponer los eventos al log en memoria** | `comandos/tipos.ts:91-102` (`exito`) | **2** | 0,05→1,9 | **N2**: copia el log entero en cada mutación: dos por tick (tick y turno NPC; el auto-comercio apagado no muta) |
| 41 | Auto-comercio de simulación | `comandos/avanzarAutoComercio.ts` | — | ~0 | solo batch, palanca apagada |
| 42 | Turno del NPC de gobernanza | `npcGobernanza.ts:1498` | mixto | 22-38 | tabla propia, §4.3 |
| 43 | Reloj de mundo y catch-up | `runnerDePartida.ts:383-432` | 1 | | infra |
| 44 | Precios (TTL perezoso) y geometría (por identidad) | `runnerDePartida.ts:240-267` | ✔ | | derivan al leer; el patrón a copiar |
| 45 | Proyección y niebla "viéndolo ahora" | `proyecciones/jugador.ts:457-519` | ✔ | | derivada en cada lectura |
| 46 | Mantenimiento de disco | `server/mantenimiento.ts` | 1 | | periódico, opt-in |

### 4.3 NPC de gobernanza (`npcGobernanza.ts:1498`)

La pregunta de la petición era *¿decide cada tick lo que podría reaccionar a eventos?* **La mayoría de lo que
decide es política por umbrales de variables continuas** (stock, déficit, solvencia): un bot que mira cada minuto lo
que un jugador miraría de vez en cuando. Eso no está mal asignado. Pero el paso **más caro de todos** es una
excepción: re-deriva cada tick una condición discreta.

| Paso | Ubicación | Reacciona a | Cat. | ms |
|---|---|---|---|---|
| **Reclutar (`reclutarParaTodos`)** | `644-679` | **obra terminada** (Barracón/Galería), escuadra que pierde unidades, población, recursos | **2 → N1** | **19-20** |
| **Publicar órdenes (`publicarOrdenesNpc`)** | `1043-1090` | umbrales de stock, y "no tengo ya una activa" | 1, pero con **N2** | 0,4→12,5 |
| Fundar el asentamiento inicial | `1514-1535`, `1421` | crear la Facción NPC (ya lo hace el comando) **o perder la última plaza** | **2 → (C)** | ~0 |
| `asegurarGobernanzaBase` | `232`, `1540` | plaza fundada o conquistada, cargo vacío, `nivelActual` 2 | 3, invariante multi-camino | ~0 |
| `materializarFundadoresNpc` | `1306`, `1543` | una caravana fundó una plaza NPC | ✔/no mover (§7) | ~0 |
| `responderPropuestasNpc` | `513`, `1601` | llega una propuesta | 3 → **(I)** | ~0 |
| `guarnecerNpc` / `prepararDefensaNpc` | `689`, `713`, `1644-1647` | escuadra nueva, héroe vuelve, cambia la residencia | 3 → **(F)** | 0,4 |
| `replegarLosQueYaTerminaron` | `1198`, `1702` | una columna acampa | 3: repliega también las que quedan `estacionado` por otros caminos | |
| Granjas, comercio, núcleo militar, muralla | `299-429`, `1551-1575` | umbrales de recursos o de nivel | 1 | 0,4-0,6 |
| Ascenso, trueque de supervivencia, atacar campamentos, campañas, persecuciones, expandir | `1580`, `553`, `750`, `1093`, `976`, `847` | umbrales o geometría | 1 | ~0 |

---

## 5. Categoría 2 — mal asignadas

Formato de cada ficha: ubicación · hecho · dónde se produce hoy · qué cambia al moverlo · riesgo · esfuerzo.
**N1** y **N2** son nuevas de esta segunda pasada y van primero porque son las que pesan.

### (N1) El NPC re-deriva cada tick si tiene Barracón o Galería, y se entera lanzando excepciones

- **Ubicación.** `reclutarParaTodos` (`npcGobernanza.ts:644-679`): para cada residente, prueba las tropas de
  `TROPAS_POR_PREFERENCIA_NPC` (`620`, todas, de mejor a peor) hasta que una sale, y captura
  `ReclutamientoInvalidoError` en cada fallo.
- **Hecho que lo desbloquea.** Que exista el edificio de la tropa en su nivel interno: termina una obra
  (`construction.ts:1265`) o una mejora (`aplicarMejoraTerminada`, `construction.ts:1024`). Las dos ya emiten
  `construccion.edificio_completado` y `construccion.mejora_completada` en el **mismo tick que el NPC sigue**.
  Mucho menos a menudo, perder unidades (combate, hambre) o crecer la población.
- **Medido.** ~1 050 intentos por tick con 20 plazas; 5 éxitos. 950 fallan por falta de edificio, 95 por "ya al tope".
  ~20 ms/tick, plano en el tiempo; **29 % del tiempo total en el constructor del error** (traza de pila incluida) y
  9 % en `poblacionDisponibleParaReclutar`, que `reclutarTropa` calcula **antes** de comprobar el edificio
  (`tropas.ts:105` frente a `tropas.ts:115`).
- **Qué se rompería al moverlo.**
  - *Comportamiento:* nada si se descartan antes solo las tropas que el motor iba a rechazar igualmente (sin edificio,
    escuadra al tope): mismo resultado, sin excepción.
  - *Batch:* **no es byte-idéntico**, aunque sí equivalente. El `contador` de ids se consume por intento
    (`contador++` dentro de la llamada), así que dejar de intentar desplaza los ids de lo que se cree después. Es el
    mismo matiz que Doc 6 dejó escrito para la búsqueda de colocación ("lo único que no preserva son los ids").
  - *Reordenar las comprobaciones de `reclutarTropa`* (edificio antes que población) cambia el **mensaje** que ve un
    jugador humano cuando fallan las dos cosas a la vez; conviene decidirlo aparte.
  - `snapshot_baseline` y `determinismo` no lo ven: no corren el NPC.
- **Dos formas de hacerlo**, para que se decida: (a) **comprobar barato antes de intentar**: lee el estado, no
  necesita evento y no puede perderse uno; (b) que el NPC mantenga sus tropas desbloqueadas reaccionando a los
  eventos de obra del tick que acaba de correr. La (a) es más simple y robusta (sobrevive a una recarga sin
  reconstruir nada); la (b) es la versión "por hecho" pura.
- **Riesgo:** bajo. **Esfuerzo:** bajo (S). **Impacto:** **alto**: el mayor coste del tick completo.

### (N2) Barrer cada tick historia que solo crece

- **Qué.** Colecciones cuyos elementos llegan a un **estado terminal** y nunca salen de la lista viva que el tick y el
  NPC recorren:

  | Colección | Se vuelve terminal en | La recorren cada tick | Medido |
  |---|---|---|---|
  | `ordenes` (expirada, cumplida) | `caducarOrdenes` (`market.ts:89`), el mostrador | `caducarOrdenes`; **`publicarOrdenesNpc` → `yaTiene` (`npcGobernanza.ts:1054`)**, que hace `[...ordenes, ...nuevas].some(...)` por cada plaza y recurso | 0,4 → 12,5 ms en 3 000 ticks; 93 % expiradas |
  | `eventosDominio` (en memoria) | al emitirse | `exito()` (`tipos.ts:99`) hace `[...nuevos, ...todoElLog]` en cada mutación: el tick y el turno NPC (el auto-comercio apagado no muta) | 0,05 → 1,9 ms |
  | `acuerdos` (expirado, cumplido, rechazado) | `trade.ts:876-905`, comandos | `avanzarComercio` (copia a un `Map`), `responderPropuestasNpc`, `yaTieneAyudaEnCaminoPara` | no medido: no se cerró ninguno en la corrida |
  | `batallas` (cerrada, fallida) | `resultadoBatalla.ts`, `vencerBatallas` | `batallasActivas` y `vencerBatallas` filtran todas cada tick | no medido |

- **Hecho que debería resolverlo.** El **paso a estado terminal**: es puntual, tiene pocos sitios de escritura y es
  justo cuando deja de importarle al tick. Lo que conviene **no** perder: la proyección enseña a propósito el
  historial de órdenes propias ("las cumplidas son el historial de tu mercado", `jugador.ts:781-785`), y el log de
  eventos en memoria es el cursor incremental de los clientes (`eventosDesde`). Archivar no es borrar: es dejar de
  recorrerlo cada tick.
- **Qué se rompería.** Con un índice de activas, o separando "vivas" de "archivadas", nada de comportamiento. Si se
  cambia la forma del estado persistido, hay migración de snapshot y `BALANCE`/snapshot sube de versión. Con una
  cota en el log de memoria, un cliente con un cursor muy viejo tendría que leer del JSONL en vez de la memoria.
- **Riesgo:** bajo para el índice (que es lo que paga el coste medido); medio si se reestructura el estado.
  **Esfuerzo:** S para el índice de `publicarOrdenesNpc`; M para archivar de verdad.
  **Impacto:** **alto a medio plazo**: es el único coste del tick que crece con la edad de la partida y no con el
  tamaño del mundo.

### (A) El nivel de Facción se recalcula al final del tick, aunque la XP se gane en un comando

- **Ubicación.** `simulation.ts:298-300`; `faccion.ts:54-79`.
- **Hecho.** Ganar experiencia. El nivel es función pura de `experiencia` (`calcularNivelFaccion`, `faccion.ts:25`).
- **Dónde se produce.** Seis funciones de `engine/combate.ts` llaman a `aplicarAjustesExperiencia` directamente:
  `iniciarAsedio:383`, `atacarCampamentoBandidos:457`, `atacarCampamentoConColumna:515`,
  `asediarConEjercito:608,650` y `encuentroEntreEjercitos:696`. Las invocan comandos (`militar.ts`,
  `interaccion.ts`, `resultadoBatalla.ts`) o el turno NPC. Ningún comando llama a `avanzarNivelesFaccion`: solo lo
  hace `simulation.ts`.
- **El defecto.** El comando devuelve la XP nueva con el **nivel viejo**; el nivel y `faccion.nivel_subio` llegan en
  el tick siguiente. En ese hueco leen el nivel el cap de fundación (`settlement.ts:235`, `expansion.ts:97`), los
  slots de política (`politicas.ts:58`) y el cupo de nivel de asentamiento (`ascenso.ts:84`).
- **Qué se rompería.** Nada del pipeline: después de `avanzarEjercitos` nadie lee `faccion.nivel`.
  `eventosDominioFaccion.test.ts` llama a `avanzarNivelesFaccion` directamente y habría que repuntarlo. En batch,
  el nivel subiría en el mismo turno NPC en vez de en el tick siguiente: deja de ser byte-idéntico si hay combate NPC.
  `snapshot_baseline` y `determinismo` no se mueven, porque solo tienen XP de construcción y esa ya se recalcula en el
  mismo tick.
- **Punto único de escritura:** `aplicarAjustesExperiencia`. **Riesgo:** bajo. **Esfuerzo:** S. **Impacto:**
  medio-bajo (ventana ≤1 min, visible al jugador).

### (B) Una ruina deja a sus dependientes para que el tick los descubra

- **Ubicación.** El hecho: `simulation.ts:224-227` (`destruido`, `mantenimiento.ts:260-275`). Es un único sitio.
- **Dependientes y cómo se enteran hoy:**

  | Dependiente | Cómo se entera | Resultado |
  |---|---|---|
  | Caravana comercial que sale de o va a la plaza | `trade.ts:551-556`, barrido por caravana | se pierde **sin evento**; libera la escolta |
  | Caravana de fundación que sale de ella | `expansion.ts:187-193` | se pierde con `expansion.caravana_perdida` |
  | **Campamento de bandidos asignado** | **nunca** | huérfano para siempre: **B1** (reproducido) |
  | **Trueque activo con ella** | solo al vencer (`trade.ts:887`) | penaliza a la otra parte: **B2** (reproducido) |
  | Ejército con ella de origen | al volver a casa (`reintegrar`, `ejercitos.ts:1457`) | se resuelve tarde |
  | **Cualquier referencia por id** | **nunca** | se reengancha si el id vuelve a nacer: **B3** (reproducido) |
  | Residentes | `9f9a098`, fuera de este árbol | — |

- **Qué se rompería.** Cerrar los dependientes en el sitio de la ruina adelanta cosas que hoy ocurren más tarde en el
  mismo tick (el comercio va después, `simulation.ts:229-231`): eventos reordenados (`determinismo` lo detecta si
  cubriera ruinas, que no las cubre) y batch no byte-idéntico, porque las ruinas son la causa de muerte que miden los
  diarios. Los trueques pasarían a `expirado` sin penalizar a nadie, que es justo el cambio buscado.
- **Riesgo:** medio. **Esfuerzo:** M. **Impacto:** medio (arregla B1-B3 de raíz y deja de narrar tarde).

### (C) El NPC re-funda su asentamiento inicial mirando cada tick si tiene alguno

- **Ubicación.** `npcGobernanza.ts:1514-1535`; `fundarAsentamientosIniciales` (`1421-1465`).
- **Hecho.** Dos casos que hoy comparten el mismo `if`: (1) crear la Facción NPC, ya resuelto en `crearFaccionNpc.ts`;
  (2) **perder la última plaza** por conquista (`militar.ts:127`, `resultadoBatalla.ts:225`, `ejercitos.ts:1239`) o por
  ruina (`simulation.ts:226`).
- **Mezcla de casos.** El caso (2) es una **resurrección inmediata** de una Facción derrotada. El código la admite en
  un comentario ("los que ya tenga (se quedó sin asentamientos)"), pero ningún `Docs/Game` la declara como regla. Si
  no encuentra sitio, reintenta cada tick. Y su búsqueda de posición es determinista: con **B3**, refundar en el
  mismo sitio puede devolver el id de la plaza perdida.
- **Qué se rompería.** Poco: Paso 0 solo corre con `config.faccionesIds` (partida real), **no en batch**. Pide una
  decisión de diseño: resurrección inmediata, con retardo o ninguna.
- **Riesgo:** bajo. **Esfuerzo:** S-M (lo caro es la decisión). **Impacto:** bajo.

---

## 6. Categoría 3 — dudosas

Las que la primera versión mandaba "medir" (**D**, **G**, **H**) ya están medidas y **no pesan**. Se quedan aquí solo
por si hay razón de corrección.

| | Qué | Medido | Veredicto |
|---|---|---|---|
| **(D)** | Spawn de bandidos: guarda por conteo (`bandidos.ts:79`) y reintento cada tick si no hay bosque libre | 0,1-0,2 ms | **no** por coste; la guarda se arregla como bug (**B1**) |
| **(E)** | Títulos recalculados cada minuto sobre oro y tropa, y narrados en cada cambio de manos | 0,2-0,4 ms | dudoso por **ruido**: con dos Facciones parecidas el título puede oscilar y narrarse cada vez. Darle cadencia propia o histéresis cambia qué evento sale en qué tick (`determinismo`, `eventosDominioTitulos.test.ts`). No es un evento |
| **(F)** | NPC: guarnición y loadout reescritos cada tick; `guardarLoadout` (`heroe.ts:83`) siempre devuelve objetos nuevos y `asignarGuarnicion` falla por excepción | 0,4 ms | hacerlo idempotente si se toca el NPC; **no mover**: cuatro caminos de ruptura, uno dentro del motor |
| **(G)** | Zonas, reclamos, `factorLineaProduccion`, `campamentoDe`, derivados recalculados | ~1 ms (zonas) | **no** por ahora |
| **(H)** | Memoria de niebla: `marcarVisto` por ojo y ficha reescrita con `conocidoEn` mientras se ve | ~0,6 ms | **no mover** (§7); la micro-optimización no compensa |
| **(I)** | NPC: responder propuestas en su turno (≤1 tick de latencia) | ~0 | decisión de comportamiento, no corrección |
| **(J)** | Columna `marchando` con la ruta acabada esperando a que sane un héroe o acabe una batalla (`ejercitos.ts:1582`) | ~0 | sanar no tiene punto de disparo (`heridoHasta` es perezoso); dejar |

---

## 7. Lo que NO conviene mover, y por qué

| Qué | Por qué se queda |
|---|---|
| **Caducidades por fecha** (políticas, trueques, órdenes, obras, mejoras, ascenso, `preparaHasta`, ocupación, batallas, regeneración, respawn) | Ya son "eventos baratos": la fecha vive en la entidad (Doc 10 §6; D3 cambió contadores por `completaEn`). Un scheduler añadiría una segunda fuente de verdad y sacaría los efectos del punto exacto del pipeline (`avanzarAscenso` va antes de la nutrición y el crecimiento, así que el techo de población del nivel nuevo aplica ese mismo tick). Lo que sí conviene es que no recorran los **terminales** (N2). Revisar solo si llega el scheduler de comandos programados (D5) |
| **Producción, consumo, crecimiento, mantenimiento, ración, tributo, reputación** | Tasas por minuto. No hay hecho |
| **Auto-construcción, mejoras automáticas, recetas, muralla** | Umbral de stock. Ya memoizadas y con guardián (Doc 6 E3) |
| **El NPC por umbrales** (granjas, comercio, ascenso, trueque de supervivencia, atacar, campañas, persecuciones, expandir) | Es el agente mirando variables continuas, como un jugador. Moverlas a los comandos rompe su principio rector (decidir después del tick, con las funciones públicas) y la comparabilidad del batch. Medido, ninguna pesa |
| **`asegurarGobernanzaBase` y la disolución de columnas vacías** | Sostienen un **invariante con caminos de ruptura abiertos**. Un evento por camino deja un caso atrás. Es la excepción a "resolver en el hecho": aplica cuando los hechos son enumerables (la residencia), no cuando son abiertos |
| **`materializarFundadoresNpc`** | Crear el héroe bot dentro de la fundación obligaría al motor a construir `Heroe` bot (`heroeBot` vive en `session/`): cruza capas. La ventana es nula: el NPC va en el mismo `aplicarYPersistir` que el tick (`runnerDePartida.ts:346-352`) |
| **Fichas de la memoria de niebla** | `conocidoEn` es "la última vez que la vi". Mientras se ve, es ahora, y la única forma de tener la última fecha exacta es escribirla mientras se mira. Habría que enganchar cada mutación de posición. La ventana ya la cubre la proyección, que calcula `visibles` en vivo (`jugador.ts:498-529`) |
| **Proyección, precios, geometría** | Ya derivan al leer; un evento sería guardar lo derivable |
| **`heridoHasta`, `ocupacionHasta` (lógica)** | Ya perezosos (`estaHerido`, `estaOcupado`) |
| **Reloj de mundo, mantenimiento de disco** | Infraestructura periódica por naturaleza |

**Dónde el principio ya está bien aplicado** (útil como ejemplo): cupo y gates del ascenso al **pedirlo**
(`ascenso.ts:119-190`), asedio como orden (`98f4e4c`), conquista con `aplicarConquista` + `desalojarResidentes` en el
acto, `radioPotencial` por edificio completado, combates de Unity abiertos desde el hecho (`avanzarTick.ts:70`).

---

## 8. Bugs y observaciones aparte

Todos los bugs están **reproducidos** con scripts sobre el motor real (`avanzarSimulacion`) o la sesión real
(`GameSession`), sin tocar el repositorio.

### Bugs

**B0 · Todos los campamentos de bandidos se llaman `campamento-0`. Grave.** `avanzarSpawnBandidos` construye el id
con un `contador` que por defecto vale 0 (`bandidos.ts:77,90`), y `simulation.ts:247` no se lo pasa. Consecuencias,
las tres por id:
- `exigirCampamento` (`comandos/ayudas.ts:91`) devuelve **el primero** con ese id; `atacarCampamento` valida la
  distancia contra **ese** (`ejercitos.ts:1187`). Un jugador al lado de cualquier otro campamento recibe "Hay que
  estar a menos de N" y **solo puede atacar el primero que apareció en el mundo**.
- Destruir uno **los borra todos**: `campamentosBandidos.filter((c) => c.id !== campamento.id)` en
  `interaccion.ts:213`, `resultadoBatalla.ts:204` y en el NPC, `npcGobernanza.ts:815`.
- La proyección (`jugador.ts`) y los eventos llevan ids repetidos al cliente.

*Reproducido:* en la sesión con 20 NPC, al tick 1 250 había 3 campamentos con **1 id distinto**; al 1 500, 0 (el NPC
destruyó uno). `bandidos.test.ts` no lo ve porque pasa `contador` a mano.

**B1 · Campamentos huérfanos que bloquean el spawn.** Una ruina no retira el campamento asignado, y la guarda
`campamentos.length >= asentamientos.length` (`bandidos.ts:79`) cuenta al huérfano. *Reproducido:* 3 plazas con 3
campamentos; una cae (quedan 2 plazas y 3 campamentos); se funda otra; **60 ticks después sigue sin campamento** y el
huérfano sigue en el mundo atacando caravanas. El NPC lo excluye a propósito ("si el asentamiento asignado sigue
vivo", `npcGobernanza.ts:732`), así que nadie lo retira. La guarda correcta ya existe:
`asentamientoSinCampamento` (`bandidos.ts:42`).

**B2 · Penalización de reputación por la ruina del socio.** Un trueque activo con una plaza que cae no se cierra; la
otra parte ya no puede enviar (`trade.ts:936` descarta lados sin destino), y al vencer se le aplica
`penalizacionTruequeIncumplido` (`trade.ts:898-901`). *Reproducido con control*, mismos datos y bandidos
desactivados: **sin ruina**, A entrega 50/50 y su reputación queda en 0; **con la ruina del socio**, entrega 0/50 y
cae a −8 (−3,6 tras el decaimiento).

**B3 · Un id de asentamiento puede renacer.** `fundarAsentamiento` usa
`asentamiento-${asentamientosExistentes.length}-${x}-${y}` (`settlement.ts:257`). Tras una ruina, la longitud baja, y
refundar en la misma posición repite el id de la plaza perdida. *Reproducido:* arruinada `asentamiento-2-120-1440`,
refundada en el mismo punto → `asentamiento-2-120-1440`. Cualquier referencia vieja por id (campamento huérfano,
trueque aún activo, ejércitos y caravanas con ese origen, ficha de memoria, historial) se reengancha en silencio a
una plaza que puede ser de otra Facción. Es más probable de lo que parece: la búsqueda de posición del NPC es
determinista y tiende a repetir el mejor sitio (**C**).

### Observaciones (confirmar si son deliberadas)

**O1 · Los ticks del reloj de mundo no se difunden por WebSocket.** `HubDeDifusion.difundir` solo se llama desde
`rutas/comandos.ts:151`, `rutas/batallas.ts:156` y `rutas/admin.ts:418`. El camino
`iniciarRelojDeMundo → sincronizarConReloj → unTickCompleto` (`runnerDePartida.ts:383-432`) persiste y anexa al
JSONL, pero no avisa a nadie. Y `ResultadoComando` de un tick solo trae los eventos de la primera mutación
(`runnerDePartida.ts:517-521`), así que ni el tick manual difunde lo del NPC. Es el problema inverso al de esta
auditoría: un hecho sin su consecuencia.

**O2 · Lo que sobrevive a una conquista.** `aplicarConquista` (`combate.ts:156-209`) conserva `politicasActivas`,
`reservaManual` y `autoConstruccionPausada` del dueño derrotado. Las políticas siguen ocupando slots de un cargo
vacío hasta caducar, y la reserva del antiguo Tesorero bloquea gasto del nuevo dueño.
`Ocupacion_Post_Conquista_Definicion.md` no lo trata.

**O3 · Excepciones como control de flujo en el NPC.** Además de N1 (el caso caro), pasa en
`anadirEdificioManualmente`, `comprometerRecintoManualmente` y `asignarGuarnicion`: cada tick prueban y capturan.
Hoy suman ~1 ms; es el mismo patrón y conviene saberlo antes de añadir más pasos al NPC.

**O4 · Tecnología y Eras.** No hay ninguna mecánica de tecnología por tick (BA-006 sigue en diseño). Cuando llegue,
conviene diseñarla **por hecho** (obra o hito que la desbloquea) y no como condición re-derivada cada minuto: es
exactamente el patrón de N1 y (A).

---

## 9. Tabla priorizada y orden recomendado

Impacto = lo que gana el juego o el servidor. Riesgo = probabilidad de cambiar comportamiento observable o los
guardianes de §2. Esfuerzo: S = horas, M = un día, L = varios.

| # | Qué | Cat. | Impacto | Riesgo | Esfuerzo | Batch byte-idéntico | `snapshot_baseline` / `determinismo` |
|---|---|---|---|---|---|---|---|
| **B0** | Ids únicos de campamento | bug | **alto** (rompe el combate con bandidos) | bajo | S | no (ids) | no / no |
| **N1** | NPC: no intentar reclutar lo que el motor va a rechazar | 2 | **alto** (−20 ms/tick, 44-64 % del tick completo) | bajo | S | no (ids), sí equivalente | no / no |
| **N2a** | Índice de órdenes activas en `publicarOrdenesNpc` | 2 | **alto** a medio plazo (crece sin cota) | bajo | S | sí | no / no |
| **B1** | Guarda de spawn por "plaza sin campamento" y retirar huérfanos | bug | medio | bajo | S | solo con ruinas | no / no |
| **B2** | Cerrar trueques con una plaza arruinada sin penalizar | bug | medio | bajo | S | solo con ruinas | no / no |
| **B3** | Ids de asentamiento que no se reutilicen | bug | medio (corrupción silenciosa de referencias) | medio (formato de id) | S | no (ids) | **sí** si el id aparece en el resumen o los eventos |
| **A** | Nivel de Facción al ganar la XP | 2 | medio-bajo | bajo | S | no con combate NPC | no / no |
| **N2b** | Archivar terminales (órdenes, trueques, batallas) y acotar el log en memoria | 2 | medio | medio (forma del estado) | M | depende | no / no |
| **B** | Cerrar los dependientes de una ruina en el sitio de la ruina | 2 | medio | medio | M | no | no / según orden de eventos |
| **C** | NPC: re-fundación al perder la última plaza, no cada tick | 2 | bajo | bajo | S-M (decisión) | sí (no corre en batch) | no / no |
| **E** | Cadencia o histéresis declarada de los títulos | 3 | bajo (ruido) | bajo | S | no | no / **sí** |
| **F** | NPC idempotente en guarnición y loadout | 3 | bajo | bajo | S | sí | no / no |
| **D, G, H, I, J** | — | 3 | medido despreciable o decisión de diseño | — | — | — | no tocar |

**Orden recomendado.**

1. **B0.** Es un arreglo de una línea con un efecto de juego grave.
2. **N1 y N2a.** Juntas son ~32 ms de los ~45 del tick completo a los 3 días con 20 NPC, y N2a crece sin cota. Las dos
   se resuelven **comprobando antes o indexando**, sin eventos ni cambio de reglas. Conviene aceptar a la vez el
   desplazamiento de ids del batch (es el mismo trato que Doc 6 E3).
3. **B1, B2 y B3.** Bugs con arreglo local; B3 antes de que haya partidas largas con ruinas y refundaciones.
4. **(A).** El caso más limpio del principio. Sirve de ensayo de "mover una consecuencia al hecho" con riesgo bajo.
5. **(B), y N2b si N2a no basta.** Son los más grandes; se hacen con los sellos del batch delante, aceptando que
   cambian.
6. **(C), (E), (F)** cuando haya hueco. **D, G, H, I, J y todo §7: dejar.**

**El criterio que conviene dejar escrito** junto al caso de la residencia:

- **Mover al hecho** cuando los hechos son pocos y enumerables y cada uno ya tiene un sitio de escritura. Ejemplos:
  residencia (conquista, ruina, dejar casa), nivel de Facción (`aplicarAjustesExperiencia`), ruina (un único
  `destruido`), paso a estado terminal.
- **Mantener el barrido** cuando sostiene un invariante que puede romperse por caminos abiertos.
- **Comprobar barato antes, o memoizar con clave**, cuando lo que se recalcula es una lectura (N1).

---

## 10. Qué cambió respecto a la primera versión

- **Las prioridades se invierten.** La primera versión, solo leyendo, ponía al frente el nivel de Facción y la ruina y
  daba por hecho que el tick era barato. Medido, el coste está en el **turno del NPC** (N1, N2), que la primera
  versión había dado por legítimo "por umbrales" sin ver que su paso más caro re-deriva una condición discreta.
- **Nuevos:** N1, N2 (órdenes y log de eventos), **B0** (ids de campamento) y **B3** (ids de asentamiento).
- **B1 y B2 pasan de "por lectura" a reproducidos.** B2 con escenario de control; el primer intento no lo aislaba.
- **D, G y H dejan de ser "medir".** Están medidos y son despreciables. La recomendación de un perfil por etapa del
  tick ya está hecha (§3).
- **Se corrigen números de línea** de la primera versión que estaban desplazados.
