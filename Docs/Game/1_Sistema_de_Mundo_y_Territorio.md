# 1. Sistema de Mundo y Territorio


## 1.0a La escala del mundo (a petición del usuario, 2026-09-02)

El motor maneja **dos espacios** y hasta ahora nadie había declarado cómo se relacionan, así que el código los
trataba como iguales. La equivalencia es:

> **1 unidad de mapa = 40 unidades locales de la Vista de Asentamiento.**

De ahí sale la jerarquía territorial de la ficción:

| | radio | equivale a |
|---|---|---|
| **Ciudad** (casco urbano + afueras) | 121-205 unidades locales = **3-5 de mapa** | el núcleo habitado |
| **Provincia** (zona de influencia) | **30-180 de mapa**, según nivel | lo que controla un asentamiento |
| **Reino** | todas las provincias de una Facción | sin número fijo: una Facción podría llegar a todo el mapa |

**Cuántas provincias caben.** El mundo mide 2000×2000 y el 91% es habitable (medido: solo 7,6% agua y 1,3%
cima), o sea 3,64 millones de unidades². Repartido entre las **200 provincias** que Fase 0 quiere, salen
18.207 u² por provincia: un **radio de ~76**, casi exactamente el tope de zona de influencia de nivel 1 que ya
existía (60). **La escala de las provincias ya era buena; lo que fallaba era la ciudad.**

**Por qué 40 y no 10.** Una ciudad tiene un TAMAÑO MÍNIMO de ~121 unidades locales que no encoge —al fundar,
la Granja inicial no cabe más cerca— mientras que la provincia sí arranca pequeña (radio 30). Con un factor de
10, una aldea recién fundada ocupaba el **40%** de su provincia y solo llegaba a la décima en niveles altos.
Con 40, ese peor caso mide 3,0 de mapa dentro de una provincia de 30: el **10,1%**. De ahí para arriba el
ratio solo baja (5% en nivel 1, 3% en nivel 5), porque la provincia crece y el mínimo de la ciudad no.

**La regla, entonces: una ciudad nunca pasa de una décima de su provincia.** El resto es campo, bosque y
minas — lo que debe haber entre dos ciudades.

**Qué estaba roto.** Al compartir unidades sin decirlo, una ciudad medía ~121 y su provincia 30-180: **la urbe
era más grande que el territorio que controlaba**. El código incluso lo daba por hecho — `radioMaximoAfueras`
documenta que "el campo de una ciudad está FUERA de su zona de influencia", que es justo el síntoma.

**Un reino mide ~5 provincias**, y eso tampoco lo había decidido nadie: sale de `MANTENIMIENTO.escalaDistancia`
= 400, la distancia a la capital a la que el coste de mantenimiento toca su tope (×2). 400 / 76 ≈ 5.

> La corrección es **conceptual, no numérica**: ni el trazado urbano ni las zonas de influencia cambian de
> tamaño. Lo que cambia es que la equivalencia está dicha, y que `radioUrbanoDe` (`engine/trazado.ts`) es el
> único punto donde los dos espacios se tocan — antes `radioPotencial` hacía los dos trabajos a la vez.

## 1.0b El agua es un OBSTÁCULO (a petición del usuario, 2026-09-02)

**Ni los ejércitos ni las caravanas pueden moverse sobre agua.** No es terreno caro: es infranqueable. Todo lo que se desplaza por el mapa —caravanas de comercio, caravanas de fundación, ejércitos y el trazado de los caminos comerciales— la rodea o no llega.

Esto **revierte una decisión de Fase 0.3**, que la penalizaba fuerte (coste 15) pero la dejaba cruzable "para no romper el pathfinding en un mundo donde el camino más corto la roce". El síntoma que la tumbó: trazando un ejército tick a tick se le veía arrastrarse sobre el mar a **1/14 de su velocidad**, en vez de bordearlo. Nadie camina sobre el agua.

Tres consecuencias, todas deliberadas:

1. **Un destino puede quedar SIN RUTA.** Una isla, una península cortada. Antes el pathfinder caía a una línea recta cuando no encontraba camino; ahora **devuelve "no hay ruta"** y quien lo pidió decide qué hacer — no se moviliza el ejército, no sale la caravana, no se traza el camino comercial. Una recta de reserva sería precisamente una ruta por el mar.
2. **No se puede fundar sobre agua.** Antes solo era absurdo; ahora sería una trampa, porque el asentamiento quedaría incomunicado para siempre. `'agua'` se suma a `'cima'` como terreno inhabitable.
3. **`cima` sigue siendo transitable**, solo que cara (coste 12): es terreno difícil, no un medio distinto.

**No cubre los RÍOS**, que en este motor no son terreno sino una entidad aparte y que el coste de movimiento nunca ha mirado. Que un río corte el paso es parte del rediseño de rutas de caravana (`Docs/Mecanicas a desarrollar.md` §3) y necesita vados o puentes para no fragmentar el mapa.

El comercio por mar sigue fuera de alcance (Doc 3.11): sin barcos, dos costas enfrentadas no comercian.

## 1.1 Generación del mundo (Fase 0)
- Mapa CUADRADO, espacio de coordenadas continuo (no grid discreto).
- Tamaño base: 1000x1000 unidades, PARAMETRIZABLE.
- Terreno con RELIEVE REAL desde Fase 0.1 (ya no plano): campo de ELEVACIÓN continuo por ruido fractal, clasificado en bandas agua/costa/llano/colina/montaña/cima; RÍOS como polilíneas que nacen en montaña y descienden por gradiente de máxima pendiente; BIOMA derivado de terreno+fertilidad+humedad — ver `Consideraciones/Fase_0_1_Definicion.md`. El coste de moverse por el mapa depende del tipo de terreno desde Fase 0.3 (ver 1.5/1.6 y Doc 3.6). Opcionalmente sesgado a una región geográfica real desde Fase 0.2 (Grecia continental/Anatolia/Egeo/Nilo/Mesopotamia — "Libre", sin sesgo, sigue siendo el valor por defecto). Sigue sin mar navegable (eje naval en fase inicial, ver `Roadmap_Escalado.md`).
- Generación PROCEDURAL de recursos por niveles de rareza:
  - Común (alta frecuencia, disperso): Madera (bosques, ver 1.4), Piedra, Trigo (vía fertilidad, ver 1.4).
  - Intermedio (frecuencia media, varios clusters): Cobre.
  - Raro (baja frecuencia, pocos clusters, espaciado mínimo forzado entre ellos): Estaño, Oro.
- Spawn de jugadores nuevos: posición aleatoria uniforme, INDEPENDIENTE de la ubicación de recursos.
- Fases avanzadas (fuera de alcance Fase 0): mar navegable; posible mapa fijo diseñado a mano en vez de procedural; puntos de interés fijos (ruinas, maravillas); clima (el relieve/ríos/biomas de terreno ya se implementaron en Fase 0.1, ver arriba).

## 1.2 Fundación de asentamientos
- El jugador elige LIBREMENTE dónde colocar el edificio de fundación.
- **Aviso de viabilidad al fundar** (ayuda de UI, no bloqueante, a petición del usuario): al mover el ratón sobre el mapa antes de confirmar, se dibuja una previsualización del radio inicial (30 unidades, ver TAMAÑO abajo) coloreada según viabilidad — VERDE si la posición es fundable y hay bosque alcanzable dentro del radio (madera garantizada desde el principio), ÁMBAR si es fundable pero sin madera al alcance (el asentamiento tendrá que arrancar solo con la reserva de materiales iniciales, ver 1.3), ROJO si la posición no es válida (fuera del mapa o solapa una zona de influencia existente). No impide fundar en ámbar ni en ningún punto válido — es solo información para decidir mejor, el jugador puede ignorarla. Implementado en `evaluarViabilidadFundacion` (`engine/settlement.ts`) y `drawPreviewFundacion` (`ui/canvas.ts`); también comprueba qué nodos minerales caen dentro del radio inicial.
- Al fundar se genera automáticamente una ZONA DE INFLUENCIA (círculo/polígono) = nodo.
- TAMAÑO: nace con un radio inicial de 30 unidades y crece gradualmente cada tick hacia un TECHO que escala con el NIVEL del asentamiento (ver Doc 4.5 para el modelo de nivel): nivel 1 → 60, nivel 2 → 90, nivel 3 → 120 (tope de Fase 0, ver 4.5). Subir de nivel no hace saltar el radio de golpe — solo levanta el techo hacia el que la zona ya venía creciendo. NIVEL 4 Y 5: planeados a futuro (fuera del rango original de nivel de asentamiento pensado en diseño), pero NO IMPLEMENTADOS en Fase 0 — no tienen gates de población/edificios definidos ni techo de radio propio todavía. No inventar valores hasta que se definan junto con sus gates correspondientes en Doc 4.5.
- Reglas de construcción: solo se puede construir dentro de una zona de influencia existente (ampliándola si es del mismo bando) o fuera de cualquier zona (creando una nueva).
- FRONTERAS: al chocar dos zonas en expansión se genera un LÍMITE DURO — ninguna zona sigue creciendo en esa dirección. Solo se rompe si el asentamiento rival cae o su zona se debilita/reduce (guerra u otros medios). Las fronteras son "vivas", reflejan el poder relativo de cada bando en cada momento.

## 1.3 Onboarding de nuevos jugadores
- Jugador nuevo aparece en un punto ALEATORIO del mapa con una "caravana de asentamiento" para fundar donde decida.
- **Aparecer ahí es literal (ver 1.10)**: el jugador nace situado en mundo abierto, con su columna, y se mueve por el mapa hasta el sitio donde quiera fundar. **Se funda DONDE SE ESTÁ** — no se elige un punto cualquiera sobre el mapa desde fuera. Caminar hasta un buen emplazamiento es la primera decisión del juego, y es lo que da sentido a explorar antes de asentarse.
- **SE LLEGA A UN MUNDO HABITADO, no a un vacío** (a petición del usuario, 2026-09-07). El servidor arranca
  con Facciones NPC ya asentadas, y son **vecinos, no depredadores**: se defienden si las tocan, pero no dan
  caza a los recién llegados, y ofrecen con qué comerciar. Un novato no es aliado de nadie, así que unas
  Facciones que cazaran a todo lo no aliado lo matarían antes de que tuviera con qué defenderse.

  El mundo no se queda por eso inofensivo: el peligro de base lo dan los **bandidos** (1.9), que atacan por su
  cuenta y no son de nadie. El reparto es **bandidos la amenaza, Facciones NPC los vecinos, otros jugadores la
  guerra**.
- **FUNDAR NO ES EL PRIMER ACTO.** Para fundar una Facción nueva hacen falta **varios ciudadanos** y **haber
  sido ciudadano de alguna antes** — fundar es un **cisma**, gente que ya vivía en algún sitio y se marcha a
  hacer el suyo. Sin esto, mil jugadores que entran son mil Facciones y mil aldeas en el primer minuto, y la
  fundación grupal (hasta 5) *permitía* compartir pero no obligaba a nada.

  Las dos exigencias son **parámetros** (`FUNDACION.minFundadoresParaFaccionNueva` y
  `FUNDACION.exigeCiudadaniaPrevia`), y durante las primeras pruebas están **abiertas** —grupo de 1 y
  ciudadanía opcional—: con cinco testers el freno estorba, con mil hace falta.
- **Un jugador sin Facción recuerda lo que explora.** La memoria del mundo es de la Facción (Doc 5.12.8), pero quien todavía no tiene bandera lleva la suya propia, que se funde con la de la Facción al fundar o al entrar en una. Sin esto el primer minuto de juego sería un paseo a ciegas sin registro.
- EDIFICIOS INICIALES: todo asentamiento nace con un Centro Urbano (marcador único, no construible por ningún otro medio), una Granja y 3 Viviendas, ya ACTIVOS sin pasar por la cola de construcción. La Leñera inicial condicional (ligada a bosque alcanzable) que existió en una versión anterior de esta mecánica fue RETIRADA (`engine/settlement.ts`, `edificiosIniciales`) — la reserva de materiales iniciales (ver abajo) ya bastaba por sí sola para evitar el deadlock de madera, dejando esa mitigación extra innecesaria.
- FUNDACIÓN GRUPAL: hasta 5 jugadores pueden organizarse para aparecer juntos en el mismo punto, compartiendo una caravana, fundando el asentamiento entre los 5. Los 5 reciben Ciudadanía de inmediato.
- MATERIALES INICIALES (confirmado durante implementación de Fase 0): la caravana de fundación entrega una reserva inicial de recursos al fundar, suficiente para arrancar la primera construcción. Sin esto el asentamiento queda bloqueado permanentemente (el edificio que produce madera también cuesta madera para construirse — deadlock detectado y corregido en Sprint 2, ver `Correcciones_Durante_Desarrollo.md`). Cifras actuales (`FUNDACION.materialesIniciales`/`POBLACION.pesants.inicial`, constants.ts, placeholder sin calibrar): 50 madera + 20 piedra + 100 trigo + 100 oro, y población inicial de 20 pesants.
- PROTECCIÓN TEMPORAL — RESUELTO durante implementación (ya no es pregunta pendiente): existe un período de gracia en ticks tras la fundación durante el cual NO se cobra Mantenimiento (ver Doc 4.5). Sin esto, todo asentamiento nuevo caía en ruinas de forma sistemática antes de que su economía pudiera arrancar. Duración exacta: placeholder ajustable.

## 1.4 Fuentes de recursos por tipo
- CULTIVOS (trigo): NO son un nodo recolectable directo. Dependen de la FERTILIDAD DEL SUELO de la zona (atributo de terreno); requieren construir una Granja para aprovecharse.
- MADERA: proviene de BOSQUES, representados como ZONAS del mapa (no puntos), con densidad variable.
- MINERALES (cobre, estaño, oro): desde Fase 0.1, CONDICIONADOS AL RELIEVE en vez de dispersos al azar — cobre/estaño en colina o montaña, oro solo en montaña (el más exclusivo, es el más raro); piedra se deja permisiva (colina/montaña/llanura fértil/estepa) para no comprometer el recurso común más consumido — ver `RECURSO_BIOMA_PERMITIDO` en `worldgen/config.ts`. Cada uno tiene su propio edificio de extracción (cantera para piedra, mina de oro, mina de cobre, mina de estaño — este último añadido en un rebalance posterior a Sprint 6: el estaño se generaba en el mundo desde el Sprint 1 pero no tenía forma de extraerse, solo de comerciarse).
- LIVESTOCK (ovejas, vacas, caballos + especies adicionales por definir): fauna LIBRE en el mapa, debe CAPTURARSE para aprovecharse. Cría/domesticación pospuesta a fases avanzadas (en Fase 0 se trata como recurso consumible/finito). Rendimientos: ovejas = carne+leche; vacas = carne+leche+cuero; caballos = fuente de entrenamiento de tropas montadas (caballería ligera, carros de guerra), no dan recurso de consumo.
- MADERA y PIEDRA: materiales base de construcción de edificios e insumo de materiales derivados.
- REGENERACIÓN DE YACIMIENTOS AGOTADOS (a petición del usuario): un nodo (mineral o livestock) que llega a stock 0 no queda muerto para siempre — vuelve a aparecer con su cantidad inicial completa pasados N ticks de cooldown, parametrizable en caliente por categoría (`REGENERACION_NODOS`, `world/mapa.ts`/`Mapa.avanzarRegeneracion`). Livestock (fauna, se recupera por reproducción/migración) regenera al DOBLE de rápido que los yacimientos minerales (piedra/cobre/estaño/oro, cooldown mucho más largo — un filón agotado tarda mucho más en "rellenarse" que una manada).
- COMMODITIES DE NOBLEZA (uvas/olivas → vino/aceite): no son alimento básico, requeridas para felicidad de la Nobleza; su déficit arriesga rebelión/estancamiento, no hambruna.

## 1.5 Chokepoints estratégicos (heredado de Iberia) — 🔷 implementado con alcance reducido (Fase 0.3)
IMPLEMENTADO desde Fase 0.3 (la condición "aplica a partir de fases con relieve" ya se cumple: el relieve real llegó en Fase 0.1). Puertos de montaña detectados como PUNTOS DE SILLA del campo de elevación (mínimo local a lo largo de la cresta, máximo local en la dirección perpendicular — geometría determinista por seed, ver `worldgen/chokepoints.ts`). El asentamiento cuya zona de influencia CUBRE un chokepoint lo controla; las caravanas comerciales de una Facción rival cuya ruta pasa cerca pagan un PEAJE EN ORO al controlador, cobrado al llegar a destino (ver `engine/chokepoints.ts`, `Consideraciones/Fase_0_3_Definicion.md`).
PENDIENTE (alcance confirmado con el usuario para esta pasada): escoltar caravanas aliadas y bloquear el suministro de una Facción/Liga rival — el diseño original completo necesita un concepto de "guerra activa" entre Facciones que no existe todavía (el combate en Fase 0 es cálculo puntual: asedio/campo abierto/intercepción, sin presencia física continua en el mapa, ver Doc 5.10). Vados de río como chokepoint: esta pasada solo detecta puertos de montaña.

## 1.6 Caminos comerciales automáticos — ✅ implementado (Fase 0.3)
- Al proponer un trueque entre dos asentamientos se genera AUTOMÁTICAMENTE un camino físico — una polilínea calculada con pathfinding (A* sobre coste de terreno) que rodea relieve costoso en vez de ir en línea recta, mismo algoritmo que usa cualquier caravana para su propia ruta (ver 1.1, Doc 3.6, `world/rutas.ts`) — el jugador no lo construye manualmente (`engine/caminos.ts`).
- El camino AFECTA LA VELOCIDAD de las caravanas que lo siguen (más rápido que campo abierto — ver Doc 3.6).
- MEJORABLE vía Políticas: cubierto por la política ya existente "Rutas Rápidas" (Tesorero, Doc 3.12) — no hizo falta ninguna política nueva.
- RESUELTO: si se rompe la relación comercial que originó el camino, el camino queda como infraestructura física PERMANENTE (no se elimina) — decisión pragmática sin validar por simulación (ver `Consideraciones/Preguntas_Abiertas.md`).
- Tropas: fuera de alcance todavía — el movimiento militar físico por el mapa sigue sin existir en Fase 0 (Doc 5.10).

## 1.7 Cap de fundación de asentamientos por Facción (inspirado en Rise of Nations)
- Límite DURO de asentamientos que una Facción puede FUNDAR (no aplica a conquista/anexión, que no tiene límite).
- ESCALA con el NIVEL DE FACCIÓN: progresión fácil de 1 a 3, luego se complica progresivamente hasta un MÁXIMO DE 7 en etapa tardía.
- Complementa (no sustituye) al sistema de Mantenimiento/Coste de Gobernanza (ver Doc 4).
- Da incentivo mecánico a preferir vasallaje/conquista sobre fundación directa una vez alcanzado el cap.
- ESTADO DE IMPLEMENTACIÓN: implementado y enforced (`calcularCapFundacion`/`CAP_FUNDACION_POR_NIVEL`, `engine/faccion.ts`/`constants.ts`) — `fundarAsentamiento` rechaza con `FundacionInvalidaError` si la Facción ya está en el cap de su nivel. Curva actual: `[1, 2, 3, 3, 4, 5, 5, 6, 6, 7]` para niveles de Facción 1 a 10 (progresión fácil de 1 a 3, luego se complica hasta el máximo de 7). Cubierto por test de unidad y por `invariantes_simulacion_larga.test.ts`.
- **Qué hace subir el nivel de Facción — RESUELTO**: por experiencia acumulada (combate, edificio completado, conquista, defensa/ataque de caravana), ver Doc 2.2.1 para el criterio completo, la curva de umbrales y el cupo de asentamientos nivel 2/3 que también depende de este mismo nivel.
- PENDIENTE: calibración final de la curva de cap y de los umbrales de XP por simulación.

## 1.8 Caravana de Fundación (mecanismo de expansión más allá del primer asentamiento, sometido a consejo LLM)

Mecanismo COMPLEMENTARIO al Cap de Fundación (1.7) — ambos coexisten, no se sustituyen. El objetivo es que fundar un asentamiento adicional cueste recursos reales y se sienta orgánico, evitando fundación en cadena sin fricción.

**COSTE** = suma de tres componentes:
1. Los materiales iniciales que recibe todo asentamiento nuevo al fundar (ver 1.3: reserva de madera+piedra).
2. El coste de construcción de los edificios que nacen automáticamente con la fundación (Centro Urbano, Granja, 3 Viviendas — ver Doc 4.2.1).
3. +50 de madera extra, representando el coste de fabricar la caravana en sí (placeholder, sin calibrar por simulación todavía).

**GATES DE ORIGEN (todos necesarios simultáneamente):**
- Solo puede lanzarse desde un asentamiento que PUEDA PAGAR el coste completo.
- Solo puede lanzarse desde un asentamiento en NIVEL 2 como mínimo (no Nivel 1) — gate estructural independiente del coste, pensado como salvaguarda robusta ante futuras recalibraciones de precios (no depende de números ajustables).
- **Cooldown de creación (2026-08-20, a petición del usuario)**: tras lanzar una Caravana de Fundación —o construir una comercial (Doc 3.12), mismo cooldown COMPARTIDO entre las dos— el asentamiento de origen no puede crear otra hasta que pasen `CARAVANA_COOLDOWN.ticksCooldown` (`constants.ts`, 10 ticks, parametrizable). Evita spam de creación cuando la caravana recién lanzada es destruida (bandidos, Doc 1.9; intercepción, Doc 3.10) y el Cap de Fundación/recursos vuelven a estar disponibles de inmediato. Regla del MOTOR (`puedeCrearCaravana`, `engine/asentamientoQuery.ts`), no de la interfaz ni del NPC — gatea igual el lanzamiento manual y el de la gobernanza NPC.

**NATURALEZA:**
- Se lanza por ACCIÓN MANUAL EXPLÍCITA del jugador desde la interfaz (elige destino y confirma) — a diferencia del trueque/mercado, que en Fase 0 se despachan automáticamente (ver Doc 3.2/3.3). Es la única caravana donde la intencionalidad del jugador es el punto central del diseño.
- INTERCEPTABLE Y ESCOLTABLE igual que cualquier otra caravana (Doc 3.10) — mismas reglas de combate asimétrico y umbral de captura del 50%, sin regla especial.

**CASOS RESUELTOS (vía consejo LLM):**
- Si el punto de destino elegido deja de estar disponible en tránsito (ej. otra Facción funda ahí primero): en fases con movimiento libre de caravana por el mapa (Fase 1+), el jugador simplemente MUEVE la caravana ya en marcha hacia otro punto válido y funda allí — no se pierde el viaje ni el coste.
- CANCELACIÓN: el jugador puede DESARMAR la caravana de fundación en el asentamiento de origen y recuperar el contenido COMPLETO — sin pérdida por cambiar de opinión antes de fundar.

PENDIENTE: número de 50 madera sin validar por simulación todavía (igual que el resto de cifras del proyecto).

## 1.9 Campamentos de bandidos — IMPLEMENTADO (nuevo, a petición del usuario — inspirado en análisis comparativo con Travian)

- Aparecen únicamente en BOSQUES (ver 1.4) que NO se solapan con ninguna zona de influencia existente — territorio no reclamado por ninguna Facción. Simplificación de implementación: se chequea el CENTRO del bosque contra los polígonos de zona (mismo criterio que `posicionLibreParaFundar`), no el círculo completo.
- **UNO por asentamiento, SIEMPRE (rediseño a petición del usuario, corregido tras detectar un bug real)**: el tope ya no es un número fijo de campamentos en todo el mundo — es UNO por cada asentamiento vivo, apareciendo en SU bosque no reclamado MÁS CERCANO. Así nunca aparece en la otra punta del mapa sin ningún asentamiento cerca (irrelevante: nadie puede atacarlo, ninguna caravana pasa por ahí) ni pegado a una zona de influencia (ya excluido por la regla de "no reclamado"). Cada campamento queda ASIGNADO a su asentamiento (`asentamientoId`) — un asentamiento sin campamento propio siempre puede recibir el suyo, sin importar lo cerca que esté de otro asentamiento ya atendido. (Un criterio anterior consideraba "atendido" a cualquier asentamiento con un campamento a menos de una distancia fija, lo que dejaba asentamientos vecinos sin campamento propio para siempre si compartían esa distancia con otro — bug real: con 3 asentamientos cercanos solo llegaba a aparecer 1.) Mientras haya asentamientos sin cubrir y se cumpla el plazo de reaparición, cada tick se cubre como mucho uno.
- Mientras el campamento sigue en pie, ATACA CARAVANAS (ver 1.6, Doc 3.6/3.10) que pasen dentro de un radio fijo de su posición, evaluado cada tick — mismo tipo de resolución de combate asimétrico que la intercepción entre Facciones (Doc 3.10: poder fijo con jitter contra la defensa base de caravana), pero contra un bando NPC en vez de un rival jugador. Si gana, la caravana se pierde por completo (nadie la recibe: el bandido no tiene almacén propio).
- Al ser DESTRUIDO (acción militar MANUAL del jugador — elige campamento y escuadrones propios, pestaña Guerra — mismo cálculo de combate sin representación gráfica que el resto de Fase 0, ver Doc 5.10), entrega una RECOMPENSA (loot) fija a quien lo destruye y se agenda el plazo de reaparición. SIN gate de cargo (corrección — a diferencia de asediar, Doc 5.2, e interceptar caravanas, Doc 3.10, que sí requieren General): un campamento bandido es una amenaza NPC de mundo abierto, no una acción de guerra entre Facciones — basta con elegir escuadrones propios (de cualquier jugador residente, Doc 2.5) y vencer en combate.
- REAPARICIÓN: tras destruirse, aparece un campamento nuevo pasados N ticks — cerca del primer asentamiento sin cobertura que encuentre (no necesariamente el mismo que perdió el suyo).
- Implementado en `engine/bandidos.ts` (spawn/respawn automático y ataque a caravanas, evaluados cada tick dentro de `avanzarSimulacion`) y `engine/combate.ts` (`atacarCampamentoBandidos`, ataque manual del jugador). Marcador visible en el mapa (diamante rojo) y selector dedicado en la pestaña Guerra. Verificado en el navegador con 2 asentamientos en esquinas opuestas de un mapa 2000×2000: cada uno recibió su propio campamento en su bosque más cercano (69 y 103 unidades de distancia respectivamente, nunca en la zona del otro asentamiento), tope respetado en 2 (= número de asentamientos), y ataque/destrucción/botín/reaparición verificados de punta a punta.

Cifras actuales, todas PLACEHOLDER sin calibrar por simulación (`CAMPAMENTOS_BANDIDOS`, constants.ts):
- Poder de combate: 30 (fijo, sin escalado por región).
- Radio de ataque a caravanas: 40 unidades del mapa.
- Recompensa: 40 madera + 20 piedra + 15 oro.
- Reaparición: 60 ticks tras destruirse (parametrizable en caliente desde el panel de balance, `CAMPAMENTOS_BANDIDOS.ticksRespawn`).

PENDIENTE (calibración, no diseño):
- Ajustar poder/radio/recompensa/cadencia por simulación.
- Si el campamento debería escalar con región/proximidad a Facciones fuertes, o seguir fijo.
- Si el campamento tiene algún efecto pasivo sobre el bosque que ocupa (ej. bloquear su explotación) o solo amenaza caravanas de paso — actualmente NO bloquea nada, solo amenaza caravanas.

## 1.10 El héroe está SITUADO en el mundo (a petición del usuario, 2026-09-06)

El héroe de cada jugador es un partícipe del mundo, no un ente volador superior. En todo momento está en **uno** de tres sitios, nunca en dos y nunca en ninguno:

| Dónde | Qué significa |
|---|---|
| **Dentro de un asentamiento** | Ve su interior completo y puede dar órdenes ahí |
| **En una columna, en el mapa** | Se mueve, ve lo que su columna alcanza, interactúa con lo que se cruza (Doc 5.12) |
| **Desconectado** | Fuera del mundo, con su sitio guardado |

### 1.10.1 Solo ves donde estás, y solo actúas donde estás

**Nunca se ve el interior de un asentamiento en el que no se está físicamente.** Ni siquiera uno propio, ni siquiera siendo su Gobernador. Almacén, guarnición, colas de construcción, cargos y trazado urbano son cosas que se miran desde dentro.

Y la regla es también de ACCIÓN: **construir, reclutar, comerciar, activar políticas y designar cargos exige estar dentro** de la plaza en cuestión (Doc 2.5). Con acción remota el jugador seguiría siendo un ojo volador, solo que con una venda.

**Lo ya ordenado sigue corriendo solo.** Salir no congela tu ciudad: la auto-construcción avanza, las colas terminan, las caravanas en ruta llegan, la producción produce. Lo que no puedes es darle órdenes NUEVAS mientras no estés.

**Lo que se deja atrás se RECUERDA.** Al salir de una plaza queda la última foto de su interior, fechada — el mismo mecanismo con el que se recuerda una ciudad ajena que se dejó de ver (Doc 5.12.8). El jugador juzga si fiarse de un dato de hace tres horas.

> **Esto NO toca la niebla de guerra.** Tus plazas siguen vigilando su radio para toda tu Facción aunque no estés dentro de ninguna, y lo avistado y lo recordado del MAPA siguen igual (Doc 5.12.7-5.12.8). Lo que se restringe son los INTERIORES, no saber dónde están las cosas ni qué pasa en el mundo.

### 1.10.2 Salir al mundo

Se sale desde la **residencia**, que es donde el jugador tiene su roster entero de tropas. Al salir elige, en una sola pantalla:

1. **Con qué tropas sale**, bajo su Liderazgo (Doc 5.11) — puede ser una sola, todas las que el Liderazgo permita, o **ninguna**.
2. **Con qué materiales sale**, hasta llenar su carro (Doc 5.13).

Y aparece en el mapa de mundo **junto al asentamiento**, sin destino todavía.

*Pendiente:* que el **Tesorero** pueda fijar cuánto material del almacén puede retirar cada jugador.

### 1.10.3 Entrar en un asentamiento

Se entra estando en la **puerta** —a corta distancia de la plaza— y se ofrece como una acción, no ocurre solo (Doc 5.12.3). Lo que pasa con la columna depende de dónde entres:

| Entras en… | Tu columna |
|---|---|
| **Tu residencia** | Se disuelve: los escuadrones vuelven a la guarnición y el carro al almacén |
| **Cualquier otra plaza** | Se queda **aparcada a la puerta**. Al salir la retomas con lo que llevabas, sin ninguna pantalla de equipamiento |

De ahí salen dos consecuencias sin necesidad de más reglas: si conquistan la plaza ajena mientras estás dentro, tu columna está fuera y la retomas; y si te destruyen la columna aparcada, sales a pie.

### 1.10.4 Dentro de una plaza ajena solo se ve la capa pública

Trazado, edificios visibles y mercado. **Nunca** almacén exacto, guarnición, colas de construcción ni cargos.

Entrar es reconocimiento legítimo —ves si la ciudad es grande, rica y está amurallada— y por eso cerrar la puerta sigue siendo una defensa real sin que abrirla sea suicida.

### 1.10.5 La puerta la controla el Gobernador

El Gobernador fija quién puede entrar en su plaza: **abierta a todos**, **solo a su Facción**, **a su Facción y sus aliados**, o **cerrada**. Puede además vetar a jugadores concretos por encima de esa política.

No es una política de las que expiran (Doc 4.4): una puerta que se abre sola a las dos horas y media no es una puerta.

### 1.10.6 Desconectarse

**Al desconectarse, el jugador desaparece del mundo en el punto donde quedó, y reaparece ahí al volver.** No es una excepción caprichosa: es lo que hace jugable un mundo persistente para una persona sola.

**Y se lleva sus tropas con él.** El jugador y todo lo que carga —sus escuadrones y su carro— entran y salen del mundo juntos. Al volver aparece con ellos en el último sitio donde estuvo, lo que le da la oportunidad de alcanzar a los suyos si iba en un ejército.

Si iba en un **Ejército**, este **sigue su marcha sin él**, más débil: mecánicamente, desconectarse es separarse (Doc 5.14.2) y desaparecer. Y si el que se desconecta era el **Líder**, el mando pasa al integrante con más antigüedad (Doc 5.14.3) — el Líder no puede separarse por voluntad propia, pero sí puede caerse la conexión, y la columna no puede quedarse sin mando.

> **Alcanzar de vuelta a tu ejército no siempre se puede.** Una columna va al ritmo de su escuadrón más lento (Doc 5.12.5), así que solo alcanzas a los tuyos si tus tropas son más rápidas que la más lenta de la columna. El que llevaba la tropa pesada que frenaba a todos no vuelve a alcanzarlos.

**Desaparecer tarda dos minutos y medio.** No es instantáneo: al desconectarse, el jugador sigue en el mundo ese rato —moviéndose como iba— y solo entonces se lo lleva todo consigo. Es lo que impide desconectarse para escapar de un combate que ya se tiene encima, sin castigar por ello a quien sufre un corte de verdad: en 2:30 un perseguidor cubre 30-55 unidades, así que alcanza a quien ya tenía a tiro y no a quien iba lejos.

**Si se desconectan TODOS los integrantes de un ejército**, cada uno se lleva lo suyo y las **caravanas adjuntas vuelven solas a su asentamiento de origen** — haciendo el camino, así que son interceptables durante el regreso. No es lo mismo que perderlas: a un ejército DERROTADO se las quita el enemigo (Doc 5.13.2), y a estas no las venció nadie. Si su origen ya no existe, se pierden.
