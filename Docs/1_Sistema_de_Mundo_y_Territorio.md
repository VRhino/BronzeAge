# 1. Sistema de Mundo y Territorio

## 1.1 Generación del mundo (Fase 0)
- Mapa CUADRADO, espacio de coordenadas continuo (no grid discreto).
- Tamaño base: 1000x1000 unidades, PARAMETRIZABLE.
- Terreno completamente plano en Fase 0: sin ríos, sin mar, sin relieve montañoso (pospuesto a fases posteriores).
- Generación PROCEDURAL de recursos por niveles de rareza:
  - Común (alta frecuencia, disperso): Madera (bosques, ver 1.4), Piedra, Trigo (vía fertilidad, ver 1.4).
  - Intermedio (frecuencia media, varios clusters): Cobre.
  - Raro (baja frecuencia, pocos clusters, espaciado mínimo forzado entre ellos): Estaño, Oro.
- Spawn de jugadores nuevos: posición aleatoria uniforme, INDEPENDIENTE de la ubicación de recursos.
- Fases avanzadas (fuera de alcance Fase 0): mapa con relieve, ríos, mar; posible mapa fijo diseñado a mano en vez de procedural; puntos de interés fijos (ruinas, maravillas); biomas/clima.

## 1.2 Fundación de asentamientos
- El jugador elige LIBREMENTE dónde colocar el edificio de fundación.
- Al fundar se genera automáticamente una ZONA DE INFLUENCIA (círculo/polígono) = nodo.
- TAMAÑO: nace con un radio inicial de 30 unidades y crece gradualmente cada tick hacia un TECHO que escala con el NIVEL del asentamiento (ver Doc 4.5 para el modelo de nivel): nivel 1 → 60, nivel 2 → 90, nivel 3 → 120 (tope de Fase 0, ver 4.5). Subir de nivel no hace saltar el radio de golpe — solo levanta el techo hacia el que la zona ya venía creciendo. NIVEL 4 Y 5: planeados a futuro (fuera del rango original de nivel de asentamiento pensado en diseño), pero NO IMPLEMENTADOS en Fase 0 — no tienen gates de población/edificios definidos ni techo de radio propio todavía. No inventar valores hasta que se definan junto con sus gates correspondientes en Doc 4.5.
- Reglas de construcción: solo se puede construir dentro de una zona de influencia existente (ampliándola si es del mismo bando) o fuera de cualquier zona (creando una nueva).
- FRONTERAS: al chocar dos zonas en expansión se genera un LÍMITE DURO — ninguna zona sigue creciendo en esa dirección. Solo se rompe si el asentamiento rival cae o su zona se debilita/reduce (guerra u otros medios). Las fronteras son "vivas", reflejan el poder relativo de cada bando en cada momento.

## 1.3 Onboarding de nuevos jugadores
- Jugador nuevo aparece en un punto ALEATORIO del mapa con una "caravana de asentamiento" para fundar donde decida.
- EDIFICIOS INICIALES: todo asentamiento nace con un Centro Urbano (marcador único, no construible por ningún otro medio), una Granja y 3 Viviendas, ya ACTIVOS sin pasar por la cola de construcción. Si hay un bosque alcanzable cerca del punto de fundación, se añade también una Leñera inicial (condicional, no garantizada si no hay bosque cercano) — reduce el riesgo de déficit de madera en los primeros ticks (ver 4.2 y `Correcciones_Durante_Desarrollo.md`, bug #1 del deadlock de madera).
- FUNDACIÓN GRUPAL: hasta 5 jugadores pueden organizarse para aparecer juntos en el mismo punto, compartiendo una caravana, fundando el asentamiento entre los 5. Los 5 reciben Ciudadanía de inmediato.
- MATERIALES INICIALES (confirmado durante implementación de Fase 0): la caravana de fundación entrega una reserva inicial de madera y piedra al fundar, suficiente para arrancar la primera construcción. Sin esto el asentamiento queda bloqueado permanentemente (el edificio que produce madera también cuesta madera para construirse — deadlock detectado y corregido en Sprint 2, ver `Correcciones_Durante_Desarrollo.md`).
- PROTECCIÓN TEMPORAL — RESUELTO durante implementación (ya no es pregunta pendiente): existe un período de gracia en ticks tras la fundación durante el cual NO se cobra Mantenimiento (ver Doc 4.5). Sin esto, todo asentamiento nuevo caía en ruinas de forma sistemática antes de que su economía pudiera arrancar. Duración exacta: placeholder ajustable.

## 1.4 Fuentes de recursos por tipo
- CULTIVOS (trigo): NO son un nodo recolectable directo. Dependen de la FERTILIDAD DEL SUELO de la zona (atributo de terreno); requieren construir una Granja para aprovecharse.
- MADERA: proviene de BOSQUES, representados como ZONAS del mapa (no puntos), con densidad variable.
- MINERALES (cobre, estaño, oro): en Fase 0, dado el terreno plano, están DISPERSOS aleatoriamente siguiendo las reglas de rareza de 1.1. En fases avanzadas (con relieve) estarán ligados a cordilleras/montañas sueltas, no dispersos al azar. Cada uno tiene su propio edificio de extracción (cantera para piedra, mina de oro, mina de cobre, mina de estaño — este último añadido en un rebalance posterior a Sprint 6: el estaño se generaba en el mundo desde el Sprint 1 pero no tenía forma de extraerse, solo de comerciarse).
- LIVESTOCK (ovejas, vacas, caballos + especies adicionales por definir): fauna LIBRE en el mapa, debe CAPTURARSE para aprovecharse. Cría/domesticación pospuesta a fases avanzadas (en Fase 0 se trata como recurso consumible/finito). Rendimientos: ovejas = carne+leche; vacas = carne+leche+cuero; caballos = fuente de entrenamiento de tropas montadas (caballería ligera, carros de guerra), no dan recurso de consumo.
- MADERA y PIEDRA: materiales base de construcción de edificios e insumo de materiales derivados.
- COMMODITIES DE NOBLEZA (uvas/olivas → vino/aceite): no son alimento básico, requeridas para felicidad de la Nobleza; su déficit arriesga rebelión/estancamiento, no hambruna.

## 1.5 Chokepoints estratégicos (heredado de Iberia, fases con relieve)
NO IMPLEMENTADO EN FASE 0 (el terreno de Fase 0 es plano, sin relieve — ver 1.1). Pasos de montaña, puentes, gargantas generan puntos de control natural donde una Facción que los domina militarmente puede cobrar peajes, escoltar caravanas aliadas o bloquear el suministro de una Facción/Liga rival entera. Aplica a partir de fases con relieve.

## 1.6 Caminos comerciales automáticos
- Al establecer una relación comercial entre asentamientos se genera AUTOMÁTICAMENTE un camino físico en el mapa (el jugador no lo construye manualmente).
- El camino AFECTA LA VELOCIDAD de caravanas/tropas que lo recorren (más rápido que campo abierto).
- MEJORABLE vía Políticas.
- PENDIENTE: qué pasa con el camino si se rompe la relación comercial que lo originó.

## 1.7 Cap de fundación de asentamientos por Facción (inspirado en Rise of Nations)
- Límite DURO de asentamientos que una Facción puede FUNDAR (no aplica a conquista/anexión, que no tiene límite).
- ESCALA con el NIVEL DE FACCIÓN: progresión fácil de 1 a 3, luego se complica progresivamente hasta un MÁXIMO DE 7 en etapa tardía.
- Complementa (no sustituye) al sistema de Mantenimiento/Coste de Gobernanza (ver Doc 4).
- Da incentivo mecánico a preferir vasallaje/conquista sobre fundación directa una vez alcanzado el cap.
- ESTADO DE IMPLEMENTACIÓN: sin verificar. No aparece registrado en `Correcciones_Durante_Desarrollo.md` pese a estar en el alcance del Sprint 4 — no hay evidencia de que se haya probado en simulación real, a diferencia del resto de sistemas de ese sprint.
- PENDIENTE: qué hace subir exactamente el "nivel de Facción"; curva numérica exacta entre cap 3 y cap 7; verificar implementación real.

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
