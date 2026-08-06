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
- Reglas de construcción: solo se puede construir dentro de una zona de influencia existente (ampliándola si es del mismo bando) o fuera de cualquier zona (creando una nueva).
- FRONTERAS: al chocar dos zonas en expansión se genera un LÍMITE DURO — ninguna zona sigue creciendo en esa dirección. Solo se rompe si el asentamiento rival cae o su zona se debilita/reduce (guerra u otros medios). Las fronteras son "vivas", reflejan el poder relativo de cada bando en cada momento.

## 1.3 Onboarding de nuevos jugadores
- Jugador nuevo aparece en un punto ALEATORIO del mapa con una "caravana de asentamiento" para fundar donde decida.
- FUNDACIÓN GRUPAL: hasta 5 jugadores pueden organizarse para aparecer juntos en el mismo punto, compartiendo una caravana, fundando el asentamiento entre los 5. Los 5 reciben Ciudadanía de inmediato.
- MATERIALES INICIALES (confirmado durante implementación de Fase 0): la caravana de fundación entrega una reserva inicial de madera y piedra al fundar, suficiente para arrancar la primera construcción. Sin esto el asentamiento queda bloqueado permanentemente (el edificio que produce madera también cuesta madera para construirse — deadlock detectado y corregido en Sprint 2, ver `Correcciones_Durante_Desarrollo.md`).
- PROTECCIÓN TEMPORAL — RESUELTO durante implementación (ya no es pregunta pendiente): existe un período de gracia en ticks tras la fundación durante el cual NO se cobra Mantenimiento (ver Doc 4.5). Sin esto, todo asentamiento nuevo caía en ruinas de forma sistemática antes de que su economía pudiera arrancar. Duración exacta: placeholder ajustable.

## 1.4 Fuentes de recursos por tipo
- CULTIVOS (trigo): NO son un nodo recolectable directo. Dependen de la FERTILIDAD DEL SUELO de la zona (atributo de terreno); requieren construir una Granja para aprovecharse.
- MADERA: proviene de BOSQUES, representados como ZONAS del mapa (no puntos), con densidad variable.
- MINERALES (cobre, estaño, oro): en Fase 0, dado el terreno plano, están DISPERSOS aleatoriamente siguiendo las reglas de rareza de 1.1. En fases avanzadas (con relieve) estarán ligados a cordilleras/montañas sueltas, no dispersos al azar.
- LIVESTOCK (ovejas, vacas, caballos + especies adicionales por definir): fauna LIBRE en el mapa, debe CAPTURARSE para aprovecharse. Cría/domesticación pospuesta a fases avanzadas (en Fase 0 se trata como recurso consumible/finito). Rendimientos: ovejas = carne+leche; vacas = carne+leche+cuero; caballos = fuente de entrenamiento de tropas montadas (caballería ligera, carros de guerra), no dan recurso de consumo.
- MADERA y PIEDRA: materiales base de construcción de edificios e insumo de materiales derivados.
- COMMODITIES DE NOBLEZA (uvas/olivas → vino/aceite): no son alimento básico, requeridas para felicidad de la Nobleza; su déficit arriesga rebelión/estancamiento, no hambruna.

## 1.5 Chokepoints estratégicos (heredado de Iberia, fases con relieve)
Pasos de montaña, puentes, gargantas generan puntos de control natural donde una Facción que los domina militarmente puede cobrar peajes, escoltar caravanas aliadas o bloquear el suministro de una Facción/Liga rival entera.

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
- PENDIENTE: qué hace subir exactamente el "nivel de Facción"; curva numérica exacta entre cap 3 y cap 7.
