# 1. Sistema de Mundo y Territorio

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
- Al fundar se genera automáticamente una ZONA DE INFLUENCIA (círculo/polígono) = nodo.
- TAMAÑO: nace con un radio inicial de 30 unidades y crece gradualmente cada tick hacia un TECHO que escala con el NIVEL del asentamiento (ver Doc 4.5 para el modelo de nivel): nivel 1 → 60, nivel 2 → 90, nivel 3 → 120 (tope de Fase 0, ver 4.5). Subir de nivel no hace saltar el radio de golpe — solo levanta el techo hacia el que la zona ya venía creciendo. NIVEL 4 Y 5: planeados a futuro (fuera del rango original de nivel de asentamiento pensado en diseño), pero NO IMPLEMENTADOS en Fase 0 — no tienen gates de población/edificios definidos ni techo de radio propio todavía. No inventar valores hasta que se definan junto con sus gates correspondientes en Doc 4.5.
- Reglas de construcción: solo se puede construir dentro de una zona de influencia existente (ampliándola si es del mismo bando) o fuera de cualquier zona (creando una nueva).
- FRONTERAS: al chocar dos zonas en expansión se genera un LÍMITE DURO — ninguna zona sigue creciendo en esa dirección. Solo se rompe si el asentamiento rival cae o su zona se debilita/reduce (guerra u otros medios). Las fronteras son "vivas", reflejan el poder relativo de cada bando en cada momento.

## 1.3 Onboarding de nuevos jugadores
- Jugador nuevo aparece en un punto ALEATORIO del mapa con una "caravana de asentamiento" para fundar donde decida.
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
- PENDIENTE: qué hace subir exactamente el "nivel de Facción" (la curva de cap ya está fijada, pero el criterio de progresión de nivel de Facción en sí sigue sin definir, distinto del nivel de asentamiento de Doc 4.5); calibración final de la curva por simulación.

## 1.8 Caravana de Fundación (mecanismo de expansión más allá del primer asentamiento, sometido a consejo LLM)

Mecanismo COMPLEMENTARIO al Cap de Fundación (1.7) — ambos coexisten, no se sustituyen. El objetivo es que fundar un asentamiento adicional cueste recursos reales y se sienta orgánico, evitando fundación en cadena sin fricción.

**COSTE** = suma de tres componentes:
1. Los materiales iniciales que recibe todo asentamiento nuevo al fundar (ver 1.3: reserva de madera+piedra).
2. El coste de construcción de los edificios que nacen automáticamente con la fundación (Centro Urbano, Granja, 3 Viviendas — ver Doc 4.2.1).
3. +50 de madera extra, representando el coste de fabricar la caravana en sí (placeholder, sin calibrar por simulación todavía).

**GATES DE ORIGEN (ambos necesarios simultáneamente):**
- Solo puede lanzarse desde un asentamiento que PUEDA PAGAR el coste completo.
- Solo puede lanzarse desde un asentamiento en NIVEL 2 como mínimo (no Nivel 1) — gate estructural independiente del coste, pensado como salvaguarda robusta ante futuras recalibraciones de precios (no depende de números ajustables).

**NATURALEZA:**
- Se lanza por ACCIÓN MANUAL EXPLÍCITA del jugador desde la interfaz (elige destino y confirma) — a diferencia del trueque/mercado, que en Fase 0 se despachan automáticamente (ver Doc 3.2/3.3). Es la única caravana donde la intencionalidad del jugador es el punto central del diseño.
- INTERCEPTABLE Y ESCOLTABLE igual que cualquier otra caravana (Doc 3.10) — mismas reglas de combate asimétrico y umbral de captura del 50%, sin regla especial.

**CASOS RESUELTOS (vía consejo LLM):**
- Si el punto de destino elegido deja de estar disponible en tránsito (ej. otra Facción funda ahí primero): en fases con movimiento libre de caravana por el mapa (Fase 1+), el jugador simplemente MUEVE la caravana ya en marcha hacia otro punto válido y funda allí — no se pierde el viaje ni el coste.
- CANCELACIÓN: el jugador puede DESARMAR la caravana de fundación en el asentamiento de origen y recuperar el contenido COMPLETO — sin pérdida por cambiar de opinión antes de fundar.

PENDIENTE: número de 50 madera sin validar por simulación todavía (igual que el resto de cifras del proyecto).

## 1.9 Campamentos de bandidos — IMPLEMENTADO (nuevo, a petición del usuario — inspirado en análisis comparativo con Travian)

- Aparecen únicamente en BOSQUES (ver 1.4) que NO se solapan con ninguna zona de influencia existente — territorio no reclamado por ninguna Facción. Simplificación de implementación: se chequea el CENTRO del bosque contra los polígonos de zona (mismo criterio que `posicionLibreParaFundar`), no el círculo completo.
- **UNO por asentamiento (rediseño a petición del usuario)**: el tope ya no es un número fijo de campamentos en todo el mundo — es UNO por cada asentamiento vivo, apareciendo en SU bosque no reclamado MÁS CERCANO. Así nunca aparece en la otra punta del mapa sin ningún asentamiento cerca (irrelevante: nadie puede atacarlo, ninguna caravana pasa por ahí) ni pegado a una zona de influencia (ya excluido por la regla de "no reclamado"). Un asentamiento se considera "atendido" (no recibe un segundo campamento) si ya tiene uno dentro de su radio de cobertura (placeholder, ver más abajo); mientras haya asentamientos sin cubrir y se cumpla el plazo de reaparición, cada tick se cubre como mucho uno.
- Mientras el campamento sigue en pie, ATACA CARAVANAS (ver 1.6, Doc 3.6/3.10) que pasen dentro de un radio fijo de su posición, evaluado cada tick — mismo tipo de resolución de combate asimétrico que la intercepción entre Facciones (Doc 3.10: poder fijo con jitter contra la defensa base de caravana), pero contra un bando NPC en vez de un rival jugador. Si gana, la caravana se pierde por completo (nadie la recibe: el bandido no tiene almacén propio).
- Al ser DESTRUIDO (acción militar MANUAL del jugador — elige campamento y escuadrones propios, pestaña Guerra — mismo cálculo de combate sin representación gráfica que el resto de Fase 0, ver Doc 5.10), entrega una RECOMPENSA (loot) fija a quien lo destruye y se agenda el plazo de reaparición. SIN gate de cargo (corrección — a diferencia de asediar, Doc 5.2, e interceptar caravanas, Doc 3.10, que sí requieren General): un campamento bandido es una amenaza NPC de mundo abierto, no una acción de guerra entre Facciones — basta con elegir escuadrones propios (de cualquier jugador residente, Doc 2.5) y vencer en combate.
- REAPARICIÓN: tras destruirse, aparece un campamento nuevo pasados N ticks — cerca del primer asentamiento sin cobertura que encuentre (no necesariamente el mismo que perdió el suyo).
- Implementado en `engine/bandidos.ts` (spawn/respawn automático y ataque a caravanas, evaluados cada tick dentro de `avanzarSimulacion`) y `engine/combate.ts` (`atacarCampamentoBandidos`, ataque manual del jugador). Marcador visible en el mapa (diamante rojo) y selector dedicado en la pestaña Guerra. Verificado en el navegador con 2 asentamientos en esquinas opuestas de un mapa 2000×2000: cada uno recibió su propio campamento en su bosque más cercano (69 y 103 unidades de distancia respectivamente, nunca en la zona del otro asentamiento), tope respetado en 2 (= número de asentamientos), y ataque/destrucción/botín/reaparición verificados de punta a punta.

Cifras actuales, todas PLACEHOLDER sin calibrar por simulación (`CAMPAMENTOS_BANDIDOS`, constants.ts):
- Poder de combate: 30 (fijo, sin escalado por región).
- Radio de ataque a caravanas: 40 unidades del mapa.
- Radio de cobertura (a partir del cual un asentamiento ya se considera atendido): 600 unidades — sin techo máximo real: si el bosque no reclamado más cercano de un asentamiento queda lejos porque está rodeado de zonas de otras Facciones, se spawnea igual ahí (la mejor opción disponible, no bloquear la mecánica).
- Recompensa: 40 madera + 20 piedra + 15 oro.
- Reaparición: 60 ticks tras destruirse.

PENDIENTE (calibración, no diseño):
- Ajustar poder/radio/recompensa/cadencia/radio de cobertura por simulación.
- Si el campamento debería escalar con región/proximidad a Facciones fuertes, o seguir fijo.
- Si el campamento tiene algún efecto pasivo sobre el bosque que ocupa (ej. bloquear su explotación) o solo amenaza caravanas de paso — actualmente NO bloquea nada, solo amenaza caravanas.
