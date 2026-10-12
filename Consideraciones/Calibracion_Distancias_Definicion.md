# Calibración de distancias y velocidades — definición

> **Estado (2026-10-12): en diseño, cero código, canon sin tocar.** El canon (`Docs/Game/`) se reescribe al implementar
> (decisión del usuario, 2026-10-11). Página de trabajo con el simulador y las tablas:
> https://claude.ai/artifact/9MViDfr2yzTsqaRgUCXbMU
>
> - **Cerrado:** sensación de movimiento, velocidad de la tropa ligera, separación mínima entre plazas, tamaño de zonas y
>   ciudad, la tabla de valores de §4 y bosques mezclados (§6).
> - **Cerrado también:** focos alrededor de los campamentos, fijos a 600 de separación; estaño solo fuera de los focos,
>   agrupado en distritos mineros (§5).
> - **Cerrado también:** contrato de escala con el mundo regional de Codex: **factor 12** unidades Unity por unidad
>   estratégica, mapa estratégico de **2.667 × 2.000** (§2.1). El número de plazas o de jugadores no es prioritario.
> - **Abierto:** dónde va el oro (§5.1, O1) y la capacidad de Leñeras de un bosque grande (§6, O2).
> - **Propuesta pendiente:** migración total del worldgen al mundo regional, borrando el viejo y las partidas (§11).
> - **Acordada (2026-10-12):** reconstrucción con soluciones finales (`Reconstruccion_Definicion.md`): mundo reactivo
>   sin tick, servidor en Rust, artefacto de mundo, Unity como cliente final. Resuelve §3.1 y absorbe §11.
>
> **Para Codex:** este documento se actualiza con cada acuerdo. Responde a
> `Conquest_prototype/Docs/Worldgen_Recap_Divergencias_Calibracion_2026-10-12.md` (correcciones en §9) y lo que toca
> evaluar del lado regional y Unity está en §10.

## 0. Razón de ser

En el playtest (a 2 ticks/s, sin Unity) el movimiento parecía bien, pero las distancias no. A 1:1, que es como se juega
con Unity, ir a la plaza vecina cuesta 8,5 minutos de tropa ligera: **nadie juega si tiene que caminar 8,5 minutos para
atacar a la plaza más cercana**. El mundo no está lo bastante lleno para que vivir en él sea interesante.

El diagnóstico es de **densidad, no de escala**. Cruzar el mapa en una hora u hora y media está bien; lo que sobra es el
espacio entre plazas. Por eso no vale escalar todo con la velocidad (×10 dejaría cruzar el mapa en 10 minutos): la
velocidad y el mapa se quedan, y lo que cambia es cuántas cosas hay por unidad de mapa.

## 1. La sensación de movimiento: una fórmula

```
velocidad en pantalla (px/s) = (velocidad en unidades/tick ÷ segundos reales por tick) × (px de pantalla ÷ ancho visible)
```

Medida con el simulador: **~29 px/s sobre un lienzo de ~866 px, o sea, cruzar el ancho de la pantalla en ~30 s
(≈ 3,3 % del ancho por segundo)**. Es la cifra que se conserva; los px/s dependen del monitor, el porcentaje no.

Con los valores elegidos (ligera 30, 1 tick = 60 s, ancho visible 15): 30/60 × 866/15 ≈ 29 px/s.

Consecuencias en el cliente:

- El **zoom de juego** muestra unas **15 unidades de mapa de ancho** (8,4 de alto en 16:9). Hoy el zoom del cliente va de
  1 a 4 (`ZOOM_MAX`, `geometriaVista.ts`): hace falta ~133.
- El terreno se dibuja en un canvas de 900 px que se escala por CSS (`pantallaMapa.ts`, nota ponytail): a ese zoom hay que
  dibujarlo por tramos a resolución alta.
- A ese zoom una ciudad (0,6-1,0 de mapa tras §3) se ve como un pueblo dentro de la pantalla.

## 2. Valores elegidos por el usuario (2026-10-12)

| | Valor |
|---|---|
| Velocidad de la tropa ligera | **30** unidades/tick (antes 20) |
| Reloj | 1 tick = 60 s (1:1) |
| Separación mínima entre dos plazas | **30** (1 min de centro a centro) |
| Ancho visible del zoom de juego | **15** |
| Zona de nivel 2 / nivel 1 | **15 / 10** (antes 90 / 60) |
| Ciudad (casco + afueras) en el mapa | **−80 %** |

Factores que salen de ahí: velocidades **×1,5**, zonas **×1/6**, separación y radios de interacción **×0,2**, ciudad
**×0,2**.

Tiempos resultantes a velocidad ligera 30 (0,5 unidades/s), en línea recta:

| Trayecto | Distancia | Tiempo |
|---|---|---|
| Plaza vecina más cercana, de centro a centro | 30 | 1 min |
| Entre los bordes de dos zonas recién fundadas (6 + 6) | 18 | 36 s |
| Entre los bordes de una zona de nivel 2 y una de nivel 1 (15 + 10) | 5 | 10 s |
| Del centro al borde de una zona de nivel 2 (su **radio**) | 15 | 30 s |
| Cruzar una zona de nivel 2 (su diámetro) | 30 | 1 min |
| Carro de acarreo hasta el borde de su zona de nivel 2 (buey 24) | 15 | 38 s |
| Del campamento al borde de su foco | 150 | 5 min |
| Hueco mínimo entre dos focos vecinos | 300 | 10 min |
| Entre los centros de dos focos vecinos | 600 | 20 min |
| Cruzar el mapa de lado / en diagonal (§2.1) | 2.667 / 3.333 | 89 / 111 min |

Las zonas de nivel 5 (radio 30) de dos plazas a 30 se recortan mucho al crecer: está permitido (fronteras duras, Doc
1.2), pero obliga a contar los recursos accesibles sobre la zona recortada, no sobre el círculo.

### 2.1 Contrato de escala con el mundo regional (2026-10-12)

El mundo regional de Codex (`generarRegionalVisor`, 32.000 × 24.000 unidades Unity, 16 áreas) **mantiene su tamaño en
Unity**. Decisión del usuario: el mapa estratégico es el **doble de lado** que el que daba el factor 24 (1.333 × 1.000),
para que las regiones sean más grandes y quepa más actividad dentro.

| | Valor |
|---|---|
| Factor Unity / estratégico | **12** (1 unidad estratégica = 12 unidades Unity) |
| Mapa estratégico | **2.667 × 2.000** (el regional ÷ 12) |
| Cruzar el mapa a velocidad 30 | 89 min de lado, 111 en diagonal |

**No es un ×2 de todo.** Todo lo que se mide en unidades estratégicas se queda como en §2 y §4: velocidades, separación,
zonas, visión, radios de interacción, foco 150, frontera 250, separación entre campamentos 600, mantenimiento y
comercio por distancia, víveres y batallas. Lo que cambia:

1. **El tamaño macro.** Cruzar el mapa se duplica frente al factor 24 (44 → 89 min), y el área se cuadruplica. Es casi
   el tamaño del mapa de 2.000 con el que se calibró (un tercio más ancho), así que la calibración se aplica casi tal
   cual.
2. **La geografía queda el doble de grande frente a lo que hay encima.** Una cordillera de 2.000 Unity son 167
   estratégicas, 5,5 min a pie. Caben más focos y más actividad por región.
3. **Lo que depende de la tierra firme total** se recuenta en proporción a ella: número de campamentos a 600, totales
   de estaño, hierro y demás nodos, el grafo de navegación (~333 × 250 nodos a paso 8) y las celdas de niebla (~213.000
   a 5, ~27 KB por Facción). Primera medida (§5): **66 % de tierra firme, 10-12 campamentos**.
4. **En Unity, lo que viene del servidor se ve a la mitad** que con el factor 24 (tabla en §10).

**El número de plazas o de jugadores no es prioritario** (decisión del usuario): la capacidad del mundo sale de las
dimensiones, no al revés. Las capacidades de §5 son geométricas y sirven para medir, no como objetivo.

## 3. Tres cambios de código que salen de aquí

1. **Movimiento sin tick (obligatorio).** Con 24-42 unidades por tick y radios de 2-10, ir a la plaza
   vecina es un solo tick: la detección por tick (puerta, encuentro, inspección, bandidos, reabastecimiento) se salta
   cosas, y perseguir, escapar o interceptar se decide en uno o dos ticks. Pasar todo el servidor a 1 tick/s se
   descartó: las ~54 tasas `*PorMinuto` del motor se aplican por tick sin escalar y el coste sería ×60 para todo el
   mundo.

   **Solución final (2026-10-12): el mundo reactivo** de la reconstrucción (`Reconstruccion_Definicion.md`;
   `Docs/Arquitectura/13_Mundo_Reactivo_Rust.md`). Sin tick: planes de movimiento con el instante de paso por cada
   vértice, `posición(t)` exacta y encuentros como eventos (cuadrática por par de tramos). Una orden se aplica desde la
   posición real del instante en que se da, y ninguna detección depende de un paso de tiempo. Se descartaron los
   subpasos dentro del tick (arreglaban la detección, no el retraso de hasta 60 s en empezar a andar) y los dos relojes
   en TypeScript: el usuario no quiere soluciones intermedias.
2. **Desacoplar la ciudad de la zona.** `radioUrbanoDe` (`engine/trazado.ts`) devuelve `radioPotencial` tal cual. Con
   zonas de 10-30 todas las ciudades se quedarían en el suelo del trazado (96-121 locales) y dejarían de crecer.
   Pasa a ser `radioPotencial × 6`: el trazado urbano no cambia en locales. Y `ESCALA.unidadesLocalesPorUnidadMapa`
   pasa de 40 a 200: la ciudad mide 0,6-1,0 de mapa (el −80 %).
3. **La escolta frena** (bug ya decidido en esta revisión): las caravanas adjuntas entran en el mínimo de velocidad,
   pero `velocidadDeEjercito(ejercito)` se llama sin ellas en el movimiento (`engine/ejercitos.ts:1744`). Al
   arreglarlo, la Caravana de Fundación (sin carros, velocidad 0) no puede entrar en el mínimo.

## 4. Tabla de valores (actual → propuesto)

Aprobada por el usuario el 2026-10-12, salvo lo marcado en §5 y §6.

### 4.1 Velocidades (×1,5)

| Valor | Actual | Propuesto |
|---|---|---|
| Montada | 28 | 42 |
| Caravana con caballo | 24 | 36 |
| Héroe solo (`MOVIMIENTO.velocidadJugador`) | 22 | 33 |
| Ligera / Aedas itinerantes | 20 | 30 |
| Caravana con camello | 19 | 28 |
| Media / buey (comercial base, acarreo) | 16 | 24 |
| Pesada | 12 | 18 |
| Caravana de Fundación que vuelve sola | 10 | 15 |

Las reglas de Doc 5.12.5 se mantienen: ligera (30) > buey (24); solo la montada (42) alcanza a la de caballo (36).

### 4.2 Zona y ciudad

| Valor | Actual | Propuesto | Motivo |
|---|---|---|---|
| `ESCALA.unidadesLocalesPorUnidadMapa` | 40 | 200 | ciudad −80 % |
| `ZONA_INFLUENCIA.radioInicial` | 30 | 6 | ×1/6 da 5; 6 mantiene «la ciudad nunca pasa de una décima de su provincia» |
| `ZONA_INFLUENCIA.radioMaximoPorNivel` | 60/90/120/150/180 | 10/15/20/25/30 | ×1/6 |
| `ZONA_INFLUENCIA.crecimientoPorEdificioCompletado` | 5 | 1 | ×1/6 |
| `radioUrbanoDe` | = radioPotencial | radioPotencial × 6 | §3.2 |
| `VISION.margenAsentamiento` | 60 | 10 | ×1/6 |
| `LINEAS_PRODUCCION` | 25/150/90 | se elimina | decisión previa del usuario |

### 4.3 Separación entre plazas (×0,2)

| Valor | Actual | Propuesto | Motivo |
|---|---|---|---|
| Separación mínima entre plazas | no existe | **30** | regla nueva; hoy sale de «la zona inicial no solapa otra zona» |
| `FUNDACION.distanciaMinimaAparicion` | 200 | 40 | |
| `MERCENARIOS.radioExclusionFundar` | 100 | 20 | protección + zona inicial + margen |
| `MERCENARIOS.radioProteccion` | 60 | 10 | |
| `MERCENARIOS.separacion` | 900 | 600 | 8-10 focos (§5) |
| `MERCENARIOS.margenBorde` | 150 | 60 | |
| `MERCENARIOS.margenBosque` / `pegadoAlBorde` | 100 / 10 | 20 / 2 | |
| `CAMPAMENTOS_BANDIDOS.anillo` | 150-250 | 30-50 | fuera de toda zona (§5) |
| Bandidos de cada plaza | bosque más cercano fuera de la zona | a ≥ 10 del borde de la zona | decisión de esta revisión: más separados |
| `CAMPAMENTOS_BANDIDOS.radioAtaqueCaravana` | 40 | 7 | |
| `ALIJOS.bandas` | 100-200 / 200-300 / 300-450 | 20-40 / 40-90 / 250-300 | las dos primeras dentro del foco; la tercera en la tierra lejana, junto a las vetas (§5.1) |
| `MANTENIMIENTO.escalaDistancia` | 400 | 80 | reino cómodo de ~5 provincias |
| `COMISION.distanciaParaBonusMax` | 600 | 120 | |
| `ASIGNACION_CARAVANA.distanciaReferencia` | 600 | 120 | |
| `INTEL.mirada.oroPorUnidad` | 0,1 | 0,5 | mismo precio por viaje |
| `PERSECUCION.umbralRecalculo` | 30 | 6 | |

### 4.4 Visión y reacción

| Valor | Actual | Propuesto | Motivo |
|---|---|---|---|
| `VISION.ejercito` | 150 | 30 | ve el anillo de vecinas con 1 min de aviso |
| `VISION.jugadorSolo` | 80 | 15 | ~el ancho de la pantalla |
| `VISION.caravanaGrande.radio` | 300 | 60 | |
| `INTEL.mirada.radio` | 150 | 30 | «el radio de una columna» |
| `MOVIMIENTO.radioInspeccion` | 40 | 8 | |
| `LOGISTICA.radioReabastecimiento` | 60 | 10 | sigue igual al margen de la plaza |
| `LOGISTICA.radioEncuentro` (distancia de ataque) | 15 | 3 | |
| `MOVIMIENTO.radioPuerta` | 10 | 2 | justo fuera de la muralla |
| Detección y movimiento | por tick de 1 min | planes y eventos del núcleo reactivo, sin tick | §3.1 |

### 4.5 Mundo y worldgen

| Valor | Actual | Propuesto | Motivo |
|---|---|---|---|
| Mapa | 2000 × 2000 | 2.667 × 2.000 | el regional a factor 12 (§2.1) |
| `RECURSO_RAREZA.espacioMinimo` | 20 / 40 / 120 | 4 / 7 / 20 | caben en zonas de 10-30 |
| `RECURSO_RAREZA.cantidadBase` | 240 / 80 / 24 | denso en los focos (×20-36 dentro), escaso fuera; los raros, en proporción a la tierra firme | conservar nodos por zona; a medir con worldgen |
| `LIVESTOCK.espacioMinimo` | 30 | 5 | |
| `BOSQUE` | radio 45-120, 170 | mezcla (§6) | |
| `NAVEGACION.espaciadoMin` / `nodosPorLado` | 45 / 160 | 8 / ~333 | el A* tiene que distinguir plazas a 30 (paso 8 sobre 2.667) |
| `NAVEGACION.separacionVadosMin` / `vadosPorLado` | 300 / 16 | 100 / 20 | un vado cada 300 son 10 min de rodeo |
| `NAVEGACION.radioPasoCiudadMin` | 60 | 10 | |
| `EXPLORACION.tamanoCelda` | 25 | 5 | niebla legible a esta escala (~27 KB por Facción sobre 2.667 × 2.000) |
| `ZONA_INFLUENCIA.pasoFusionContorno` | 5 | 1 | solo dibujo |
| Tamaño máximo de un elemento nuevo (Guerra logística) | 1 de mapa | 0,2 de mapa | = 40 locales, igual que antes |

**No cambia:** víveres (duran los mismos minutos), batallas (15/30 min), plazos en minutos.

## 5. Focos de actividad: los crean los campamentos

Decisión del usuario: **los focos nacen orgánicamente de los campamentos de mercenarios.** Toda Facción sin plaza vive
en un campamento y funda andando desde él; los bots sin plaza también. Con la exclusión a 20 las plazas se apiñan
alrededor, y las plazas de nivel 2+ lanzan sus Caravanas de Fundación desde ahí: el foco crece hacia fuera y puede
llegar a tocar otro.

Lo que cambia sobre los campamentos:

- **Sus funciones no cambian** (residencia, préstamo, reclutar, mercado, taberna, refundar, ración).
- **Radio de foco ~150** (5 min a velocidad 30): hasta donde camina un fundador sin perder el campamento de vista.
  Capacidad **geométrica**, no económica ni de jugadores: a separación mínima 30 (hexágono de ~779 u²) caben ~90
  plazas en el círculo del foco; descontando pendiente, bosque, agua, exclusiones y recortes de frontera, del orden de
  la mitad. Sirve para medir, no como objetivo (§2.1).
- **El worldgen tiene que saber dónde estarán los focos** antes de repartir la economía. El orden del pipeline se
  invierte (coincide con la propuesta de Codex, recap §8.3):

  ```text
  geografía y accesibilidad
    → emplazamientos aptos de campamentos (fértiles, conectados)
    → campos de distancia y anillos (§5.1)
    → bosque explotable y recursos
    → vetas de estaño y su contenido (bandidos, alijos)
    → validación de fundación, propiedad y suministro
  ```

  Las posiciones salen de datos versionados y reproducibles; nada se mueve a mano después.
- **Anillo de bandidos (30-50) dentro del foco**: es contenido para los recién llegados (D42), pero ahí es donde se
  fundan las plazas. Tiene que evitar las zonas (comprobar si hoy lo hace).
- **Alijos**: los cercanos los absorben las plazas enseguida (son la primera exploración); el lejano queda en los
  huecos entre focos, que es donde explorar tiene sentido.

**Cuántos campamentos: fijos a 600** (propuesto por el usuario, 2026-10-12). Siguen existiendo todos desde la creación
del mundo (Doc 1.9b), pero con `separacion` 600 y `margenBorde` 60. Medido con `colocarCampamentosIniciales` sobre tres
seeds del mapa de 2.000: **10, 9 y 8 campamentos** (con 900 salían 5, 5 y 4). Entre dos focos vecinos quedan como
mínimo 300 de tierra (600 − 2 × 150), unos 10 minutos a pie; entre sus centros, 20. Los campamentos latentes (abrir
focos nuevos cuando los abiertos se llenan) quedan como alternativa si hiciera falta.

**Medido sobre el mundo regional** (2026-10-12, paquete `campaign-coasts-v1-seed42` de Codex, a factor 12). Es una
**emulación**: la regla de `colocarCampamentosIniciales` (puntos al azar en tierra firme, sin cima, pegados a un bosque
a menos de 20, separación 600, margen 60) sobre la máscara de tierra del paquete, con tres semillas de colocación. No es
el pipeline definitivo de §5.

| | Semilla 1 | Semilla 7 | Semilla 42 |
|---|---|---|---|
| Campamentos | 12 | 10 | 11 |
| Tierra en focos (≤ 150) | 20 % | 17 % | 17 % |
| Tierra en frontera (150-250) | 30 % | 25 % | 26 % |
| Tierra lejana (> 250) | 50 % | 58 % | 57 % |
| Estaño del regional dentro de un foco | 14 de 56 | 7 de 56 | 14 de 56 |
| Oro del regional dentro de un foco | 5 de 48 | 6 de 48 | 7 de 48 |

La tierra firme es el **66 %** del rectángulo: ~3,54 millones de u² estratégicas, casi lo mismo que la tierra habitable
del mapa de 2.000 con el que se calibró (3,64 millones). La mitad o más de la tierra queda lejos de todo campamento: es
el sitio de los distritos mineros de §5.1. El estaño del regional cae más en los focos que el de BronzeAge porque Codex
lo coloca en enclaves pensados para fundar al lado (bosque y cantera cerca); con la exclusión de §5.1 esos enclaves se
mueven a la tierra lejana.

### 5.1 Tres anillos de tierra, por distancia al campamento más cercano

La tierra lejana **no es yerma** (decisión del usuario): tiene que darle algo que buscar al explorador. El worldgen
reparte por anillos:

| Anillo | Distancia al campamento | Qué hay |
|---|---|---|
| **Foco** | 0-150 | Tierra fértil: bosques medianos y pequeños densos, piedra, fauna, algo de cobre y hierro. **Sin estaño.** |
| **Frontera** | 150-250 | Bosques grandes compartidos (§6), cobre, hierro, piedra y algo de oro. Es donde se expande la segunda generación de plazas. |
| **Tierra lejana** | > 250 | **Distritos mineros**: vetas de estaño (y oro) en colina o montaña, guardadas por bandidos. Alijos lejanos y fauna de estepa. |

**El estaño solo fuera de los focos**: regla dura del worldgen con un parámetro propio, **`exclusionEstanoCampamento` =
200** (el foco de 150 más un margen de 50, para que ninguna plaza del borde del foco lo tenga dentro de su zona). Hoy
ya cae casi todo fuera, porque solo admite colina o montaña y los campamentos buscan bosque: en el mapa de 2.000 con
600 de separación, 2, 10 y 5 de los 64 yacimientos de cada seed quedan dentro de un foco de 150. Lo que falta es
**agruparlo**.

**Distritos mineros (vetas)**:

- Cada veta junta 3-6 nodos de estaño (y 0-2 de oro) en un radio de ~20-30. Una plaza de nivel 2-3 (zona 15-20)
  plantada en el centro cubre 2-3 de ellos: compensa fundar una **colonia minera** allí. Varios nodos del mismo tipo
  dentro de una zona son válidos (decisión del usuario).
- **Total de estaño en proporción a la tierra firme** (referencia: 64 en el mapa de 2.000, con `ESTANO_EXTRA`), solo
  reagrupado y sin supernodos. Las cuotas por veta se reparten para **sumar exactamente el total**: no puede salir una
  cantidad variable por accidente. Orden de magnitud: ~1,5 vetas por campamento, de unos 5 nodos.
- Pueden convivir con **fuentes aisladas por región** (como en el regional de Codex), siempre fuera de la exclusión.
- Cada veta tiene su **campamento de bandidos de nivel 2-3** (sin plaza dueña) y el alijo lejano cerca.
- El bronce pasa a depender de **expandirse, comerciar o pelear**: las caravanas de estaño cruzan tierra de nadie, que
  es donde la guerra de caravanas es de tipo A (Guerra logística §1). Así era en la Edad del Bronce: el estaño viajaba
  lejos.

**Riesgos a medir**:

- **Los bots tienen que saber ir a buscarlo** (fundar en una veta o comerciar). La Era I ya se atascó una vez por falta de
  estaño (por eso existe `ESTANO_EXTRA`).
- **Mantenimiento de la colonia**: con `escalaDistancia` 80, una colonia a 300 de la capital paga ×4,75. Encarece mucho
  la expansión minera; puede ser lo que se quiere (el estaño se compra) o pedir subir la escala a 120-150.

**O1 — El oro.** ¿Se va también a la tierra lejana, o se reparte entre frontera y distritos? *Propuesta:* mitad y mitad;
el estaño es el que obliga a salir.

## 6. Bosques: pocos grandes y muchos medianos o pequeños

Decisión del usuario: **mezcla**.

- **Grandes** (radio ~40-80, del orden de 10-20 en el mapa): comarcas compartidas por varias plazas, entre focos o en
  su borde. Dan disputa por la madera.
- **Medianos y pequeños** (radio ~4-20, muchos): dentro de los focos, para que cada plaza tenga madera cerca.

**Bosque continuo de Codex.** El regional genera el bosque como campo continuo de densidad, no como discos. Propuesta
compatible (recap §8.4): dentro del campo visual, **masas explotables con ID y capacidad de Leñeras**, cuya propiedad se
resuelve por las zonas reales; las pequeñas en los focos y las comarcas en la frontera. El aspecto no muestra los
límites de esas masas.

**O2 — Leñeras por bosque.** Hoy 1-3 según el radio (`LENERA_POR_BOSQUE`, umbrales 45/65). Para los pequeños,
umbrales ~8/11; para los grandes compartidos, 3 se queda corto: ¿por área, sin tope fijo? Bosque grande no equivale a
madera ilimitada para todos.

## 7. Canon a reescribir al implementar

Doc 1.0a (escala, provincias que caben, reino cómodo), 1.2 (zona), 1.3 (aparición), 1.8 (exclusión), 1.9 (bandidos),
1.9b (campamentos, alijos, protección, focos), 3.8 (comercio), 4.5 (mantenimiento), 5.12.3 (inspección, encuentro),
5.12.5 (velocidades), 5.12.7 (visión), 5.13 (reabastecimiento) y 5.13.1 (el radio operativo: un cuarto del mapa de
2.667 ida y vuelta son ~1.333 de recorrido, 44 min a 30; los 350 de víveres siguen sobrando). Además,
las cifras del carro de acarreo en `Guerra_Logistica_Definicion.md` §2.3.

## 8. Plan

0. Contrato de escala (§2.1) escrito como contrato entre BronzeAge y Unity: factor, unidades y reloj. Medir la tierra
   firme del regional y el ancho visible real de la cámara de Unity.
1. Subpasos de movimiento y detección (§3.1), con la escolta (§3.3). Batch: sin regresión en el movimiento actual.
2. Velocidades, radios de interacción y visión (§4.1, §4.4).
3. Zona, ciudad y `ESCALA` (§3.2, §4.2).
4. Worldgen: campamentos a 600, anillos de tierra, vetas de estaño con sus bandidos, recursos y bosques (§4.5, §5,
   §6). Subir la versión del worldgen; las partidas actuales no sirven. Los bots tienen que aprender a buscar estaño.
5. Separación entre plazas y el resto de §4.3.
6. Cliente: zoom de juego ~133 y terreno por tramos (§1).
7. Medir: worldgen + batch con los bots previstos; distancias reales (vecina, mercenarios, bandidos) y la prueba de zoom.
8. Canon (§7).

## 9. Correcciones tras la revisión de Codex (2026-10-12)

Codex revisó este documento en `Conquest_prototype/Docs/Worldgen_Recap_Divergencias_Calibracion_2026-10-12.md` §9.
Las siete observaciones son correctas y quedan resueltas así:

| # | Observación | Resuelto en |
|---|---|---|
| 1 | Quedaba «900 entre focos, 30 min»; con 600 son 20 min entre centros y 10 min de hueco | §2 (tabla de tiempos) |
| 2 | «Recorrer una zona de nivel 2 en 30 s» es su radio; el diámetro tarda 1 min | §2 (tabla de tiempos) |
| 3 | Con zonas iniciales 6 y centros a 30 hay 18 entre bordes (36 s); los «50 s» eran con zonas 15 + 10 | §2 (se quitan los 50 s) |
| 4 | Exclusión del estaño ambigua entre 150 y 200 | §5.1: parámetro explícito `exclusionEstanoCampamento` 200 |
| 5 | Las vetas tienen que sumar exactamente el total | §5.1 |
| 6 | 90 plazas por foco es geometría, no economía; las zonas de nivel 5 a 30 se recortan | §2 y §5 |
| 7 | La capacidad no es número de jugadores | §2.1: el número de plazas o jugadores no es prioritario |

## 10. Lo que toca evaluar del lado regional y de Unity

Para Codex, a partir de §2.1 (factor 12). Nada de esto está aplicado; es lo que hay que medir o decidir allí.

**Lo que ya aceptó el usuario en la prueba de Unity, traducido a factor 12.** La sensación objetivo es la de §1: cruzar
la pantalla en ~30 s y el radio de una zona de nivel 2 en 30 s.

| En Unity | Prueba aceptada | Con factor 12 |
|---|---|---|
| Velocidad del héroe (ligera) | 12 u/s | **6 u/s** |
| Zona inicial / N1 / N2 / N3 / N4 / N5 | 90 / 180 / 270 / 360 / 450 / 540 | **72 / 120 / 180 / 240 / 300 / 360** |
| Separación mínima entre plazas | 180 (auditoría de arranques) | **360** |
| Ancho visible de la cámara para la misma sensación | `viewSize` 320 | **~180** (medido, no el parámetro) |
| Radio de una ciudad (0,6-1,0 estratégicas) | — | **7-12** |
| Foco / frontera / separación entre campamentos | — | **1.800 / 3.000 / 7.200** |
| Radio de una veta de estaño (20-30) | concentraciones amplias | **240-360** |
| Puerta (2) / encuentro (3) | 18 de prueba | **24 / 36** |

**A evaluar en el regional:**

1. **La máscara de tierra firme**: primera emulación hecha (§5: 66 % de tierra, 10-12 campamentos a 7.200 Unity). Falta
   repetirla con el pipeline de §5 y fijar los totales de nodos.
2. **Campamentos antes que recursos** (pipeline de §5) y los tres anillos de §5.1 sobre la geografía regional, sin
   perder la identidad de cada región. Una región puede tener varios focos de reinos distintos, o ninguno y atractivo
   minero.
3. **Estaño en vetas compactas.** La separación mínima actual del regional entre estaños (≥ 1.100 Unity, pensada para
   que no haya dos en una zona de nivel 5) **choca con las vetas** y tiene que quitarse, junto con el diagnóstico que
   suponía ese aislamiento.
4. **Hierro**: falta en el catálogo regional exportado y la calibración lo necesita en foco y frontera.
5. **Bosque**: masas explotables con ID y capacidad dentro del campo continuo (§6).
6. **Símbolo de la ciudad**: con radio estratégico de 7-12 Unity, la miniatura necesita su propio perfil de escala; las
   copas forestales llegan a 98 de diámetro y la ciudad no puede medirse contra ellas.
7. **Cámara**: medir el ancho visible real y ajustarlo a ~180 Unity para la sensación de §1; con 320 el héroe tardaría
   53 s en cruzar la pantalla, que es justo el problema de partida.
8. **El relieve no cambia**: montañas, pasos, ríos, lagos y costas se quedan en Unity como están; a factor 12 miden el
   doble en estratégicas que a factor 24, que es lo que se busca.

## 11. Migración total al mundo regional (propuesta, 2026-10-12)

> **Estado: propuesta, pendiente de acordar.** Decisión del usuario: migración **total**, sin convivencia de los dos
> generadores ni compatibilidad con partidas viejas. Se borra el código antiguo y todas las partidas.
>
> **Absorbida por `Reconstruccion_Definicion.md`** (acordada el 2026-10-12): el generador regional pasa a ser la
> herramienta que produce el artefacto de mundo del núcleo nuevo (paso 3 de su orden), no un generador del servidor
> actual. Lo de worldgen de abajo (fases 1, 3 y 4, §11.4 y §11.5) se mantiene como detalle de ese paso; el resto lo
> sustituye el orden de la reconstrucción.

### 11.1 Punto de partida

| | BronzeAge (`main`) | Regional de Codex |
|---|---|---|
| Dónde | `src/worldgen/`, ~1.700 líneas, en `main` | worktree `codex/worldgen-relieve`, **sin commitear** (71 archivos nuevos, 40 modificados) sobre `92220a3`, **235 commits por detrás de `main`** |
| Entrada | `generarMapa({ ancho, alto, seed, region? })` → `MapaGenerado` | `generarRegionalVisor(seed, lado, alto, densidad)`, en `scripts/`, no en `src/` |
| Mundo | 2.000 × 2.000, 8 perfiles (`REGIONES`: greciaContinental, anatolia, egeo…) o libre | 32.000 × 24.000 Unity, composición mediterránea fija de 16 áreas |
| Bosque | discos con ID y capacidad de Leñeras | campo continuo, sin fuentes explotables |
| Recursos | piedra, cobre, estaño, oro, fauna, hierro | sin hierro; estaño en enclaves a ≥ 1.100 Unity |
| Campamentos | `colocarCampamentosIniciales`, en el motor, después del mapa | no existen |
| Salida extra | — | paquete Unity por sectores (alturas, capas, costas), 63 MB por seed |

Lo que consume el mapa en BronzeAge: la fachada `Mapa` (`world/mapa.ts`, ~30 métodos: elevación, terreno, bioma, coste,
transitabilidad, ríos, fertilidad, nodos con stock y regeneración, bosques con capacidad de Leñeras), el grafo de
navegación, las rutas y la red de caminos; el motor (bandidos, alijos, mercenarios, ejércitos, movimiento, población,
combate, simulación), los bots, la sesión, la persistencia (el mapa no se guarda: se regenera de la seed y se rechaza
si cambia `WORLDGEN_VERSION`), la ruta `/mapa` y los dos clientes 2D (admin `cliente/src/ui/canvas.ts` y jugador
`render.ts`).

### 11.2 Principios

1. **Un solo generador**, en `src/worldgen/`, con una sola entrada: `generarMundo(seed)`. Sin perfiles ni parámetro
   `region`: la composición mediterránea es fija y la seed varía dentro de ella.
2. **La unidad que manda es la estratégica** (§2.1). La geografía de Codex puede seguir calculándose en sus unidades,
   pero el `MapaGenerado` que sale del generador está en estratégicas (÷ 12), y toda la economía del mundo (campamentos,
   anillos, recursos, vetas, bosque explotable) se coloca ya en estratégicas con las constantes de §4. **Una sola
   conversión, en un solo sitio.** Unity recibe su paquete a ×12.
3. **El servidor decide, Unity representa** (BA-003). Lo que Unity recibe (alturas, capas, costas) es un paquete
   versionado por hash derivado del mismo mundo; hay que reescribir BA-003 para que lo diga.
4. **Corte único**: todo se hace en una rama `mundo-regional` sacada de `main` y se fusiona de una vez. Hasta la fusión,
   `main` sigue con el mundo viejo; tras ella, el viejo ya no existe.

### 11.3 Fases

| # | Fase | Quién (propuesta) | Depende de |
|---|---|---|---|
| 0 | Acordar §11 y repartir el trabajo. Reescribir BA-003. | usuario | — |
| 1 | **Traer el regional a `main`**: rebasar `codex/worldgen-relieve` sobre `main` actual, commitear, separar lo que es geografía (`src/worldgen/*Regional*`, drenaje, costas) de visor, auditorías y exportación. Pasar `generarRegionalVisor` de `scripts/` a `src/worldgen/` como etapa de geografía. | Codex | 0 |
| 2 | **Movimiento y escolta**: lo resuelve el núcleo reactivo de la reconstrucción (§3.1, §3.3). | Claude | 0 |
| 3 | **Contrato `MapaGenerado` nuevo y fachada `Mapa`** sobre campos (elevación, capas de bosque/fertilidad/agua/región/pendiente), ríos, lagos, costas, nodos, masas forestales con ID y capacidad, campamentos. Grafo de navegación sobre la máscara nueva (paso 8, ~333 × 250). | Claude + Codex | 1 |
| 4 | **Economía en el generador**, ya en estratégicas: campamentos a 600 → anillos → recursos por anillo → vetas de estaño (exclusión 200, cuotas exactas) → hierro → bosque explotable → bandidos de veta → alijos. `colocarCampamentosIniciales` y `colocarAlijos` salen del motor y pasan al generador. Validación: arranques viables en cada foco. | Claude + Codex | 3 |
| 5 | **Constantes de §4** (velocidades, zonas, `ESCALA`, `radioUrbanoDe`, visión, radios, separación entre plazas). | Claude | 2, 3 |
| 6 | **Persistencia y versiones**: medir cuánto tarda generar el mundo; si es mucho, cachear el mundo por hash en disco del servidor. Subir `WORLDGEN_VERSION`, `BALANCE_VERSION`, `LAYOUT_VERSION` si toca, y el contrato v1 → v2 en lo que cambie del mapa. | Claude | 4, 5 |
| 7 | **Clientes**: admin y jugador dibujan campos raster en vez de discos; zoom de juego ~133 con terreno por tramos (§1). Unity: factor 12, cámara ~180, símbolos (§10). | Cliente jugador / Codex | 3, 6 |
| 8 | **Bots y batch**: el laboratorio genera con el mundo nuevo; los bots saben fundar en foco y buscar estaño. Medir Era I y distancias reales (vecina, mercenarios, bandidos). | Claude | 4, 5, 6 |
| 9 | **Borrado total** (abajo) y fusión de `mundo-regional` en `main`. | Claude | 1-8 |
| 10 | **Canon**: Doc 1 (1.0a, 1.0b, 1.1, 1.2, 1.9, 1.9b) y el resto de §7; consolidar `Docs/Arquitectura` 11-27 de Codex en un único documento del worldgen. Despliegue en Render con mundo nuevo. | Claude | 9 |

### 11.4 Qué se borra

- **Código**: `generarMapa` y su pipeline viejo (`elevacion`, `rios`, `bosques` de discos, `nodos`, `regiones.ts` con los 8
  perfiles y `REGIONES`, `RegionId`), el parámetro `region` de la creación de partidas (admin y `ConfiguracionPartida`),
  la primera fase experimental v16 de Codex si el regional la sustituye, `colocarCampamentosIniciales` y `colocarAlijos`
  del motor (se mueven al generador), las constantes de worldgen que dejen de usarse, `LINEAS_PRODUCCION`, el visor
  viejo y los scripts de auditoría que solo sirven al mundo viejo.
- **Tests**: la caracterización y los snapshots del worldgen viejo (`worldgen_caracterizacion`, `snapshot_baseline`); los
  fixtures que crean mapas con `generarMapa` pasan al generador nuevo o a un mundo de prueba pequeño.
- **Datos**: `partidas/` (incluido `mundo-vivo` y `respaldos/`), `partidas-pruebas/`, `simulaciones-batch/checkpoints/`
  y la partida desplegada en Render.
- **Docs**: los planes de worldgen viejos de `Docs/Arquitectura` que queden superados (con su contenido útil
  consolidado en la fase 10).

### 11.5 Riesgos

1. **Rebase de Codex**: 235 commits de distancia con 40 archivos del motor modificados (`mapa.ts`, `rutas.ts`,
   `estado.ts`, `types.ts`). Es la fase más delicada y conviene hacerla antes de nada más.
2. **Coste de generar**: el mapa se regenera de la seed al abrir cada partida y en cada corrida de batch. Si el regional
   tarda mucho, hace falta caché por hash (fase 6).
3. **Bosque explotable**: sin masas con ID y capacidad no hay Leñeras; es trabajo nuevo, no una adaptación (O2).
4. **Los dos clientes 2D** dibujan discos de bosque y un lienzo de 900 px; hay que rehacer su capa de terreno.
5. **Playtest**: el corte borra el mundo de los testers; hay que elegir el momento.

### 11.6 Por decidir

- **M1.** ¿Reparto de fases como en la tabla (Codex: geografía y Unity; Claude: fachada, economía, motor, persistencia,
  bots)?
- **M2.** ¿Se conserva algo de la primera fase experimental v16 (relieve por perfiles) o la sustituye entera el
  regional?
- **M3.** ¿Un mundo de prueba pequeño para los tests (p. ej. un recorte del regional) o los tests generan el mundo
  entero?
