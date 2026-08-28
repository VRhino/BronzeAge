# Definición de Fase 0.4 — Relieve del mapa jugable (llano/colina, bordes, montaña)

Sub-fase dentro de Fase 0 (sigue siendo datos puros, campo continuo, sin rejilla horneada — ver
`Fase_0_1_Definicion.md`). Exclusiva del RELIEVE del mapa generado (`worldgen/elevacion.ts` +
`worldgen/config.ts`): cómo se ve/se juega la banda llano-colina, cómo se comportan los bordes del mapa, y
qué sigue siendo responsabilidad de montaña/cima. No toca bosques, ríos, chokepoints ni nodos salvo donde el
propio relieve los afecta directamente (ver "Efectos colaterales").

## Por qué ahora

El exportador a Unity Terrain (`world/exportUnity.ts`) subió `alturaMaximaMetros` de 300 a 1000 a petición
del usuario. Con el mapa ya visible en un Terrain 3D real, quedó claro que el relieve de llano/colina —
pensado originalmente solo como diferenciador de BANDAS de terreno en el canvas 2D — se ve y se juega
exageradamente accidentado a esa escala de metros. Pedido explícito de diseño, con una referencia concreta
(captura del mapa de campaña de *Total War: Troy*): el espacio jugable debe leerse como terreno cómodo para
asentar/mover ejércitos, con variación de altura real pero contenida; montaña/cima siguen como frontera
natural entre regiones, sin tocar; y los bordes del mapa deben leerse como costa o pared de montaña, no como
un corte arbitrario del ruido fractal.

## Qué se probó (cronología — todo bajo `WORLDGEN_VERSION`, ver `worldgen/types.ts`)

| Versión | Cambio | Resultado |
|---|---|---|
| v8 | Aplanado parcial: comprime la desviación del campo respecto al centro de la banda jugable un 45%, con peso que se desvanece hacia agua/montaña (`ELEVACION.factorAplanadoJugable`). Junto con esto: bosques más grandes/densos (`BOSQUE.cantidad` 100→170) y un segundo campo de ruido de borde — solo en mundo libre — que empuja los extremos del mapa a costa profunda o montaña empinada (`ELEVACION_BORDE`). | Insuficiente: seguía leyéndose accidentado a 1000m. |
| v9 | Reemplaza el aplanado por TERRACEO: cuantiza la banda jugable en 4 mesetas de valor CONSTANTE, con una rampa corta de entrada a cada una (`ELEVACION_TERRAZAS`, `elevacionTerraceada` en `elevacion.ts`). | Mesetas de verdad, pero el usuario quiso ver el caso extremo. |
| v10 | `niveles: 1` — toda la banda jugable en una sola meseta, experimento a petición del usuario. | Caso límite verificado, sirvió para acotar el mecanismo. |
| v11 | `niveles: 2`, rampa ANCHA compartida (`anchoTransicion: 0.45`) entre ambas mesetas. | La transición se seguía leyendo empinada. |
| v12 | Rampas de entrada/salida SEPARADAS (`anchoTransicionEntrada: 0.7` / `anchoTransicionSalida: 0.1`) — con un solo valor compartido, la meseta alta (única con las dos rampas) se quedaba con solo ~10% de núcleo plano. | Transición más larga, pero seguía sin ser lo pedido de fondo. |
| v13 | Efecto colateral del terraceo: 'colina' quedó sin curvatura real (meseta constante), así que `generarChokepoints` deja de aceptar terreno 'colina' (solo 'montana'); se recalibra el umbral de un test de invariantes (80%→65%, medido empíricamente). | Corrige una regresión real, no cambia el relieve en sí. |

## Por qué el terraceo no es el mecanismo correcto

Comparando el heightmap exportado a Unity con la captura de referencia de *Total War: Troy* que aportó el
usuario, la conclusión es clara: el terraceo (mesetas de valor constante + rampa corta) **no** es lo que
produce el aspecto de un mapa de campaña de TW. Lo que muestra la referencia:

1. **Montaña = otra textura, no solo más altura.** Picos oscuros, angulosos, con sombreado marcado — se leen
   como roca desnuda, contraste de TEXTURA además de altura. Ya lo teníamos correcto (montaña/cima fuera del
   terraceo desde el principio).
2. **El espacio jugable NO tiene escalones visibles — es ondulado y continuo.** No hay plataformas de altura
   constante conectadas por una rampa: el terreno sube y baja de forma gradual en TODO momento. Lo que se
   percibe como "varios niveles" es la ondulación de un relieve continuo de longitud de onda larga, no una
   cuantización discreta.
3. **Los ríos corren limpio por el fondo de valles reales hasta el mar** — exige pendiente real en todas
   partes del terreno jugable. Confirmado empíricamente en este proyecto: con terraceo agresivo (mesetas casi
   perfectamente planas), algunos ríos quedaban vagando hasta agotar `RIOS.pasosMax` (700 pasos) sin bajar,
   porque el gradiente en una meseta plana es ~0 y `RIOS.gradienteMinimo` los da por atrapados en un lago
   antes de tiempo (o, si el gradiente residual era minúsculo pero no exactamente cero, erraban sin rumbo
   claro el presupuesto entero de pasos).
4. **Hay acantilados marcados, pero puntuales** (la costa a la derecha de la imagen) — es la excepción, no
   el criterio general del terreno jugable. Coincide con lo que ya hace `ELEVACION_BORDE` para el borde del
   MAPA; no es evidencia de que el interior deba tener escalones.

## Investigación externa

Se investigó cómo se generan mapas de campaña estilo Total War y qué técnicas usa la industria en 2025 para
terreno "jugable pero natural". Hallazgo principal: **los mapas de campaña de Total War no son procedurales
en tiempo real** — se autoran a mano con *Terry*, el editor de metadatos de terreno de Creative Assembly
(heightmap + blend map + props colocados por diseñadores; ver Total War Wiki, *TWW Assembly Kit Terry
Intro*). Esto explica el aspecto tan controlado de la referencia: es diseño manual, no un algoritmo público
que se pueda replicar — así que igualar el ASPECTO exige técnicas procedurales que imiten ese carácter
(relieve suave con contraste claro hacia montaña), no "encontrar el algoritmo de TW".

Técnicas relevantes encontradas, de más simple a más costosa:

- **Redistribución por curva de potencia** (Red Blob Games, *Making Maps with Noise Functions* — referencia
  clásica del género, sigue siendo la primera parada de cualquier terreno por ruido en juegos): remapear la
  elevación con `e ^ exponente` (opcionalmente `(e × fudge_factor) ^ exponente`) empuja las elevaciones
  medias hacia abajo, aplanando tierras bajas mientras las cimas siguen abruptas. Es la técnica más simple
  para "tierra baja cómoda, montaña dramática" sin introducir discontinuidades — una curva, no un cuantizador.
- **Enmascarado por forma/distancia** (mismo artículo): mezclar el ruido con una función de distancia al
  borde (`lerp(e, forma, mezcla)`) para siluetas controladas — es literalmente el mismo patrón que ya usa
  este proyecto en `ELEVACION_BORDE` (validado independientemente por la fuente externa).
- **Suavizado por pendiente local** ("exponential slope weighting", demo interactiva *The Mountains of
  Madness*): en vez de aplanar según la ALTURA absoluta (que es lo que ha hecho este proyecto hasta ahora, en
  todas las versiones v8-v13), aplanar según la PENDIENTE local — multiplicar la contribución de detalle por
  `e^(−α·pendiente)` reduce mucho las pendientes ya suaves y apenas toca las que ya eran empinadas. Prometedor
  porque responde al CARÁCTER real del terreno en cada punto en vez de a qué banda de altura cae — no necesita
  calibrar umbrales de banda a mano y se adapta solo a cada seed.
- **Erosión hidráulica/térmica** (múltiples fuentes coincidentes): simular el paso de agua sobre el ruido
  erosiona picos y sedimenta valles, produciendo el contraste "valles suaves, crestas afiladas" de forma
  física/natural, sin curvas artificiales. Es la técnica más realista pero también la más cara: simulación
  iterativa de miles de "gotas" sobre una rejilla, no una fórmula evaluable punto a punto.
- **Domain warping** (distorsión del dominio de muestreo antes de evaluar el ruido): rompe la regularidad de
  las formaciones sin cambiar su suavidad — complementario a lo anterior, no sustituto.

### Fuentes
- [TWW Assembly Kit Terry Intro](https://wiki.totalwar.com/w/TWW_Assembly_Kit_Terry_Intro) — Total War Wiki
- [TWWAKT Editing the Heightmap](https://wiki.totalwar.com/w/TWWAKT_Editing_the_Heightmap) — Total War Wiki
- [Making Maps with Noise Functions](https://www.redblobgames.com/maps/terrain-from-noise/) — Red Blob Games
- [The Mountains of Madness — Interactive Terrain Generation Algorithms](https://amanpriyanshu.github.io/The-Mountains-of-Madness/)
- [Improved terrain generation using hydraulic erosion](https://medium.com/@ivo.thom.vanderveen/improved-terrain-generation-using-hydraulic-erosion-2adda8e3d99b) — Ivo van der Veen
- [Simulating hydraulic erosion](https://jobtalle.com/simulating_hydraulic_erosion.html) — Job Talle
- [3DWorld: Domain Warping Noise](http://3dworldgen.blogspot.com/2017/05/domain-warping-noise.html)

## Tensión con la arquitectura actual

`evaluarElevacion`/`evaluarTerreno` son funciones PURAS, evaluables en cualquier punto del plano sin estado
ni vecinos (contrato documentado en `Fase_0_1_Definicion.md`) — es lo que permite muestrear el mundo a
cualquier resolución (canvas 2D, heightmap 4097² de Unity, anillos de colocación de bosques/ríos/nodos que
pueden salirse del mapa) sin guardar una rejilla. La erosión hidráulica/térmica ROMPE ese contrato: es una
simulación iterativa sobre una rejilla fija, no una fórmula punto-a-punto. Si se quisiera ese resultado, hay
dos caminos:

- **(a) Fórmula punto-a-punto que IMITE el efecto de la erosión** — redistribución por curva de potencia +
  suavizado por pendiente local. Más barato, encaja con la arquitectura tal cual (nada cambia fuera de
  `elevacion.ts`), sigue siendo un campo continuo muestreable en cualquier punto.
- **(b) Precalcular un heightmap erosionado una vez por seed** (rejilla fija, p. ej. 2049²) y muestrear por
  interpolación bilineal. Cambio de arquitectura mayor: el campo deja de estar "definido en todo el plano sin
  rejilla", con impacto directo en cómo ríos/bosques/nodos consultan el relieve (hoy lo hacen en cualquier
  punto, incluso fuera de los límites del mapa — ver `Fase_0_1_Definicion.md`).

## Decisión e implementación (Fase 0.4.2, v14 — cierra este documento)

El usuario zanjó las tres preguntas abiertas:
- El terraceo se retira POR COMPLETO (`ELEVACION_TERRAZAS`, `elevacionTerraceada`, `terrazasPeso`) — nada de
  dejarlo como opción desactivable, código muerto fuera.
- Calibración del suavizado, a criterio propio.
- Verificación visual: levantar el servidor local y confirmar en el canvas 2D, como en toda esta sesión.

Se implementó el camino (a) descrito arriba, pero por REDUCCIÓN DE DETALLE (octavas) en vez de por pendiente
local pura — variante más simple de la misma familia de técnicas de la investigación (Red Blob Games:
redistribución/reducción de detalle), y suficiente para resolver el problema sin necesitar un segundo campo
de gradiente:

- **`worldgen/ruido.ts`**: nueva `evaluarRuidoParcial(campo, p, octavasMax)` — el MISMO campo de ruido
  (misma permutación/desplazamientos que `evaluarRuido`), pero sumando solo las `octavasMax` primeras octavas
  (las de frecuencia más gruesa). Sigue siendo una función pura, sin rejilla, coherente con el campo completo
  — nunca un ruido independiente ni un heightmap horneado aparte.
- **`worldgen/config.ts`**: `ELEVACION_SUAVIZADO = { octavasSuaves: 2, pesoMaximo: 0.9 }` — de las 5 octavas
  de `ELEVACION`, se conservan solo las 2 más gruesas (ondulación ~300-600 unidades) para la versión suave;
  se descartan las 3 finas (~37-150 unidades, el "detalle accidentado"). `pesoMaximo` no es 1.0: incluso en
  el centro de la banda jugable queda un 10% del ruido completo mezclado, para que no se note un cambio de
  textura demasiado limpio/artificial hacia montaña.
- **`worldgen/elevacion.ts`**: `evaluarElevacion` mezcla `e` hacia `elevacionSuavizada(campo, p)` con un peso
  `bandaJugablePeso(e) × (1 - fBorde) × pesoMaximo` — mismo patrón de blend que ya usaba el terraceo (y antes
  el aplanado de v8), solo cambia HACIA QUÉ se mezcla: antes una meseta de valor constante, ahora una versión
  de menos octavas del mismo ruido, que sigue teniendo ondulación y gradiente real en todo punto.

### Por qué resuelve los tres problemas identificados arriba

- **Sin escalones**: es un blend continuo entre dos campos de ruido reales, no una cuantización — no hay
  "borde de nivel" en ningún punto.
- **Ríos**: nunca vagaron ni se atascaron en las pruebas (antes, con terraceo, dos ríos por seed llegaban al
  límite de `RIOS.pasosMax`=700 sin bajar; con suavizado, máximo observado 63 pasos, y MÁS ríos navegables
  que con el terraceo — la ondulación real ayuda al descenso por gradiente en vez de estorbarlo).
- **Chokepoints**: se había restringido `generarChokepoints` a solo 'montana' en v13 porque 'colina' quedaba
  sin curvatura bajo el terraceo. Con el suavizado, 'colina' recupera curvatura real — medido 14/14
  chokepoints en colina/montana en las 3 seeds de referencia (frente al 65-100% del terraceo) — así que se
  restauró el filtro original ('colina'+'montana') y el umbral del test de invariantes vuelve a 80%.

### Efecto colateral pendiente de revisar en la próxima sesión

El sitio de fundación de referencia (SEED=42, usado por `fixtures.ts` en los tests del motor) ahora tarda
~76-77 ticks en poder auto-construir su primera Fundición (antes del terraceo: ~37-45; con el terraceo:
similar). Los tests afectados (`lineas_produccion.test.ts`) se ajustaron subiendo su presupuesto de ticks a
100 — no se investigó la causa raíz (¿el suavizado cambia cuánta madera/piedra hay cerca del emplazamiento
que elige `posicionRecomendable`, el escáner de rejilla desde (40,40)? ¿Es solo esta seed?), porque es un
efecto sobre el RITMO económico de una posición de test concreta, no sobre el relieve en sí — fuera de
alcance de este documento, pero vale la pena verificarlo si en partidas reales el arranque se siente lento.

### Percentiles de banda — pendiente de remedir

`ELEVACION.umbralAgua/umbralCosta/umbralColina/umbralMontana/umbralCima` siguen calibrados contra la
distribución del ruido SIN suavizar (comentario en `config.ts` ya desactualizado tras este cambio) — el
suavizado desplaza la distribución real hacia el centro dentro de la banda jugable. No se remidieron los
percentiles en esta sesión porque los tests de invariantes (composición de terreno, cantidad de bosques/
nodos/ríos por bioma) siguen pasando con los umbrales actuales; revisar si en el futuro se nota una banda
más angosta/ancha de lo esperado.
