# Preguntas y Huecos por Definir

Lo que sigue sin resolver del todo. La mayoría son ajustes numéricos/de balance, no decisiones de diseño pendientes — usar valores placeholder razonables donde se necesiten para implementar, y ajustar después.

## 1. Guerra y combate — mayormente resuelto
Pendiente:
- ¿Cómo se declara una guerra? ¿Requiere condición previa (frontera compartida, casus belli) o es libre?
- ¿Cómo se conquista un asentamiento enemigo exactamente tras ganar el asedio? ¿Se captura, se destruye, se vasalliza automáticamente?
- ¿Existen unidades navales de combate?
- Ventana horaria exacta del Attack Timer (pospuesto, no bloqueante para Fase 0)

## 2. Diplomacia y alianzas — cerrado (mayormente)
Pendiente (menor): valores numéricos exactos de cada evento que mueve el score de reputación, velocidad del decaimiento, umbrales exactos de cada uso.

## 3. Generación del mundo — resuelto para Fase 0
Pendiente (fases posteriores): mapa fijo vs. procedural con relieve; puntos de interés fijos; distribución de climas/biomas; especies adicionales de livestock; qué pasa con un camino comercial si se rompe la relación que lo originó.

## 4. Exploración — sin abordar
- ¿Niebla de guerra / mapa oculto hasta explorarlo?
- ¿NPCs hostiles (bandidos, fauna peligrosa)?
- ¿Mecánica propia de exploración (unidad exploradora, coste, riesgo)?

## 5. Economía interna — cerrado
Sin preguntas pendientes.

## 5b. Cargos, Facción, Liga y Vasallaje — cerrado (mayormente)
Pendiente (menor):
- Definir "desarme" de una Facción con precisión (más allá de destrucción total)
- Qué pasa con votación de Rey-por-alianza si solo hay 2 Facciones votantes (empate más probable que excepción)
- Mecanismo de sucesión de un Rey-por-vasallaje si el jugador abandona

## 5c. Comisiones de comercio
- ¿En qué se puede usar la riqueza acumulada por comisiones?
- ¿Fórmula/porcentaje concreto de la comisión (interna / aliada-vasalla / externa)?

## 6. Progresión de imperio (etapa tardía)
Pendiente:
- ¿Qué diferencia mecánicamente a un "imperio" de una alianza grande?
- ¿Qué pasa cuando un imperio se fragmenta? (mecanismo concreto)
- Lista completa de títulos disponibles

## 7. Interfaz de decisiones del jugador — cerrado
Sin preguntas pendientes.

## 8. Nuevo jugador / curva de aprendizaje — RESUELTO (durante implementación)
RESUELTO: protección temporal para asentamientos recién fundados = período de gracia en ticks durante el cual no se cobra Mantenimiento (ver Doc 1.3 y Doc 4.5). Se descubrió como necesidad real al implementar: sin esto, todo asentamiento nuevo caía en ruinas sistemáticamente en pocos ticks. Duración exacta del período sigue siendo placeholder ajustable. Ver `Correcciones_Durante_Desarrollo.md` para el detalle del bug que lo originó.

## 9. Identidad visual y de audio — sin abordar
- Referencias estéticas concretas (micénica, hitita, mesopotámica...)

## 10. Recursos y cadenas de producción
RESUELTO (parcialmente): qué pasa en déficit sostenido de un asentamiento — ver sistema unificado de Mantenimiento (medidor 0-100, coste escalonado de madera/comida/piedra/oro según nivel, degradación proporcional, destrucción a 0; umbral de oro ahora en nivel 8, no 6). RESUELTO: agotamiento de recursos finitos (cantera/mina de oro/mina de cobre/mina de estaño/lenera) — se reemplazan automáticamente hasta un máximo ligado al nivel del asentamiento (ver Doc 4.2 y `Correcciones_Durante_Desarrollo.md`). RESUELTO (rebalance posterior a Sprint 6): la auto-construcción priorizaba mal Vivienda/Granja/Cantera por delante de la Leñera y podía copar la cola de construcción, dejando la única fuente de madera sin forma de construirse — corregido con reordenamiento de prioridades, un slot de cola reservado para recursos de supervivencia (Granja/Leñera) y una reserva mínima de recursos que la construcción nunca puede tocar mientras Mantenimiento los necesite (ver Doc 4.2/4.5 y `Correcciones_Durante_Desarrollo.md`). RESUELTO: el estaño no tenía edificio de extracción propio — añadida `minaEstano`, mismo patrón que cantera/mina/minaCobre.

Pendiente:
- ¿Más cultivos/ganado además de lo confirmado?
- ¿Cuántos pasos de procesamiento tienen las cadenas de producción?
- Cantidades exactas finales de mantenimiento por nivel, velocidad exacta de degradación/regeneración, duración exacta del período de gracia inicial (validado como jugable en pruebas de 150-300 ticks, pero sigue siendo placeholder)

## 11. Población: clases sociales — cerrado
Pendiente (menor):
- Fórmula exacta de crecimiento de Artesanos
- Criterio exacto de la cola de prioridad de reclutamiento

## 12. Políticas — cerrado (mayormente)
RESUELTO (parcialmente, ejemplos ilustrativos no exhaustivos): hay un puñado de políticas concretas implementadas con efecto mecánico real, no solo flags — incluye "Protección de Riesgos" (Maestro de Obras, rebalance posterior a Sprint 6), que fuerza a la auto-construcción a priorizar Leñeras hasta un mínimo antes de construir cualquier otra cosa (ver Doc 4.2/4.4).

Pendiente:
- Catálogo concreto de políticas dentro de cada pool (más allá de los ejemplos ya implementados)
- Si son excluyentes entre sí dentro de un mismo slot

## 13. Cap de asentamientos y fusión de Facciones — cerrado (mayormente)
Pendiente (menor):
- Qué hace subir exactamente el "nivel de Facción"; curva/números exactos entre cap 3 y cap 7
- ¿Se requiere aceptación mutua explícita para fusión/anexión, o la Opción 1 se puede forzar unilateralmente?

## 14. Gremios (edificios especiales)
RESUELTO (parcialmente): 4 gremios (Comerciantes, Artesanos, Constructores, Ladrones), edificios escasos a nivel de servidor. Disparador por tirada periódica mientras se cumplan 3 requisitos (score de reputación >90, título de servidor específico, nivel/mantenimiento del asentamiento >90%). Se pierden si se incumple alguna condición. Gremio de Ladrones confirmado: info de acuerdos comerciales/caravanas/Facciones ajenas, acotado para no ser desequilibrante (NO revive el sistema de rumores general, que sigue descartado). Ver Doc 2.10.

Pendiente:
- Valores numéricos exactos de cada requisito por gremio
- Título de servidor específico asociado a cada gremio
- Detalle exacto de beneficios de Comerciantes, Artesanos y Constructores
- Duración del margen de gracia antes de perder el gremio (si existe)
