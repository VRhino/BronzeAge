# Ritmo de crecimiento de los asentamientos

**Abierto el 2026-09-25.** Sale de la revisión de BA-006: **D49** fija qué nivel de asentamiento le toca a cada
Era, y **D34/D44/D48** fijan cuánto dura cada Era. Para saber si D49 es cierto hace falta saber **cuánto tarda
de verdad un asentamiento en subir de nivel**, y eso hoy no está medido. Aquí está el mecanismo tal como vive
en el código, lo que se puede medir con lo que ya hay, y las palancas para balancearlo.

Sigue en modo diseño: aquí no se toca código.

## 1. Qué hace subir de nivel a un asentamiento

`NIVEL_ASENTAMIENTO.requisitos` (`src/constants.ts`) — gates puros: a la vez un mínimo de población **y** unos
edificios. El nivel es monótono (nunca baja).

| Nivel | Pesants | Artesanos | Edificios |
|---|---|---|---|
| 2 | 200 | 0 | 3 de 7 tipos de extracción (cantera, leñera, granja, mina, mina de cobre, mina de estaño, corral) |
| 3 | 500 | 200 | **los 5**: armería, curtiduría, fundición, barracón, galería de tiro |
| 4 | 1000 | 400 | un **recinto terminado** (la empalizada basta) |
| 5 | 2000 | 800 | **Palacio** |

Las cifras de los niveles 4 y 5 están marcadas como PLACEHOLDER en el propio código, sin calibrar.

## 2. La cadena real que gobierna el ritmo

No es una cosa, son cinco encadenadas:

1. **Vivienda.** El crecimiento es logístico contra el cupo de Vivienda:
   `tasa = base × comida × (1 − población/cupo)` (`crecerPoblacion`, `src/engine/population.ts`). Cada Vivienda
   da **15 pesants y 5 artesanos**, cuesta 10 de madera y tarda 4 minutos.
2. **Tope de Viviendas por nivel.** `maximoViviendasPorNivel` (`src/engine/construction.ts:445`) se deriva del
   gate siguiente: el mayor entre `pesants_siguiente/15` y `artesanos_siguiente/5`. O sea, **el cupo de
   población está pegado exactamente al gate que hay que cumplir**, ni uno más.

   | Nivel actual | Tope de Viviendas | Cupo que da |
   |---|---|---|
   | 1 | 14 | 210 pesants / 70 artesanos |
   | 2 | 40 | 600 / 200 |
   | 3 | 80 | 1200 / 400 |
   | 4 | 160 | 2400 / 800 |
   | 5 | 1334 | techo de 20 000 |

3. **Artesanos.** No aparece ninguno hasta el primer edificio de transformación activo, y crecen a 0.1/minuto
   contra su propio cupo (5 por Vivienda). **Es el reloj lento del sistema**: los gates piden más proporción de
   artesanos que la que la Vivienda regala (por eso el tope se calcula con el mayor de los dos, tras un
   deadlock real ya corregido).
4. **Obra.** `NECESIDADES.maximoEnConstruccionSimultanea: 2` y `maximoEnCola: 4`: solo dos edificios
   construyéndose a la vez. Levantar las Viviendas de un nivel cuesta, solo en tiempo de obra, 28 / 80 / 160 /
   320 minutos para los niveles 1 a 4.
5. **Comida y madera.** La Granja rinde 60 de trigo/minuto (se dobló dos veces) contra 0.1 por habitante: una
   sola granja alimenta a 600. Hoy **la comida ya no es el freno**, y está medido:
   `issues/npc_no_alcanzan_nivel_3.md` (triplicar el trigo no movió el nivel 3 ni un asentamiento).

**La unidad de tiempo:** `SIMULACION.duracionTickMs = 60_000`. Un tick es **un minuto de mundo, 1:1 con el
tiempo real**. Todas las tasas de arriba son por minuto. Una season de 12 meses son **525 600 ticks**; la Era I
(5 semanas, D44) son **50 400 ticks**.

## 3. El desajuste

Con las tasas de hoy (0.12/min pesants, 0.1/min artesanos) y la fórmula logística, el tiempo de llenado de cada
gate sale así, suponiendo que la Vivienda y los recursos no estorban:

| Salto | Pesants | Artesanos | Obra de Viviendas |
|---|---|---|---|
| 1 → 2 | ~44 min | — | ~28 min |
| 2 → 3 | ~19 min | ~100-150 min | ~80 min |
| 3 → 4 | ~16 min | ~40-90 min | ~160 min |
| 4 → 5 | ~16 min | ~40-90 min | ~320 min |

**Sobre el papel, la escalera entera cabe en menos de un día de mundo**, contra una Era I de 50 400 ticks: dos
órdenes de magnitud de diferencia entre el reloj del crecimiento y el del calendario de Eras.

### Lo que dice la medición (2026-09-25)

Corrida real: `BATCH_SEED=7 BATCH_FACCIONES=30 BATCH_TICKS=6000 BATCH_FOTO_CADA=250` (6000 ticks = 4 días de
mundo, un 1,1 % de una season).

| Tick | Vivos | Colapsados | n1 | n2 | n3 | n4 | n5 | Pesants med. | Artesanos |
|---|---|---|---|---|---|---|---|---|---|
| 250 | 37 | 0 | 9 | **28** | 0 | 0 | 0 | 249 | 438 |
| 500 | 34 | 14 | 17 | 14 | **3** | 0 | 0 | 411 | 4012 |
| 750 | 48 | 21 | 27 | 19 | 0 | **2** | 0 | 365 | 4072 |
| 1500 | 68 | 57 | 52 | 14 | 0 | 2 | 0 | 337 | 4387 |
| 3000 | 71 | 92 | 58 | 11 | 0 | 2 | 0 | 321 | 3785 |
| 6000 | 69 | **126** | 57 | 10 | 0 | 2 | 0 | 323 | 3554 |

Lo que sale de ahí, y **no es lo que la aritmética hacía esperar**:

1. **El nivel 2 llega rapidísimo**: 28 de 37 asentamientos a tick 250 (cuatro horas de mundo). Eso sí confirma
   el desajuste de relojes.
2. **El nivel 3 es un pico, no una meseta**: tres asentamientos en el tick 500 y **cero desde el tick 750 hasta
   el final**. No es que se tarde en llegar: es que no se sostiene.
3. **El nivel 4 son dos asentamientos, congelados** desde el tick 750. Son los dos únicos con recinto completo.
4. **El nivel 5 no se alcanza nunca**: cero Palacios en toda la corrida.
5. **126 colapsos contra 69 vivos.** La nutrición está a 100 todo el rato, así que no es comida. Los
   asentamientos nuevos que se fundan para reemplazarlos se quedan en nivel 1 (de 9 a 57 en la corrida).

### La medición que importa: edad al subir de nivel

Con la instrumentación nueva (misma corrida, 192 asentamientos nacidos en 6000 ticks):

| Nivel | Llegaron | Mediana | p90 | Máx | Objetivo (§6) | Factor que falta |
|---|---|---|---|---|---|---|
| 2 | 118 (61 %) | **174 ticks** (~3 h) | 274 | 589 | ≥ 10 000 | **×58** |
| 3 | 3 (2 %) | **406 ticks** (~7 h) | 439 | 439 | 50 000-110 000 | **×120-270** |
| 4 | 2 (1 %) | **724 ticks** (~12 h) | 724 | 724 | 110 000-260 000 | **×150-360** |
| 5 | 0 | — | — | — | ≥ 260 000 | nadie construye Palacio |

### Y la causa de los colapsos

Diagnóstico de ruinas de la misma corrida (148 eventos):

- **Duración antes de caer: mediana 280 minutos**, o sea 4 h 40 de vida.
- **Nivel al caer: 84 % en nivel 2**, 15 % en nivel 1, 1 % en nivel 3.
- **Recurso que faltó: oro en el 69 % de las muertes** (solo o con piedra), piedra en el 43 %, madera en el 21 %.

Y `MANTENIMIENTO.nivelParaOro` y `.nivelParaPiedra` valen **2**, los dos.

**La cadena causal queda cerrada:**

1. El asentamiento se funda y llega a 200 pesants en ~174 ticks, tres horas, sin que nada lo frene.
2. Al alcanzar el nivel 2 se le enciende el mantenimiento en **oro y piedra**.
3. A las tres horas de vida no tiene ni economía de oro ni cantera productiva: llegó al nivel 2 con una
   aldea de chozas y 200 campesinos, no con una ciudad.
4. Muere de déficit a las 4 h 40 de mediana.
5. La Facción refunda, y el nuevo repite el ciclo. De ahí los 126-148 colapsos contra ~69 vivos.

**La conclusión honesta:** el problema no es solo que el crecimiento sea rápido, y no son dos problemas
separados. **Son el mismo.** El asentamiento llega al nivel 2 mucho antes de tener con qué sostenerlo, porque
el reloj de la población corre ~60 veces más rápido que el de la economía que debería pagarlo. Frenar el
crecimiento hasta los objetivos de §6 debería arreglar las dos cosas a la vez: un asentamiento que tarda una
semana en llegar al nivel 2 llega con minas, cantera y comercio detrás, y el mantenimiento en oro deja de ser
una sentencia de muerte.

No es un descuido: las tasas se subieron a propósito (comentario en `POBLACION`) *"porque la población tardaba
muchísimos más ticks en duplicarse que un asentamiento en subir de nivel"*. Estaban calibradas para una partida
que se medía en cientos de ticks. El modelo de season de 12 meses es posterior y nunca se recalibró contra él.

### Corrida de una Era I entera (2026-09-26)

`BATCH_SEED=7 BATCH_FACCIONES=12 BATCH_TICKS=50400 BATCH_FOTO_CADA=2520 BATCH_RUINAS_DIAG=1` — las cinco
semanas completas de la Era I. 117 asentamientos nacidos, 157 colapsos.

**Ritmo (confirma la corrida corta, sin mover una coma):**

| Nivel | Llegaron | Mediana | p90 | Máx |
|---|---|---|---|---|
| 2 | 74 (63 %) | 177 ticks | 252 | 329 |
| 3 | 1 (1 %) | 350 | 350 | 350 |
| 4 | 1 (1 %) | 620 | 620 | 620 |
| 5 | 0 | — | — | — |

**Colapsos:** mediana de vida 337 minutos; **97 % caen en nivel 2**; el oro falta en el **75 %** de las muertes.

**Y el hallazgo nuevo: a partir de la semana 2 el mundo se congela y no se mueve en tres semanas.**

| Semana | Vivos | n1 | n2 | n3 | n4 | Pesants | En tope de Vivienda | Falta solo población | Falta solo edificios | En obra | En cola | Madera | Piedra |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 0.5 | 43 | 35 | 7 | 0 | 1 | 315 | 95 % | 0 | 38 | 0.00 | 0.05 | 2869 | 606 |
| 2.0 | 46 | 41 | 4 | 0 | 1 | 298 | 100 % | 0 | 44 | 0.00 | 0.00 | 6637 | 415 |
| 3.5 | 46 | 41 | 4 | 0 | 1 | 290 | 98 % | 1 | 45 | 0.00 | 0.00 | 9846 | 416 |
| 5.0 | 46 | 41 | 4 | 0 | 1 | 287 | 98 % | 0 | 44 | 0.00 | 0.00 | 9949 | 409 |

Las columnas nuevas contestan la pregunta de golpe:

- **`faltaSoloPoblacion` es 0 y `faltaSoloEdificios` es 43-45.** Absolutamente nadie está esperando a tener
  gente: tienen 287 pesants de media contra los 200 que pide el gate del nivel 2. **Lo que les falta son
  edificios.**
- **La obra está parada:** 0.00 en construcción y 0.00 en cola, sostenido durante tres semanas, con **9 900 de
  madera muertas en el almacén**. No es que no puedan pagar: es que no se propone nada.
- **El motivo:** de 46 asentamientos vivos solo hay **18 canteras**, y los extractores minerales están a cero
  (`mina` 0, `minaCobre` 5, `minaEstano` 3, `corral` 3). El gate del nivel 2 pide **3 tipos distintos de
  extracción** y la mayoría se queda en leñera + granja. `conGateNivel2Cumplido` baja de 9 a 5 durante la
  corrida.

Eso es exactamente `issues/extractores_minerales_nunca_se_construyen.md`, **REABIERTO desde el 2026-08-18**.

**Conclusión operativa: con el batch de hoy no se puede calibrar el ritmo de crecimiento.** El mundo no crece
despacio, ni deprisa: se **atasca** por un bug de construcción de extractores que no tiene nada que ver con el
reloj. Lo único que la corrida mide de verdad es la velocidad de los pocos que sí suben —y ahí el ×58 se
confirma—, pero el estado estacionario no dice nada sobre el ritmo porque nadie llega a recorrerlo.

## 4. Por qué esta medición todavía no sirve para balancear

Tres razones, en orden de gravedad:

1. **La rotación contamina todo.** Con 126 colapsos y refundaciones, el reparto de niveles de cada foto mezcla
   asentamientos de edades distintas. Una foto no dice cuánto tardó nadie en subir: dice cuántos hay ahora en
   cada nivel, que es otra cosa. Hasta saber si esa mortandad es un problema del ritmo de crecimiento o una
   patología de la expansión NPC (bots fundando de más), cualquier calibración va a ciegas.
2. **El nivel 5 no se puede alcanzar.** La gobernanza NPC (`src/session/npcGobernanza.ts`) solo añade a mano
   granja, mercado, barracón y galería de tiro: **no construye Palacio**, que es el gate del nivel 5 — cero
   Palacios en 6000 ticks lo confirma. Es el mismo patrón que `issues/nivel_3_inalcanzable_sin_jugador_humano.md`
   tuvo con Barracón y Galería, un nivel más arriba. El nivel 4 sí se alcanza (dos asentamientos con recinto
   completo), pero con una muestra de dos no se calibra nada.
3. **La ventana es corta.** 6000 ticks son 4 días de mundo, el 1,1 % de una season y el 12 % de la Era I.

## 5. Cómo se mide

El arnés es `scripts/run-batch-sim.ts`, determinista por semilla.

**Añadido el 2026-09-25:** el bloque **RITMO DE CRECIMIENTO**, que mide lo que de verdad hacía falta — la
**edad** (ticks desde su propia fundación) a la que cada asentamiento alcanza cada nivel, con mediana, p90 y
máximo, y qué porcentaje de los nacidos llega. Se lee `a.nivel` (nivelAlcanzado, monótono) para que una
degradación temporal no borre el dato. Siempre encendido.

```bash
BATCH_SEED=7 BATCH_FACCIONES=30 BATCH_TICKS=6000 BATCH_RUINAS_DIAG=1 npx tsx scripts/run-batch-sim.ts
```

`BATCH_RUINAS_DIAG=1` ya existía y enciende el diagnóstico de colapsos (duración antes de caer, nivel al caer,
recurso que faltó).

**Reanudar desde un punto de control** (2026-09-26): `BATCH_CHECKPOINT_TICKS` guarda el estado y `BATCH_DESDE`
arranca otra corrida desde él, solo si es compatible con el motor (misma `WORLDGEN_VERSION` y `LAYOUT_VERSION`).
Así la Era II se mide desde el final de la Era I sin repetirla. Uso y garantías en `simulaciones-batch/README.md`.

Lo que sigue faltando:
1. **Que la gobernanza NPC construya Palacio**, o el nivel 5 no se medirá nunca (hoy no lo construye nadie).
2. Correr con `BATCH_TICKS` del orden de una Era real (50 000), aunque sea con pocas facciones — con los
   tiempos objetivo de §6 no hay forma de ver un nivel 3 en 6000 ticks.

## 6. El objetivo: qué debería tardar cada nivel

Sale de cruzar **D44** (duración de cada Era) con **D49** (qué nivel le toca a cada Era) y con el criterio del
usuario del 2026-09-25: *"que se llegue a nivel 2 en 45 minutos no tiene sentido, debería ser al menos una
semana, y para subir a nivel 3 aún más"*.

Una semana = 10 080 ticks. Era I = 50 400. Season = 525 600.

| Nivel | Cuándo debería llegar | Edad objetivo (ticks desde su fundación) | Hoy (medido) | Factor que falta |
|---|---|---|---|---|
| 2 | dentro de la Era I, a partir de la primera semana | ≥ 10 000 | ~250 | **×40** |
| 3 | Era II (semanas 5-11) | 50 000 – 110 000 | ~500, y no se sostiene | **×100-200** |
| 4 | Eras III-IV (semanas 11-26) | 110 000 – 260 000 | ~750, solo 2 casos | **×150-350** |
| 5 | Era V (semana 26 en adelante) | ≥ 260 000 | nunca | — |

## 7. Todos los elementos que intervienen

Siete grupos. Ninguno es suficiente por sí solo: el tiempo real de subir de nivel es el **máximo** de los
cuatro primeros, no su suma, porque corren en paralelo.

### 7.1 El reloj de la población

| Elemento | Dónde | Hoy |
|---|---|---|
| Tasa de crecimiento de Pesants | `POBLACION.pesants.tasaCrecimientoBase` | 0.12 por minuto |
| Tasa de crecimiento de Artesanos | `POBLACION.artesanos.tasaCrecimientoBase` | 0.1 por minuto |
| Tasa de crecimiento de Nobleza | `POBLACION.nobleza.tasaCrecimientoBase` | 0.01, y pide Palacio + 3 ciudadanos |
| Población inicial | `POBLACION.pesants.inicial` | 20 |
| Aparición de Artesanos | `crecerPoblacion` | 0 hasta el primer edificio de transformación activo, luego 1 |
| Factor de comida | `POBLACION.hambre.factorCrecimientoMinimo` | entre 0.2 (hambre) y 1 (nutrición 100) |
| Presión fiscal | `factorCrecimientoPoblacion` (políticas) | 1 sin política activa |
| Ocupación tras conquista | `OCUPACION.factorCrecimiento` | frena mientras dura la ocupación |

**Es la palanca más directa del reloj.** Las dos primeras tasas se subieron a propósito en su día (0.05/0.03 →
0.12/0.1) *"porque la población tardaba muchísimos más ticks en duplicarse que un asentamiento en subir de
nivel"* — una calibración hecha para partidas de cientos de ticks, anterior al modelo de season.

### 7.2 El cupo: cuánta población cabe

| Elemento | Dónde | Hoy |
|---|---|---|
| Capacidad de la Vivienda | `EDIFICIO_CATALOGO.vivienda` | 15 pesants y 5 artesanos por Vivienda |
| Tope de Viviendas por nivel | `maximoViviendasPorNivel` (`engine/construction.ts:445`) | derivado del gate siguiente: 14 / 40 / 80 / 160 |
| Techo de población por nivel | `NIVEL_ASENTAMIENTO.techoPoblacion` | 300 / 1500 / 6000 / 12 000 / 20 000 |
| Capacidad de nobles | `palacio.capacidadNobles` | 200 |

El cupo importa doblemente: es el techo **y** el freno logístico (el crecimiento se apaga al acercarse a él).
Y como el tope de Viviendas se calcula desde el gate siguiente, tocar los umbrales de §7.3 arrastra esto solo.

### 7.3 Los gates de nivel

| Elemento | Dónde | Hoy |
|---|---|---|
| Umbrales de población por nivel | `NIVEL_ASENTAMIENTO.requisitos[n].pesants/.artesanos` | 200/0 · 500/200 · 1000/400 · 2000/800 |
| Edificios exigidos por nivel | `.edificios` y `.edificiosMinimo` | 3 de 7 extractores · los 5 de transformación y militares · recinto · Palacio |
| Gate de construcción de cada edificio exigido | `requisitoNivelAsentamientoConstruccion` | encadena niveles: el Palacio pide nivel 4 y es el gate del 5 |
| Nivel mínimo para levantar recinto | `MURALLA.nivelMinimoConstruccion` | 3 |

### 7.4 La obra: cuánto se tarda en construir

| Elemento | Dónde | Hoy |
|---|---|---|
| Tiempo de construcción de cada edificio | `tiempoConstruccionMinutos` | **3 a 20 minutos**; Vivienda 4, Palacio 20 |
| Cuadrillas simultáneas | `NECESIDADES.maximoEnConstruccionSimultanea` | 2 |
| Cola de proyectos pagados | `NECESIDADES.maximoEnCola` | 4 |
| Ritmo de obra del recinto | `MURALLA.celdasPorMinuto` | 1 celda por minuto |
| Prioridad de la auto-construcción | `SCORE_BANDAS` | supervivencia > extractores > crecimiento > manual > transformación |
| Sitio físico disponible | trazado urbano (`TRAZADO`, anclas, manzanas) | si no hay hueco, no se construye aunque sobre todo lo demás |

**Este grupo es tan responsable como la población y está mucho menos mirado.** Una ciudad entera se levanta hoy
en unas horas porque ningún edificio pasa de 20 minutos.

### 7.5 La economía que paga la obra

| Elemento | Dónde | Hoy |
|---|---|---|
| Coste en materiales de cada edificio | `costo` y `costoMejora` | Vivienda 10 madera; Palacio 1500 madera + 1000 piedra |
| Coste del recinto | `MURALLA.tarifaPorCelda` | 20 madera + 2 piedra por celda (nivel 1) |
| Producción de trigo | `granja.produccionBaseTrigo` | 60/min en nivel 1 (doblada dos veces); una granja alimenta a 600 |
| Producción de madera y piedra | `lenera`, `cantera` | 5/min cada una |
| Capacidad de almacén | `almacen.capacidadPorRecursoAdicional`, `maximoAlmacenesPorNivel` | 300 por almacén; 4 almacenes en nivel 1 |
| Reserva que veta gastar | `RESERVA_CONSTRUCCION` | 8 minutos de mantenimiento y de comida |
| Mantenimiento | `MANTENIMIENTO.costoBase` | 1.5 madera/min, escala con lo construido y con la distancia a la capital |
| Comercio | caravanas y trueques | trae lo que el sitio no da |

### 7.6 Lo que hoy rompe la medición

| Elemento | Dónde | Hoy |
|---|---|---|
| Colapso por déficit sostenido | evento `asentamiento.ruinas` | 126 colapsos en 6000 ticks contra 69 vivos |
| Ritmo de fundación de la expansión NPC | `engine/expansion.ts`, gobernanza NPC | refunda sin parar, y lo refundado se queda en nivel 1 |
| Qué construye la gobernanza NPC | `session/npcGobernanza.ts` | granja, mercado, barracón y galería; **no Palacio** |

### 7.7 Las Eras (todavía sin código)

| Elemento | Estado |
|---|---|
| Plazos máximos de cada tramo (D44) | decidido, sin implementar |
| Logros del servidor: contadores y umbrales | los X están sin fijar a propósito (se calibran con el playtest) |
| Hitos de Facción | definidos para la Era I, borrador en la II, vacíos en III-V |
| Coste de adopción de una tecnología | sin cifras |
| Techo de nivel por Era (§8a) | propuesto, sin decidir |

**`SIMULACION.duracionTickMs` NO es una palanca.** Todo el motor declara sus plazos en minutos de mundo,
incluidas las Eras, así que cambiar cuánto dura un tick estira los dos relojes por igual y deja la proporción
intacta.

## 8. Propuesta

## 9. Propuesta

Dos piezas, no una:

**(a) — Descartado como techo físico (2026-09-26, D54).** El usuario lo quiere DERIVADO: una tecnología de cada
Era desbloquea algo necesario para la subida, y el techo sale solo. La tabla de abajo sigue siendo el objetivo; lo
que cambia es el mecanismo. Texto original de la propuesta:

**D49 como techo duro, no como aspiración.** Que la Era imponga el nivel máximo de asentamiento alcanzable:

| Era | Nivel máximo |
|---|---|
| I | 3 |
| II | 3 |
| III | 4 |
| IV | 4 |
| V | 5 |

Es una puerta del mismo tipo que las cuatro que ya usamos para las tropas (D29), y garantiza la tabla de D49
pase lo que pase con la economía. Sin esto, cualquier calibración se puede desbordar en un servidor rápido.

**(b) Calibrar el reloj para que llegar al techo cueste una parte real de su Era.** Con el techo puesto, la
calibración ya no tiene que ser exacta: solo tiene que evitar que todo el mundo lo toque el primer día.

**Decidido (D54):** ni duro ni blando — derivado de la tecnología.

## 10. Subida de nivel manual y con coste (decidido 2026-09-26; cifras en propuesta)

**Decisión del usuario:** el paso de nivel de asentamiento deja de ser automático. Los gates actuales (población
+ edificios) pasan a ser el requisito para **poder pedir** la subida; la subida la pide alguien y cuesta.

**Aprobado el 2026-09-26:** las cuatro piezas de abajo; **la pide el Gobernador** del asentamiento; **si
conquistan el asentamiento a mitad de obra, la obra se pierde** sin devolución. Las cifras de la tabla son el
punto de partida para calibrar con el batch.

**Quién paga:** el almacén del asentamiento. Los jugadores no tienen almacén de recursos propio: el héroe solo
lleva `inventario` (objetos de equipo, contrato de Conquest) y `monedasHeroe` (bronce/plata/oro), que el canon
declara *"sin relación con el oro recurso"* (Doc 5.16.1). El Gobernador es un cargo: decide en qué se gasta el
almacén del asentamiento, igual que hoy cuando añade un edificio a mano. El "oro" del coste es el oro recurso
del almacén, no las monedas del héroe.

Las cuatro piezas:

1. **Coste de la obra de ascenso**, pagado del almacén del propio asentamiento al empezar. Pide lo que el nivel
   siguiente va a cobrar (piedra y oro desde el nivel 2) y, desde el nivel 3, un bien elaborado de la cadena
   del nivel que se deja. Tiene que caber en el almacén máximo del nivel de partida.
2. **Duración de la obra** (3 días / 1 semana / 2 semanas / 3 semanas): es la palanca de ritmo que no depende de
   la economía y que por tanto aguanta mientras se recalibra lo demás.
3. **Prueba de solvencia:** solo se puede pedir si los ingresos actuales cubren el mantenimiento del nivel
   siguiente. Ataca la causa medida de muerte: el 73 % de los colapsos son en nivel 2 y el oro falta en el 59 %.
   Un stock no arregla un flujo negativo; esto sí.
4. **El NPC tiene que aprender a pedirla**, o el batch se congela en nivel 1 (mismo agujero que el Palacio).

| Ascenso | Madera | Piedra | Oro | Bien elaborado | Obra | Almacén máx. del nivel de partida |
|---|---|---|---|---|---|---|
| 1 → 2 | 600 | 400 | 150 | — | 3 días (4 320 ticks) | 1 600 |
| 2 → 3 | 1 200 | 1 000 | 400 | 100 lingotes de cobre + 100 cuero curtido | 7 días (10 080) | 2 800 |
| 3 → 4 | 2 500 | 2 500 | 1 000 | 150 lingotes de bronce | 14 días (20 160) | 5 200 |
| 4 → 5 | 5 000 | 5 000 | 2 500 | bienes de lujo, si se aprueban los talleres | 21 días (30 240) | 7 600 |

### 10.1 Implementado (2026-09-26)

Por capas, sin mezclar:

| Capa | Qué |
|---|---|
| Dominio | `Asentamiento.ascenso?: AscensoEnCurso` (`domain/types.ts`) |
| Constantes | `ASCENSO_ASENTAMIENTO` — coste y duración por nivel objetivo (`constants.ts`) |
| Motor | `engine/ascenso.ts` (nuevo, puro): `evaluarAscenso`, `iniciarAscenso`, `avanzarAscenso`, `cupoLibreParaNivel`. `evaluarGatesDeNivel` en `engine/mantenimiento.ts`, compartida. El tick ya no sube el nivel: solo termina obras (`engine/simulation.ts`). `aplicarConquista` borra la obra (`engine/combate.ts`) |
| Sesión | comando `solicitarAscenso` (`session/comandos/ascenso.ts`), solo el Gobernador (`autorizacion.ts`), código `ascenso.invalido`. El NPC la pide en `session/npcGobernanza.ts` |
| Servidor | `RunnerDePartida.ascensoDeAsentamiento` + ruta de jugador: `ProyeccionJugador.ascensoDeAsentamiento`, mismo camino que `produccionDeAsentamiento` |
| Batch | bloque "BLOQUEOS DE LA SUBIDA DE NIVEL"; el ritmo mide la edad al **pedir** y al **llegar** a cada nivel; `enObraDeAscenso` en las fotos |

Tests: `engine/__tests__/ascenso.test.ts` (reglas, incluido que el tick ya no sube solo), `session/__tests__/comandosAscenso.test.ts`, `ascensoNpc.test.ts`, autorización y la ruta HTTP de punta a punta.

### 10.2 Lo que midió el batch (2026-09-26)

`BATCH_SEED=7 BATCH_FACCIONES=12 BATCH_TICKS=5040`:

- **Cero colapsos** (132 antes en el mismo tramo).
- Los 12 asentamientos **cumplen los gates del nivel 2**, tienen el almacén lleno (oro y piedra a 1600) y **ninguno pide la subida**. Tampoco hay expansión: el NPC solo expande desde nivel 2.
- Motivo, del bloque de bloqueos: **los 12 son insolventes en oro**. Ingreso mediano **0,84 oro/min** contra un mantenimiento en nivel 2 de **2,84 oro/min**. Uno, además, en piedra.

**Era I entera** (`BATCH_TICKS=50400`, mismas condiciones): idéntico durante las cinco semanas — 12 vivos, 0
colapsos, los 12 en nivel 1 con el almacén lleno (oro y piedra a 1600), cero obras pedidas, y los 12 insolventes en
oro con las mismas cifras (0,84 contra 2,84). El mundo es estable, pero no avanza.

**Lectura:** la solvencia está haciendo su trabajo — impide exactamente la muerte que medíamos (el 97 % de los colapsos eran en nivel 2 y por oro) — pero descubre que **la economía del oro del nivel 2 es estructuralmente negativa**: la recaudación de 200 pesants no cubre ni un tercio del oro que cobra el nivel 2, y las minas de oro son raras (24 nodos en todo el mapa). Antes el asentamiento subía, no podía pagar y moría; ahora no sube.

**Decidido el 2026-09-26: el oro pasa al nivel 3** (`MANTENIMIENTO.nivelParaOro = 3`). Las opciones que había:
- **Subir `nivelParaOro` de 2 a 3.** El oro se empieza a cobrar donde aparece la base que lo paga: en el gate del nivel 3 hay 200 artesanos, que recaudan casi 4 veces más por cabeza que un pesant. Cuentas en el gate: 500 × 0,004 + 200 × 0,015 = **5 oro/min** de recaudación contra 2 × (1 + 700/500) = **4,8 oro/min** de mantenimiento junto a la capital; lejos de ella, el factor de distancia lo dobla y hace falta mina o comercio — que es el "imperio disperso cuesta más" que ya quería el diseño. Se bajó de 3 a 2 porque *"a nivel 3, que el NPC casi nunca alcanza, el mantenimiento-oro era letra muerta"*; ese motivo eran los bugs que ya están arreglados.
- Bajar `MANTENIMIENTO.oroBase` para el nivel 2 (de 2 a ~0,5).
- Subir `IMPUESTOS.tasaPesants` (0,004 → ~0,012).
- Dejarlo así: solo suben quienes tengan mina de oro o comercio de oro, y el oro pasa a ser recurso estratégico del nivel 2.

### 10.3 Era I con el oro en el nivel 3 (2026-09-26)

`BATCH_SEED=7 BATCH_FACCIONES=12 BATCH_TICKS=50400 BATCH_RUINAS_DIAG=1`, con checkpoints en 25 200 y 50 400
(`simulaciones-batch/checkpoints/`). Primera medición real de la subida manual.

**Ritmo:**

| Nivel | Pidieron | Mediana al pedir | Llegaron | Mediana al llegar | p90 al llegar |
|---|---|---|---|---|---|
| 2 | 41 de 60 (68 %) | 378 ticks (6 h) | 33 (55 %) | **4 561 ticks (3,2 días)** | 8 287 (5,8 días) |
| 3 | 1 | 4 715 (3,3 días) | 1 | 14 795 (1,5 semanas) | — |

- **El nivel 2 llega a los 3,2 días, y casi todo lo pone la obra** (3 días). Los requisitos se cumplen en 6 horas: la
  economía sigue sin frenar nada. Contra el objetivo de ≥1 semana, falta la mitad.
- **Un solo asentamiento en nivel 3 en toda la Era I**: encaja con D49 ("pocos en nivel 3").

**Mundo:** vuelve a expandirse (60 nacidos, ~36 vivos, contra 12 inmóviles con el oro en el nivel 2). **Ya nadie muere
por oro**: los colapsos son por madera (69 %) y piedra (31 %), y la vida mediana antes de caer pasa de 280 minutos a
**4 967 (3,4 días)**.

**Qué frena al final** (36 vivos): **insolvencia en piedra** en 24 — una Cantera rinde 5/min y el mantenimiento del
nivel siguiente cobraría 8,5 —; insolvencia en oro en 6, los que apuntan al nivel 3 (5,4 contra 10,3); sin cupo de
Facción 10; recursos 13; edificios 10.

**Dos distorsiones que invalidan parte de la medición:**
1. **2 172 conquistas y 4 069 campañas en cinco semanas** entre ~36 plazas: unas 60 conquistas por plaza. Cada
   conquista borra la obra de ascenso (regla) y saquea población y edificios, así que el mundo gira en un ping-pong
   bélico constante. Con esa guerra no se puede leer el ritmo de crecimiento en limpio: hay que revisar la
   agresividad del NPC antes de calibrar las Eras siguientes.
2. **Bug de capacidad del almacén**: se suma 300 cada vez que un almacén termina, también al reconstruirlo tras un
   saqueo, y nunca se resta. Plazas con 8 almacenes llegan a 302 800 por recurso (tope esperado: 2 800), y la media
   de madera y piedra sube a ~17 900. Tarea aparte (`avanzarConstruccion`, engine/construction.ts). **El checkpoint
   de 50 400 lleva dentro esas capacidades infladas**: conviene regenerarlo cuando se arregle, antes de medir la Era II
   desde él.

### 10.4 Duración de las corridas, guerra y edificios (2026-09-26)

**Duración** (tiempo real, de los propios archivos de salida):

| Corrida (Era I, 50 400 ticks, 12 Facciones) | Duración | Por tick |
|---|---|---|
| Primera, con subida automática (25/09 22:46 → 26/09 09:41) | 10 h 55 min | 0,78 s |
| Subida manual, oro en nivel 2 (mundo congelado en 12 plazas) | 34 min | 0,04 s |
| Subida manual, oro en nivel 3 (13:59 → 18:54) | 4 h 55 min | 0,35 s |

No son comparables tal cual: entre la primera y la tercera entraron optimizaciones de la otra sesión **y** cambió el
mundo simulado (otras reglas, otro número de plazas y de guerras). La corrida congelada muestra que el coste escala
con la actividad del mundo más que con los ticks. Dentro de la tercera, la primera mitad tardó 1 h 09 y la segunda
3 h 46: el coste crece según crece el mundo.

**La guerra, en el checkpoint del final de la Era I** (36 plazas, 60 héroes, 27 ejércitos):
- **Guarnición: cero escuadrones.** El NPC nunca guarnece (`npcGobernanza.ts` filtra los escuadrones en guarnición,
  pero no pone ninguno), y una plaza sin defensores cae sin combate (Doc 5.12.4).
- **29 de las 36 plazas no tienen ni un héroe dentro.**
- **Los 60 escuadrones son milicia de lanceros de nivel 1**: el NPC recluta siempre `milicia_lanceros`
  (`config.tropaId ?? 'milicia_lanceros'`), aunque hay 8 Barracones, 8 Galerías y 38 Armerías activos.
- **Una sola plaza con muralla.**

Las 2 172 conquistas no son guerra entre ejércitos: son columnas de milicia entrando en plazas vacías. La guerra es
parte de la ecuación (decisión del usuario), pero hoy el NPC no juega la defensa.

**Edificios:** todos se construyen en **3 a 20 minutos** (salvo la Maravilla, 200), y **las mejoras de nivel interno
son instantáneas** — se pagan y suben en el mismo tick (`avanzarMejoras`, engine/construction.ts: *"no hay tiempo de
mejora especificado en el diseño original"*). El usuario ha decidido que hay que cambiar los tiempos de obra; la
tabla completa está en la respuesta del 2026-09-26 y se regenera leyendo `EDIFICIO_CATALOGO`.

## 11. Tiempos de obra y de mejora — DECIDIDO E IMPLEMENTADO (2026-09-26)

**Criterios del usuario:** encajar con D44 (duración de cada Era) y D49 (qué nivel le toca a cada Era); las mejoras
duran más que las construcciones básicas; cuanto más avanzado el edificio, más tiempo.

**Regla de las mejoras** (una sola, sin tabla aparte): la mejora al nivel interno N tarda **la obra base × 2^(N−1)**
— el nivel 2, el doble de la obra; el 3, cuatro veces; el 4, ocho. Hoy son instantáneas.

| Grupo | Edificio | Hoy | Obra propuesta | Mejoras propuestas |
|---|---|---|---|---|
| Supervivencia | Leñera | 3 min | **1 h** | — |
| | Granja | 6 min | **2 h** | N2 4 h · N3 8 h · N4 16 h |
| Base de nivel 1 | Vivienda | 4 min | **4 h** | — |
| | Cantera, Corral | 5-6 min | **4 h** | — |
| | Almacén | 6 min | **6 h** | — |
| | Granero | 6 min | **6 h** | N2 12 h · N3 1 día · N4 2 días |
| | Mina de cobre | 4 min | **6 h** | — |
| | Mina de oro, Mina de estaño | 6-7 min | **8 h** | — |
| | Mercado | 8 min | **8 h** | N2 16 h · N3 32 h |
| Nivel 2: transformación y militar | Fundición, Curtiduría, Armería, Barracón, Galería de tiro, Carpintería, Caballerizas | 6-8 min | **12 h** | N2 1 día · N3 2 días |
| Nivel 3 en adelante | Gran Fundición | 20 min | **2 días** | — |
| Nivel 4 | Palacio | 20 min | **5 días** | — |
| Nivel 5 | Maravilla | 200 min | **4 semanas** | — |
| Recinto | Empalizada | 1 celda/min | **1 celda / 15 min** | muro de piedra 1 / 30 min · adarve 1 / 60 min |

Las obras de ascenso de nivel no cambian (3 días / 1 / 2 / 3 semanas).

**Cuentas contra los objetivos** (2 cuadrillas en paralelo, sin contar esperas de recursos):
- **Nivel 2 (objetivo ≥ 1 semana):** Leñera + Cantera + ~12 Viviendas + 1 Almacén (para que quepa el coste de la
  obra) ≈ 60 horas de cuadrilla → ~1,3 días; más 3 días de obra de ascenso ≈ **4,5-5 días**. Si el batch lo confirma
  por debajo de la semana, la palanca es la obra 1 → 2 (de 3 a 5 días), no los edificios: alargar la base
  arrastra todo el arranque.
- **Nivel 3 (Era II, semanas 5-11):** 26 Viviendas más y los 5 edificios del gate ≈ 190 horas de cuadrilla → ~4
  días; más los lingotes de bronce (estaño por comercio, D54) y 1 semana de obra ≈ **2-3 semanas después del nivel
  2**. Los primeros, a final de la Era I; la meseta, en la Era II.
- **Nivel 4 (Eras III-IV)** y **5 (Era V):** los fija sobre todo la tecnología (D54) más 2 y 3 semanas de obra; los
  tiempos de edificio (recinto, mejoras de 1-2 días, Palacio de 5 días) solo tienen que no ser el cuello.

**Decidido por el usuario el 2026-09-26:**
1. **Gracia de mantenimiento: 1 día** (`MANTENIMIENTO.graciaMinutos = 1440`, antes 60).
2. **La mejora ocupa una de las 2 cuadrillas** mientras dura.
3. **El edificio sigue produciendo** con su nivel actual mientras se mejora.

**Implementado:** tiempos en `EDIFICIO_CATALOGO`, regla de mejoras en `MEJORA_EDIFICIO`, muralla en
`MURALLA.minutosPorCelda`. Motor (`engine/construction.ts`): `Edificio.mejora` con su `completaEn`; `avanzarMejoras`
termina las que llegan y arranca las elegibles mientras quede cuadrilla; `cuadrillasOcupadas` cuenta obras y mejoras;
`aplicarMejoraTerminada` es el único sitio donde sube un nivel interno (la mejora manual antes no ampliaba el
Granero). La mejora manual ahora ARRANCA la mejora y se rechaza sin cuadrilla libre. Una conquista borra la mejora
de los edificios que daña. Muralla: `Recinto.siguienteCeldaEn`.

Tests: los de RITMO derivan sus plazos de las constantes; los de geometría (trazado, murallas, perfiles, gates de
construcción) crecen la ciudad con `acelerarObras`, un fixture explícito que divide los tiempos y los restaura.

### 11.1 Lo que destapó implementarlo (decidido e implementado el 2026-09-26)

1. **La reconstrucción tras un saqueo se queda sin turno.** El edificio dañado vuelve a la cola con la prioridad que
   tenía, y los que nacieron con la ciudad (las 3 Viviendas iniciales) no tienen ninguna: van detrás de cualquier obra
   nueva. Antes la cola se vaciaba en minutos y se notaba poco; ahora, con obras de horas, en un rastreo de 1 300 ticks
   la reconstrucción no llegó a arrancar nunca. Propuesta: que una reconstrucción vaya delante de las obras nuevas.
   **Decidido (usuario): la reconstrucción no ocupa cuadrilla ni espera turno** — arranca aunque las dos estén
   ocupadas (`cuadrillasOcupadas` en engine/construction.ts no cuenta los edificios dañados).
2. **Las mejoras automáticas pueden quedarse con las dos cuadrillas.** La Granja encadena 2 → 3 → 4 (4 + 8 + 16 = 28
   horas) y en el rastreo retuvo una cuadrilla desde el tick 200 hasta pasado el 1 300. Hoy solo coincidió una, pero
   nada impide que dos mejoras paren la construcción del todo durante días. Propuesta: las mejoras AUTOMÁTICAS nunca
   ocupan la última cuadrilla libre (la manual sí puede).
   **Decidido (usuario) tal cual.** Efecto lateral corregido: la construcción prefería mejorar la Granja a construir
   otra en cuanto la mejora era pagable, aunque no pudiera arrancar por falta de cuadrilla; ahora solo la prefiere si
   puede arrancar ya. Tests: `engine/__tests__/cuadrillas.test.ts`.
3. **La política "Líneas de Producción" no puede afectar a la Fundición.** Sus insumos (cobre, estaño) salen de minas
   del mapa general, que es otro espacio de coordenadas que el plano interior de la ciudad, así que siempre cuentan como
   "sin fuente local". Su test comparaba distancias entre los dos espacios (~790) y pasaba por coincidencia. Es de
   diseño: qué debe hacer esa política con edificios cuyos insumos vienen de fuera.
   **Decidido (usuario): se elimina la política** y el código que solo servía a ella (la opción `ampliado` del
   trazado, `sitioEnBarrioLineaProduccion`, `lineasProduccionPriorizadas`/`algunaPoliticaActiva`). La penalización
   por distancia a los insumos (`LINEAS_PRODUCCION`, `factorLineaProduccion`) es otra mecánica y se queda.


### 10.5 Hijos que nacen sin madera (2026-09-26)

En la semana medida con el NPC nuevo, la vida mediana antes de caer bajó a **85 minutos** (82 % en nivel 1, madera en
el 85 %). De las 28 muertes por madera, 27 no tuvieron nunca una Leñera y **26 no tenían ni un punto de bosque dentro
de su zona real**, aunque las 28 tenían un bosque dentro del radio al fundar.

**Causa:** `evaluarViabilidadFundacion` (engine/settlement.ts) mira el bosque libre en un CÍRCULO de radio 30, pero la
zona real se RECORTA contra las de los vecinos (`computeZonaInfluencia`) y el bosque queda fuera. El sitio es
"recomendable" y el asentamiento nace sin forma de sacar madera; muere al acabar la gracia. Ya existía (69 % de muertes
por madera en la Era I anterior); pesa más ahora porque hay más expansión y el mundo es más denso. Es el mismo
"círculo contra zona recortada" que el issue de extractores sospechaba para la piedra (allí no era la causa).

**Arreglado el 2026-09-26 (decisión del usuario):** la viabilidad mira bosque y piedra dentro de la zona con la que
el asentamiento nacería de verdad — el círculo inicial recortado contra los rivales, `zonaInicialDeFundacion`
(engine/zones.ts) —, y decide la Leñera con la misma función con la que la construcción la planta
(`bosqueParaLenera`). Los minerales ya explotados por un vecino no cuentan. `evaluarViabilidadFundacion` acepta la
Facción que funda (sus propios asentamientos no le recortan); sin ella, todo vecino cuenta como rival. Test:
`engine/__tests__/viabilidadZonaReal.test.ts`, con la geometría exacta del bug.

### 11.2 Medición con todo junto (2026-09-26)

Una semana (`BATCH_SEED=7 BATCH_FACCIONES=12 BATCH_TICKS=10080`) con los tiempos nuevos, la gracia de 1 día, el
arreglo del bosque y el NPC nuevo. Tardó 3 minutos.

| | NPC nuevo, obras en minutos | Todo junto |
|---|---|---|
| Nivel 2: mediana al PEDIRLO | ~3-4 h | 3,7 días |
| Nivel 2: mediana al LLEGAR | 3,1 días | **6,6 días (0,95 semanas)** |
| Colapsos en la semana | 12 | **0** |
| Conquistas | 25 | 10 |

- **El nivel 2 cae en la semana que pedía el usuario** (objetivo ≥ 1 semana; la primera medición de todas daba 3
  horas). Ahora los requisitos pesan de verdad (3,7 días) y la obra de ascenso pone el resto.
- **Cero colapsos**: la gracia de 1 día y el arreglo del bosque se notan.
- La expansión arranca justo después del nivel 2 (13 plazas nuevas en las últimas horas de la semana).
- **Guarnición todavía a 0 y solo milicia**: en la primera semana aún no hay Barracón ni Galería (12 h de obra y
  nivel 2), así que no hay cupo ni otras tropas. Es lo esperable en la semana 1.
- **8 plazas sin ninguna defensa** al final: son las conquistadas, que se quedan sin residentes (pendiente de §4.9 del
  documento del NPC). Con menos conquistas pesa menos, pero sigue ahí.

### 11.3 Era I completa (2026-09-26)

Cinco semanas (`BATCH_SEED=7 BATCH_FACCIONES=12 BATCH_TICKS=50400`, checkpoint cada semana) con todo lo anterior
más las cuadrillas (§11.1) y el arreglo de la capacidad del Almacén (la conquista la inflaba sin límite).

| Semana | Vivos | Colapsos acum. | Nivel 1 / 2 / 3 | Conquistas acum. | Guarnición (escuadras) | Plazas sin defensa |
|---|---|---|---|---|---|---|
| 1 | 25 | 0 | 18 / 7 / 0 | 40 | 0 | 11 |
| 2 | 29 | 2 | 20 / 9 / 0 | 276 | 61 | 11 |
| 3 | 33 | 6 | 22 / 11 / 0 | 394 | 86 | 12 |
| 4 | 33 | 9 | 22 / 11 / 0 | 587 | 96 | 12 |
| 5 | 34 | 10 | 22 / 12 / 0 | 732 | 111 | 12 |

- **Nivel 2**: el 52 % de los nacidos llega; mediana al pedirlo 2,8 días y al llegar **5,8 días** (mínimo 5,5). Por
  debajo de la semana que pidió el usuario.
- **Nivel 3: nadie lo pide.** De las 13 plazas de nivel 2 al final: las 3 que tienen los cinco edificios del gate no
  tienen bronce (ni estaño); las 2 que tienen bronce (2 800, almacén lleno, con mina de estaño propia) no tienen
  Curtiduría. Hacen falta cobre, estaño y ganado en la misma plaza, y el NPC no compra estaño ni bronce en el mercado.
  Además, 8 son insolventes en oro al nivel 3 y 2 en piedra. D49 pide "pocos en 3" en la Era I: hoy son cero.
- **Almacén**: madera y piedra medias estables en ~1 900 (antes subían sin límite hasta ~17 900).
- **Guerra**: 732 conquistas y 2 826 campañas en 5 semanas, con 34 plazas vivas; 6 de 12 Facciones en nivel 10 desde
  la semana 2. La guarnición se usa (111 escuadras), pero 12 plazas siguen sin defensa: las conquistadas sin
  residentes. Ninguna muralla.
- Tropas al final: milicia 189, honderos 110, lanceros de mimbre 110, espadachines de cobre 5.

### 11.4 Era I con trueque para crecer y bloque de guerra (2026-09-27)

Misma corrida que §11.3 más el trueque para crecer del NPC (decisión del usuario: el bronce llega comerciando) y el
bloque GUERRA del batch (`scripts/batch/medidorGuerra.ts`).

- **Nivel 3: 4 plazas (9 % de las nacidas)**, pedido a las 2,4 semanas y alcanzado a las 3,4 de mediana. Cuadra con
  D49 ("pocos en 3" en la Era I). 21 trueques de bronce cumplidos. Tres plazas ya piden el nivel 4 a las 4,4 semanas y
  levantan su empalizada, porque el 3→4 aún no está atado a `instituciones_civicas` (D54, pendiente).
- Los trueques de ganado y cobre se proponen mucho y casi todos caducan (639 y 390 caducados contra 6 y 10 cumplidos).
- **Guerra** (5 semanas): 615 conquistas, el 98 % de plazas sin un solo defensor. Solo 13 plazas cambian de dueño, y
  10 de ellas más de 20 veces (una, 204): dos Facciones se pasan las mismas plazas vacías cada ~2 h, y el 96 % de las
  conquistas las recupera un dueño anterior. En los 2 256 asedios contra una plaza defendida, el atacante gana el
  0,4 %: poder mediano del atacante 0,2 veces el del defensor. Mueren ~80 000 soldados (milicia 42 700, lanceros de
  mimbre 22 800, honderos 14 000).
- **Experiencia de Facción**: combate ~182 000, conquista ~12 300, construcción ~7 800. Los campamentos de bandidos
  (5 024 destruidos, uno cada 10 ticks) llevan a una Facción a nivel 8 en 10 horas y a nivel 10 en 1,2 días; cada
  asedio suicida contra una plaza defendida también da experiencia a los dos bandos. 6 Facciones llegan a nivel 10;
  5 no pasan del nivel 2 y no conquistan nada.

### 11.5 Era I con la experiencia nueva y el NPC que ocupa lo que conquista (2026-09-27)

Umbrales de nivel de Facción ×2, guerra a la mitad, solo combate digno (`ratioCombateDigno` 0,5), el NPC solo ataca
lo que puede ganar y se queda en lo que conquista.

- **La guerra cambió de naturaleza**: 11 conquistas en 5 semanas (antes 615), el 73 % en combate; el atacante gana
  el 89 % de sus asedios (poder mediano 1,75 veces el del defensor). Ninguna plaza cae más de 3 veces, ninguna la
  recupera un dueño anterior, cada dueño aguanta 5,8 días de mediana, y no queda ninguna plaza sin defensa. Toda la
  guerra ocurre en las semanas 1-2; en las 3-5, ni una campaña.
- **Pero la expansión se paró**: 13 plazas nacidas (antes 47) y 1 caravana de fundación. La experiencia de Facción
  ya casi solo sale de construir (~30 en la primera semana, mediana 92 al final): los campamentos de bandidos se
  siguen destruyendo (5 025), pero ninguno es un combate digno. 9 de 12 Facciones siguen en nivel 1 al final de la
  Era, y el cap de fundación del nivel 1 es un asentamiento (`CAP_FUNDACION_POR_NIVEL`).
- Solo 8 plazas vivas al final (5 colapsos) y 1 en nivel 3.

### 11.6 Era I con experiencia por crecer en paz (2026-09-27)

Ascenso 30 × nivel, fundación 20, trueque cumplido 2 por lado, bandidos 1 por campamento dividido por el nivel de la
Facción (decisión del usuario).

- **Vuelve la expansión**: 47 plazas nacidas y 53 caravanas de fundación (antes 13 y 1); 33 vivas al final. Nivel 2
  a 6,2 días de mediana, nivel 3 en 5 plazas (11 %) a las 3,6 semanas.
- **Niveles de Facción repartidos**: nivel 2 entre los días 0,7 y 6,9; al final hay Facciones en 2, 4, 5, 6, 7, 9 y
  10 (una sola, al día 34). Experiencia de las 5 semanas: construcción 5 400, crecer 3 300, conquista 4 900,
  combate y bandidos 700.
- **Pero vuelve el ping-pong**: 485 conquistas, el 88 % de plazas vacías; las Facciones 7 y 12 se pasan las mismas
  plazas (205 y 197 veces). Causa: con la expansión, las plazas quedan con 1-2 residentes y el héroe de una casa
  con uno solo no puede mudarse a lo que conquista sin vaciarla. Arreglado en el NPC: no sale de campaña quien no
  podría quedarse (documento del NPC §4.9). Hay ~75 héroes huérfanos, casi todos de Facciones que ya no tienen
  plazas.

### 11.7 Era I con "no conquista lo que no puede ocupar" (2026-09-27)

- **Guerra**: 30 conquistas en 5 semanas (antes 485), el 47 % en plazas vacías; el atacante gana el 80 % de sus
  asedios con combate (poder mediano 1,07 veces el del defensor, combates parejos). Ninguna plaza cae más de 5 veces
  y solo el 13 % la recupera un dueño anterior. 0 plazas sin defensa al final, 93 escuadras en guarnición.
- **Expansión**: 41 plazas nacidas, 35 caravanas de fundación, 31 vivas (10 colapsos).
- **Ritmo**: nivel 2 a 6,0 días de mediana (46 % de las nacidas), nivel 3 en 5 plazas (12 %) a las 3,6 semanas, y 4
  plazas piden el nivel 4 a las 4,6 semanas (pendiente de atarlo a `instituciones_civicas`, D54).
- **Facciones**: la más rápida llega a nivel 8 el día 21,5 y ninguna a 10; las que expanden llegan a nivel 2 entre
  los días 0,7 y 6,9. Experiencia en 5 semanas: construcción 2 100, crecer 5 200, conquista 300, combate y
  bandidos 490.

### 11.8 Era I con protección de un día y reparto de héroes (2026-09-27)

Reglas nuevas desde §11.7: protección del nuevo dueño de un día (Doc 5.12.9), reparto de héroes NPC entre plazas,
no atacar la última plaza de una Facción, artesanos ×1,5 y la nobleza sin inmunidad.

- **Guerra**: 65 conquistas (22 + 35 en las semanas 1-2, luego 3, 2 y 3), el 55 % en plazas vacías. Ningún
  conquistador pierde su plaza antes de un día (mediana 1,3 días); los fundadores, el 86 % en menos de un día: la
  fundación nace con 5 héroes bot nuevos y sin tropa, y una campaña vecina la toma a los ~50 ticks.
- **Expansión**: 49 plazas nacidas, 32 vivas (17 colapsos). Tres Facciones (8, 10, 12) pierden su última plaza por
  colapso, sin ganador: sus 35 héroes se quedan en una plaza que ya no existe.
- **Ritmo**: nivel 2 a 5,9 días de mediana (57 %), nivel 3 en 5 plazas (10 %) a las 3,55 semanas. Bloqueo principal al
  final: insolvencia en piedra (18 de 32).
- **Facciones**: la más rápida llega a nivel 7; ninguna a 8. 79 escuadras en guarnición y 0 plazas sin defensa.
- **Reparto**: iguala las Facciones con sitio (1: 2/2/2/2/2/1/1/1/1), pero no las que tienen más héroes que casas
  (4: 24 y 10 residentes con 12 casas; 6: 20 y 5). La acogida y la anexión no respetan el tope de casas.
- **Trueques para crecer**: 867 peticiones de ganado caducadas contra 3 cumplidas.
