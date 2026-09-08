# Preguntas y Huecos por Definir

Lo que sigue sin resolver del todo. La mayoría son ajustes numéricos/de balance, no decisiones de diseño pendientes — usar valores placeholder razonables donde se necesiten para implementar, y ajustar después.

## 1. Guerra y combate — mayormente [RESUELTO]
[RESUELTO] (rediseño Fase 0, progreso de asentamientos) — IMPLEMENTADO: reclutamiento pasa de tiers genéricos a tropas específicas reclutadas en Barracón/Galería de tiro según el nivel interno del edificio (1-3), con costo en equipo fabricado en Armería (ver Doc 5.7/5.8, `TROPAS_RECLUTABLES`); Nobleza sin cambios (vía Gran Fundición). [RESUELTO Y LIMPIADO, a petición del usuario — fix de tropas]: una tropa NUNCA cambia de identidad al ganar veteranía — solo gana poder dentro del mismo `tropaId`. El antiguo sistema de ascenso de tier por veteranía (`ASCENSO_TROPA`, `TROPA_CATALOGO`, `ascenderTierSiCorresponde`) era código muerto (nunca se ejecutaba) y se retiró por completo del código, junto con el campo `Escuadron.tier` (ver Doc 5.8).

Pendiente:
- ¿Cómo se declara una guerra? ¿Requiere condición previa (frontera compartida, casus belli) o es libre?
- ~~¿Cómo se conquista un asentamiento enemigo exactamente tras ganar el asedio?~~ **[RESUELTO 2026-09-08]** Captura directa (reasignación de Facción) + **ocupación post-conquista**: el ejército conquistador se vuelve la guarnición, saqueo determinista de población/edificios/murallas, ventana de ocupación de tiempo fijo (inmune a nuevo asedio, recaudación y crecimiento a la mitad, mantenimiento congelado). Canon en Doc 5.12.9; diseño y plan en `Ocupacion_Post_Conquista_Definicion.md`.
- ¿Existen unidades navales de combate?
- Ventana horaria exacta del Attack Timer (pospuesto, no bloqueante para Fase 0)
- Establos / carros de guerra del roster anterior: sin edificio de reclutamiento definido en el rediseño — retirar de Fase 0 o crear edificio propio
- `poderBase` de las 10 tropas nuevas es placeholder (interpolado del roster anterior), pendiente de calibración por simulación
- Añadida una 11ª tropa, `milicia_lanceros` (Barracón nv1, `madera: 2`, sin metalurgia — ver Correcciones #32), y una receta `armaMadera` en Armería (`produccionBase: 2` / `madera: 2` por unidad): ambas cifras son placeholder conservador (verificado que no degradan el colapso general), no una calibración fina

## 2. Diplomacia y alianzas — cerrado (mayormente)
Pendiente (menor): valores numéricos exactos de cada evento que mueve el score de reputación, velocidad del decaimiento, umbrales exactos de cada uso.

## 3. Generación del mundo — [RESUELTO] para Fase 0
[RESUELTO] (Fase 0.3): movimiento de caravana sensible al terreno (coste continuo por tipo de terreno, ver `worldgen/costeMovimiento.ts`), pathfinding compartido (`world/rutas.ts`), caminos comerciales automáticos (Doc 1.6) y chokepoints estratégicos como puertos de montaña con control por zona de influencia + peaje en oro (Doc 1.5) — ver `Consideraciones/Fase_0_3_Definicion.md`. [RESUELTO] también (parcialmente) la pregunta pendiente sobre caminos comerciales: si se rompe la relación que originó un camino, el camino queda como infraestructura física permanente (no se elimina) — decisión pragmática sin validar por simulación.

Pendiente (fases posteriores): mapa fijo vs. procedural con relieve; puntos de interés fijos; distribución de climas/biomas; especies adicionales de livestock.

Pendiente (Fase 0.3, nuevo):
- Bloqueo/escolta militar de chokepoints (Doc 1.5): solo se implementó el peaje — el bloqueo necesita un concepto de "guerra activa" entre Facciones que no existe hoy (el combate es cálculo puntual, no presencia física continua en el mapa).
- Vados de río como chokepoint: esta pasada solo detecta puertos de montaña (puntos de silla del campo de elevación); un vado necesitaría su propio criterio de detección sobre `RioZona`.
- Calibración por simulación de `COSTE_MOVIMIENTO` (coste por tipo de terreno, bonus de camino) y `CHOKEPOINTS_PEAJE.oro`: valores placeholder, igual que el resto de cifras nuevas del proyecto.

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

## 8. Nuevo jugador / curva de aprendizaje — [RESUELTO] (durante implementación)
[RESUELTO]: protección temporal para asentamientos recién fundados = período de gracia en ticks durante el cual no se cobra Mantenimiento (ver Doc 1.3 y Doc 4.5). Se descubrió como necesidad real al implementar: sin esto, todo asentamiento nuevo caía en ruinas sistemáticamente en pocos ticks. Duración exacta del período sigue siendo placeholder ajustable. Ver `Correcciones_Durante_Desarrollo.md` para el detalle del bug que lo originó.

## 9. Identidad visual y de audio — sin abordar
- Referencias estéticas concretas (micénica, hitita, mesopotámica...)

## 10. Recursos y cadenas de producción
[RESUELTO] (parcialmente): qué pasa en déficit sostenido de un asentamiento — ver sistema unificado de Mantenimiento (medidor 0-100, coste escalonado de madera/piedra/oro según nivel, degradación proporcional, destrucción a 0; umbral de oro ahora en nivel 8, no 6). [RESUELTO]: agotamiento de recursos finitos (cantera/mina de oro/mina de cobre/mina de estaño/lenera) — se reemplazan automáticamente hasta un máximo ligado al nivel del asentamiento (ver Doc 4.2 y `Correcciones_Durante_Desarrollo.md`). [RESUELTO] (rebalance posterior a Sprint 6): la auto-construcción priorizaba mal Vivienda/Granja/Cantera por delante de la Leñera y podía copar la cola de construcción, dejando la única fuente de madera sin forma de construirse — corregido con reordenamiento de prioridades, un slot de cola reservado para recursos de supervivencia (Granja/Leñera) y una reserva mínima de recursos que la construcción nunca puede tocar mientras Mantenimiento los necesite (ver Doc 4.2/4.5 y `Correcciones_Durante_Desarrollo.md`). [RESUELTO] (post rediseño de progreso): esa priorización solo cubría qué se ENCOLA — el orden en que se GASTAN los recursos al arrancar construcciones seguía siendo por inserción, permitiendo que Curtiduría/Armería se llevaran la madera antes que una Granja/Leñera nueva; corregido resolviendo el arranque por prioridad de categoría en vez de orden de inserción (ver Doc 4.2 y `Correcciones_Durante_Desarrollo.md` #21). [RESUELTO]: el estaño no tenía edificio de extracción propio — añadida `minaEstano`, mismo patrón que cantera/mina/minaCobre. [RESUELTO]: el trigo tenía una "mecánica repetida" — Mantenimiento cobraba un valor fijo de trigo ADEMÁS del consumo real de comida de población+tropas ya descontado por separado; ahora Mantenimiento no cobra trigo directamente (solo madera/piedra/oro) y el "apartado de trigo" mostrado es la suma real de ambos consumos, descontada una sola vez donde siempre se descontó — ver Doc 4.5 y `Correcciones_Durante_Desarrollo.md` #22. [RESUELTO] (a petición del usuario): el disparador de Granja pasó de "reserva de ticks estimada" a comparar producción real contra consumo real (déficit = producción<consumo), permite hasta 3 Granjas a la vez en déficit (antes 1), y se añadió la política "Edicto de Cosecha" (Gobernador, ×1.5 trigo) — producción base de Granja recalibrada dos veces en el proceso (5→10→15 trigo/tick). [RESUELTO]: bug real donde el alcance de un bosque para Leñera solo comprobaba si su CENTRO caía dentro de la zona de influencia, no si el bosque simplemente se solapaba con ella — un bosque grande con el borde ya dentro de la zona pero el centro fuera del tope de radio por nivel quedaba inalcanzable para siempre, y el asentamiento colapsaba sin madera; corregido para aceptar cualquier punto del bosque dentro de la zona. [RESUELTO] también (a petición del usuario): un bosque grande ahora admite más de una Leñera trabajándolo, según su radio (1-3 Leñeras por bosque) — ver `Correcciones_Durante_Desarrollo.md`.

Pendiente:
- ¿Más cultivos/ganado además de lo confirmado?
- ¿Cuántos pasos de procesamiento tienen las cadenas de producción?
- Cantidades exactas finales de mantenimiento por nivel, velocidad exacta de degradación/regeneración, duración exacta del período de gracia inicial (validado como jugable en pruebas de 150-300 ticks bajo el modelo de nivel anterior — pendiente de repetir la validación tras el rediseño de nivel de asentamiento, ver 10b)
- Desajuste estructural entre población exponencial (12%/tick compuesto, sin techo salvo Vivienda) y producción de Granja con techo fijo (`ratioManoObra` no supera 1, más población no aumenta la producción): recalibrar `produccionBaseTrigo` (5→10→15), pasar el disparador a déficit real (hasta 3 Granjas a la vez), y añadir "Edicto de Cosecha" (×1.5) mitigan bastante — asentamientos con fertilidad muy baja podrían seguir entrando en déficit en algún punto, sin recalibración exhaustiva por simulación todavía. Posible solución de fondo sin explorar: producción escalable directamente con la población (no solo con `ratioManoObra`).

## 10b. Nivel de asentamiento — [RESUELTO] (para Fase 0, rediseño de progreso)
[RESUELTO]: modelo por gates en vez de fórmula de puntos continua. Tope de Fase 0 = nivel 3. Nivel 2 = 200 pesants + 50 artesanos + Armería/Curtiduría/Fundición construidas; Nivel 3 = 500 pesants + 200 artesanos + Carpintería/Barracón/Galería de tiro construidas (ver Doc 4.5 y Doc 4.2.1 para el catálogo completo de edificios nuevos). [RESUELTO] también: todos los requisitos de "Planos de X" (vía Aedas) se descartan en Fase 0 (Doc 6.5) — solo aplican los gates de nivel de asentamiento y edificio previo.

[RESUELTO] (calibrado por simulación durante implementación, ver `Correcciones_Durante_Desarrollo.md` #16-18): el tope de extractores por tipo se probó en 5 y provocó que TODO asentamiento simulado (150-600 ticks, con y sin piedra cercana) colapsara por déficit de Mantenimiento entre el tick 74 y el 674 — la extracción capada en 5 no alcanzaba a sostener la tasa de crecimiento de Pesants ya existente (12%/tick, sin tope salvo Vivienda). Subido a **10** (mismo techo que permitía el nivel máximo anterior de 10). También se detectaron y corrigieron dos interbloqueos reales de cola durante la calibración (Curtiduría/Armería podían dejar a Cantera y luego a Vivienda sin hueco para encolarse nunca).

Pendiente:
- Cifra exacta en qué nivel (1-3) empieza a exigirse piedra y en qué nivel oro en Mantenimiento (antes nivel 3/nivel 8 sobre un rango 1-10, recalibrado a rango 1-3 pero sin cifra cerrada).
- Tope de extractores en 10: placeholder razonable por simulación (ver arriba), no una calibración exhaustiva — puede seguir ajustándose. -> Error a revisar 
- Costo/tiempo/trabajadores de construcción de Carpintería (el diseño original solo da el gate de nivel, sin cifras).
- Si existirán niveles 4+ de asentamiento en fases posteriores a Fase 0, y con qué requisitos.

## 11. Población: clases sociales — cerrado
[RESUELTO] (rediseño Fase 0): disparador y tope de Artesanos. Se retira el edificio genérico "Taller"; dispara el primero que se construya entre Curtiduría/Armería/Fundición/Carpintería, y el tope de población pasa a ser la suma de trabajadoresRequeridos de los edificios de transformación activos (ver Doc 4.1/4.2.1).

Pendiente (menor):
- Fórmula exacta de crecimiento de Artesanos (la tasa en sí, no el disparador/tope — eso ya quedó [RESUELTO] arriba)
- Criterio exacto de la cola de prioridad de reclutamiento

## 12. Políticas — cerrado (mayormente)
[RESUELTO] (parcialmente, ejemplos ilustrativos no exhaustivos): hay un puñado de políticas concretas implementadas con efecto mecánico real, no solo flags — incluye "Protección de Riesgos" (Maestro de Obras, rebalance posterior a Sprint 6, ampliada a petición del usuario a 2 Leñeras + 3 Granjas), que fuerza a la auto-construcción a priorizar esos edificios hasta cumplir ambos mínimos antes de construir cualquier otra cosa; y "Edicto de Cosecha" (Gobernador, nueva, a petición del usuario), que sube ×1.5 la producción de trigo de todas las Granjas (ver Doc 4.2/4.4/4.5).

Pendiente:
- Catálogo concreto de políticas dentro de cada pool (más allá de los ejemplos ya implementados)
- Si son excluyentes entre sí dentro de un mismo slot

## 13. Cap de asentamientos y fusión de Facciones — cerrado (mayormente)
Pendiente (menor):
- Qué hace subir exactamente el "nivel de Facción"; curva/números exactos entre cap 3 y cap 7
- ¿Se requiere aceptación mutua explícita para fusión/anexión, o la Opción 1 se puede forzar unilateralmente?

## 14b. Control manual de cola de construcción — [RESUELTO] e IMPLEMENTADO en motor Y en interfaz (cambio de base, a petición del usuario)
[RESUELTO]: Gobernador y Maestro de Obras ven, reordenan, añaden y quitan proyectos de la cola — nunca eligen ubicación (ver Doc 2.2/4.2). [RESUELTO] también: las 4 políticas "Construir Barracón/Galería de tiro/Palacio/Mercado" y su cluster de cola aparte se RETIRARON por completo — esos 4 edificios pasan al mismo carril de adición manual que cualquier otro edificio del catálogo, sin gate de política. Añadir respeta los mismos gates de nivel/edificio previo del catálogo y la reserva mínima de Mantenimiento. Quitar solo aplica a `en_cola` (nunca `en_construccion`) y devuelve el costo completo pagado. Implementado en `engine/construction.ts` (`anadirEdificioManualmente`, `quitarDeCola`, `moverEnCola`) y expuesto por completo en la pestaña Asentamientos de la interfaz: tabla de cola con botones ▲/▼/Quitar, selector de cargo + tipo + botón "Añadir a la cola", y segmento "Info:" con costo/tiempo/requisitos del tipo seleccionado antes de confirmar (`CATALOGOS.catalogoEdificios` en `app/gameStore.ts`). Verificado en el navegador de punta a punta, sin errores de consola.

Pendiente (menor):
- Qué pasa con los recursos ya comprometidos de un proyecto `en_construccion` si se necesitara cancelarlo a mitad de obra (fuera de alcance: `en_construccion` no se puede cancelar, decisión ya cerrada).

## 14c. Campamentos de bandidos — [RESUELTO] e IMPLEMENTADO (a petición del usuario)
[RESUELTO] e IMPLEMENTADO: spawnean en bosques no reclamados (sin zona de influencia encima), atacan caravanas mientras siguen en pie, dan recompensa al destruirse (acción manual del jugador, pestaña Guerra), reaparecen pasados N ticks (ver Doc 1.9). [RESUELTO] también (rediseño a petición del usuario, corrige el placeholder inicial de "1 campamento fijo en todo el mundo, posición aleatoria"): el tope pasa a ser UNO por asentamiento vivo, apareciendo en SU bosque no reclamado MÁS CERCANO — nunca en la otra punta del mapa sin nadie cerca, nunca dentro de una zona de influencia. Implementado en `engine/bandidos.ts` (spawn/respawn + ataque a caravanas, automáticos cada tick) y `engine/combate.ts` (`atacarCampamentoBandidos`, manual). Verificado en el navegador con 2 asentamientos en esquinas opuestas: cada uno recibió su propio campamento cercano (69 y 103 unidades respectivamente), tope respetado en 2, reaparición exacta en el tick esperado.

Pendiente (calibración por simulación, no diseño):
- Poder de combate del campamento (placeholder: 30, fijo — sin escalado por región).
- Composición/tamaño de la recompensa (placeholder: 40 madera + 20 piedra + 15 oro).
- Cadencia exacta de reaparición (placeholder: 60 ticks).
- Radio de cobertura que decide cuándo un asentamiento ya está "atendido" (placeholder: 600, sin techo máximo real de distancia).

## 14d. Ciclo de servidor y Maravilla — EL EDIFICIO IMPLEMENTADO, el ciclo sigue sin implementar (a petición del usuario)
[RESUELTO] la decisión de diseño: ciclo de 12 meses, cerrado antes de tiempo por la primera Facción que complete la Maravilla del ciclo (coste extremo, solo en asentamiento de nivel máximo, materiales conocidos + exóticos); al resetear, la Facción ganadora permanece como Facción-legado NPC de solo mantenimiento y comercio (ver `Roadmap_Escalado.md` Eje 4).

[IMPLEMENTADO, a petición del usuario — "solo la parte básica"]: el edificio Maravilla en sí, SIN el ciclo/reset/legado. Añadido a `EDIFICIO_CATALOGO` (constants.ts), único, nivel de asentamiento 3 requerido, disponible vía control manual de cola (Doc 4.2). Coste PLACEHOLDER con recursos ya existentes (5000 madera + 5000 piedra + 500 oro + 300 cobre + 200 estaño + 200 livestock, 200 ticks) — los materiales EXÓTICOS del diseño original NO están implementados (no existe ese tipo de recurso todavía en el juego). Verificado en el navegador: Info correcta en el selector, rechazo correcto por nivel de asentamiento insuficiente.

Pendiente: TODO lo demás del ciclo — ver lista completa en `Roadmap_Escalado.md` Eje 4 (catálogo de materiales exóticos, que la Maravilla cambie de ciclo a ciclo, si el timer cierra el ciclo sin ganador, destino de las Facciones no ganadoras, si la Facción-legado es atacable, posición en el nuevo mapa, cómo se genera la Maravilla de cada ciclo, y la infraestructura de servidor/reset en sí — nada de esto tiene código todavía).

## 14e. Curva de progresión inicial / onboarding (nuevo, riesgo detectado en análisis comparativo con Travian)
El usuario confirma la necesidad: el inicio del juego debe ser SUAVE, con sistemas desbloqueándose progresivamente en vez de exponer toda la profundidad (7 cargos, Liga/vasallaje, reputación, gremios, chokepoints...) desde el primer tick — a diferencia de Travian, que resuelve el día 1 en dos acciones (construir, atacar oasis).

Sin resolver todavía — es una decisión de diseño real pendiente, no solo un ajuste numérico:
- Qué sistemas se desbloquean primero y cuáles se difieren (candidatos naturales para diferir: Liga/vasallaje, gremios, chokepoints, fusión/anexión — todos ya dependen de condiciones que tardan en cumplirse por sí mismas, pero no está confirmado si eso basta o hace falta un gate explícito).
- Si el desbloqueo se ata al NIVEL DE ASENTAMIENTO (1-3, Doc 4.5), al NIVEL DE FACCIÓN (1-7, Doc 1.7), a tiempo transcurrido, o a una combinación.
- Si aplica solo a la INTERFAZ (ocultar opciones no relevantes todavía) o también a las REGLAS (bloquear mecánicamente el acceso).

## 14. Gremios (edificios especiales)
[RESUELTO] (parcialmente): 4 gremios (Comerciantes, Artesanos, Constructores, Ladrones), edificios escasos a nivel de servidor. Disparador por tirada periódica mientras se cumplan 3 requisitos (score de reputación >90, título de servidor específico, nivel/mantenimiento del asentamiento >90%). Se pierden si se incumple alguna condición. Gremio de Ladrones confirmado: info de acuerdos comerciales/caravanas/Facciones ajenas, acotado para no ser desequilibrante (NO revive el sistema de rumores general, que sigue descartado). Ver Doc 2.10.

Pendiente:
- Valores numéricos exactos de cada requisito por gremio
- Título de servidor específico asociado a cada gremio
- Detalle exacto de beneficios de Comerciantes, Artesanos y Constructores
- Duración del margen de gracia antes de perder el gremio (si existe)
