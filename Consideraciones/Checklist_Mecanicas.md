# Checklist de Mecánicas

Lista completa de mecánicas diseñadas, organizadas por sistema. ✅ = cerrado/decidido. 🔶 = en discusión, con hueco pendiente. ❌ = descartado explícitamente.

**ESTADO:** Fase 0 (Sprints 1-6) ya fue **implementada** en TypeScript y validada jugando en el navegador. Ver `Correcciones_Durante_Desarrollo.md` para los ajustes de diseño que surgieron durante la implementación (varios resolvieron preguntas antes pendientes).

**NOTA (rediseño de progreso y arranque, post Sprint 6):** el nivel de asentamiento, el catálogo de edificios, el disparador de Artesanos y el reclutamiento militar fueron rediseñados y documentados en Docs/1, 4 y 5 (ver detalle en cada sección de abajo) — la implementación en código (`src/`) de este rediseño es un paso posterior, todavía PENDIENTE.

Ver `Docs/0_Glosario_de_Entidades_Politicas.md` para las definiciones centrales.

## Concepto y ambientación
- ✅ Ambientación: Edad Oscura de la Edad de Bronce
- ✅ Mapa enorme y desconectado al inicio, crecimiento deliberadamente lento

## Fundación y zonas de influencia
- ✅ Fundación libre, zona de influencia automática, reglas de construcción, fronteras con límite duro
- ✅ Onboarding: spawn aleatorio + fundación grupal (hasta 5 jugadores)
- ✅ Chokepoints estratégicos (fases con relieve) — IMPLEMENTADO en Fase 0.3: puertos de montaña detectados por punto de silla del campo de elevación, control por zona de influencia + peaje en oro a Facciones rivales (ver `Fase_0_3_Definicion.md`); bloqueo/escolta militar sigue pendiente (necesita concepto de guerra activa)
- ✅ Cap de fundación por Facción (escala con nivel de Facción, 1→3 fácil, hasta 7 tardío)
- ✅ Protección temporal: período de gracia sin cobro de Mantenimiento al fundar (RESUELTO durante implementación Fase 0)
- ✅ Rediseño Fase 0: radio inicial de zona de influencia sube a 30, techo escala con el nivel de asentamiento (60/90/120); Leñera inicial condicional a bosque cercano (ver Doc 1.2/1.3)
- 🔶 Qué hace subir el "nivel de Facción" exactamente; curva exacta entre cap 3 y cap 7 (distinto del nivel de ASENTAMIENTO, ver sección Mantenimiento — ese sí quedó resuelto para Fase 0)
- ✅ Caravana de Fundación: mecanismo complementario al cap, coste = materiales iniciales + edificios de arranque + 50 madera, gates de nivel 2 + poder pagar, manual (no automática como trueque/mercado), interceptable igual que caravana comercial, desarmable con recuperación completa antes de fundar (sometido a consejo LLM)

## Tecnología
- ✅ Tres vías de acceso (comercio, desarrollo propio, Aedas); sistema de Aedas completo; tecnología se compra con oro
- 🔶 Árbol tecnológico del desarrollo propio: estructurado o libre/emergente

## Comercio y economía
- ✅ Comercio siempre manual; caravanas terrestres; comercio marítimo (fuera de Fase 0)
- ✅ Economía dual (oro + trueque); oro = metal precioso, origen en minas
- ✅ Acuerdos de trueque; órdenes de mercado; precios dinámicos; comisiones más bajas intra-Facción
- ✅ Categorías de caravana; transporte individual; bonificación por distancia; dependencia logística
- ✅ Caminos comerciales automáticos — IMPLEMENTADO en Fase 0.3: se generan al proponer trueque entre dos asentamientos (pathfinding que evita terreno costoso), dan bonus de velocidad a las caravanas que los siguen (ver `Fase_0_3_Definicion.md`); qué pasa si se rompe la relación que lo originó sigue pendiente (el camino queda como infraestructura permanente por ahora)
- 🔶 Uso de riqueza por comisiones; fórmula exacta de comisión

## Recursos
- ✅ Recursos desiguales en el mapa; generación Fase 0 (mapa cuadrado 1000x1000 parametrizable, plano)
- ✅ Fuentes diferenciadas (cultivos por fertilidad+granja, madera por bosques-zona, minerales dispersos)
- ✅ Comida básica vs commodities de nobleza; madera/piedra; livestock capturable
- ✅ Cadenas de producción invisibles; almacenamiento con límites
- 🔶 Lista completa de cultivos/livestock; pasos de procesamiento de cadenas; déficit sostenido (RESUELTO vía Mantenimiento)

## Población
- ✅ 3 clases (Pesants, Artesanos, Nobleza) con roles, condiciones de aparición y fórmulas propias
- ✅ Jugadores como entidad separada; reclutamiento por pool específico (combate real vs progresión plana)
- ✅ Rediseño Fase 0: disparador de Artesanos pasa de Taller genérico a "el primero construido entre Curtiduría/Armería/Fundición/Carpintería"; tope de población = suma de trabajadoresRequeridos de los edificios de transformación activos (ver Doc 4.1/4.2.1)
- 🔶 Fórmula exacta de crecimiento de Artesanos; criterio exacto de cola de prioridad de reclutamiento

## Construcción automática
- ✅ Jugador no elige ubicación/tipo (excepto fundación y edificios estratégicos)
- ✅ Algoritmo por reglas; crecimiento por necesidad, con reevaluación CONTINUA (no solo "construir una vez"): granjas adicionales si hay déficit; Maestro de Obras; layout dinámico por política
- ✅ Reemplazo automático de extractores (cantera/mina de oro/mina de cobre/mina de estaño/lenera/Corral) cuando su fuente se agota, hasta un máximo por tipo — rediseño Fase 0: desacoplado del nivel del asentamiento (antes escalaba 1:1 con él), ahora número fijo pendiente de calibración (ver Doc 4.2)
- ✅ Rediseño Fase 0: catálogo ampliado con 8 edificios nuevos (Corral, Armería, Curtiduría, Carpintería, Palacio, Barracón, Galería de tiro; Murallas sigue fuera de alcance) con recetas multi-nivel — ver Doc 4.2.1. Curtiduría/Armería/Fundición/Carpintería son auto-construcción; Barracón/Galería de tiro/Palacio van vía política dedicada con cluster de cola aparte (ver Doc 4.4)
- ✅ Cola de PRIORIDAD al gastar recursos (fix post rediseño de progreso): el arranque de construcciones `en_cola` se resuelve por categoría (supervivencia > extractores > general), no por orden de inserción — antes Curtiduría/Armería podían llevarse la madera disponible antes que una Granja/Leñera nueva encolada más tarde, pese a haber recursos de sobra para ambas (ver Doc 4.2 y `Correcciones_Durante_Desarrollo.md`)
- ✅ Disparador de Granja por déficit REAL (producción de trigo < consumo de población+tropas), no por reserva estimada — hasta 3 Granjas a la vez en déficit (antes solo 1); "Protección de Riesgos" ampliada a 2 Leñeras + 3 Granjas; capacidad de Leñeras por tamaño de bosque (1-3); fix de un bug real donde el alcance de un bosque solo comprobaba su centro, no su solapamiento con la zona (ver Doc 4.2 y `Correcciones_Durante_Desarrollo.md`)

## Escala social y política
- ✅ Glosario central: Jugador → Facción → Asentamientos; Liga = red de Facciones
- ✅ Progresión: independiente → vasallaje/alianza → Liga → Gran Rey (prestigio)
- ✅ Reglas de vasallaje completas (tributo, defensa mutua, 4 vías de ruptura)
- ✅ Coste de gobernanza absorbido en Mantenimiento; identidad visual (sigilo/estandarte)

## Ciudadanía
- ✅ Ligada a la Facción; obtención por fundación o compra de casa; 5 beneficios confirmados
- 🔶 Residencia en cualquier asentamiento, protección militar, voz en política exterior, nivel intermedio de comisiones

## Cargos
- ✅ Nivel Facción (Rey, Embajador) y nivel asentamiento (Gobernador, Tesorero, General, Maestro de Obras, Sacerdote)
- ✅ Multi-cargo permitido; liberación tras 1 semana de inactividad

## Mantenimiento
- ✅ Sistema unificado: medidor 0-100, coste periódico escalonado por nivel (madera+comida → +piedra → +oro, todos simultáneos en niveles tardíos) y por distancia al centro de poder de la Facción
- ✅ Nivel de asentamiento — rediseño Fase 0: modelo por gates (población + edificios específicos), reemplaza la fórmula de puntos anterior. Tope de Fase 0 = nivel 3. Nivel 2 = 200 pesants + 50 artesanos + Armería/Curtiduría/Fundición construidas; Nivel 3 = 500 pesants + 200 artesanos + Carpintería/Barracón/Galería de tiro construidas (ver Doc 4.5)
- ✅ Rediseño Fase 0: con el tope bajando a 3, los umbrales de piedra/oro (antes nivel 3 y nivel 8, pensados para rango 1-10) se recalibran al rango 1-3 — cifra exacta pendiente de calibración (ver Doc 4.5)
- ✅ Período de gracia al fundar (sin cobro de mantenimiento los primeros ticks) — validado en implementación, resuelve protección temporal de asentamientos nuevos
- ✅ Degradación proporcional del medidor si no se cumple el pago; al llegar a 0 el asentamiento se destruye y cae en ruinas (conecta con abandono total)
- ✅ Fix de mecánica repetida (post rediseño de progreso): el trigo ya NO es un coste fijo dentro de Mantenimiento (duplicaba el consumo real de comida/raciones que ya se descontaba aparte) — el "apartado de trigo" mostrado ahora es la suma real `consumo de comida de población + ración de tropas` (fijo por habitante, ver sección Población), y se descuenta una sola vez donde siempre se descontó. Efecto colateral: un déficit de trigo ya no degrada el medidor directamente, solo frena el crecimiento poblacional — el medidor pasa a depender solo de madera/piedra/oro. Producción base de Granja recalibrada dos veces (5→10→15 trigo/tick), disparador de Granja pasado a déficit real (producción<consumo, hasta 3 a la vez) y nueva política "Edicto de Cosecha" (Gobernador, ×1.5 trigo) como mitigación adicional (ver Doc 4.5 y `Correcciones_Durante_Desarrollo.md`)
- ✅ Cifras recalibradas durante implementación (coste base y velocidad de degradación reducidos) para evitar espirales de déficit en asentamientos bien gestionados — validación corresponde al modelo de nivel anterior, pendiente de repetirse tras el rediseño
- 🔶 Cantidades exactas finales por nivel, en qué nivel exacto (1-3) empiezan a exigirse piedra y oro, velocidad exacta de degradación/regeneración, duración exacta del período de gracia, tope exacto de extractores por tipo (ver Doc 4.2) — siguen siendo placeholder

## Entrada tardía y mundo lleno
- ✅ Mapa difícil de saturar; nuevos servidores; deterioro libera zonas

## Guerra, diplomacia y mundo
- ✅ Guerra: combate héroe+tropa, formaciones/cohesión, 4 modalidades, permadeath+squad, doble carril, attack timer (pospuesto), exilio. Adaptación temática completa (cobre/estaño)
- ✅ Rediseño Fase 0 IMPLEMENTADO: roster reemplaza los 4 tiers genéricos — reclutamiento de tropas por edificio (Barracón/Galería de tiro) según su nivel interno (1-3), con costo en equipo fabricado en Armería; Nobleza sin cambios (vía Gran Fundición). Veteranía sigue dando bonus de poder pero ya no asciende de tier automáticamente a las tropas de equipo — "mejorar" es reclutar una tropa mejor cuando el edificio suba de nivel. Ver Doc 5.7/5.8, `constants.ts` `TROPAS_RECLUTABLES`, `engine/tropas.ts` `reclutarTropa`.
- 🔶 Guerra (detalle): declaración formal, conquista exacta tras asedio, unidades navales
- 🔶 Establos/carros de guerra sin edificio de reclutamiento definido tras el rediseño; poderBase de las 10 tropas nuevas es placeholder pendiente de calibración (ver Doc 5.8)
- ✅ Diplomacia: score de confiabilidad de Facción (-100 a +100) completo; exilio resuelto; rumores/espionaje descartado
- ✅ Generación del mundo (Fase 0) resuelta
- 🔶 Generación del mundo (fases avanzadas): mapa fijo vs procedural con relieve, puntos de interés, biomas
- 🔶 Exploración: niebla de guerra, NPCs hostiles (sin abordar)
- ✅ Progresión de imperio: sandbox sin condición de victoria, títulos dinámicos de prestigio
- 🔶 Progresión de imperio (detalle): diferencia imperio vs alianza grande, fragmentación, catálogo de títulos
- ✅ Interfaz de decisiones del jugador: resuelta (ver Políticas)
- 🔶 Identidad visual y de audio: referencias estéticas (sin abordar)

## Políticas
- ✅ Duración, renovación, múltiples activas, no cancelables; conexión con auto-construcción
- ✅ Slots y pools por cargo (Gobernador 2→5, Tesorero 2, resto 1 c/u)
- ✅ "Protección de Riesgos" (Maestro de Obras) ampliada a exigir 2 Leñeras + 3 Granjas (antes solo 3 Leñeras); "Edicto de Cosecha" (Gobernador, nueva) ×1.5 producción de trigo — ver Doc 4.4/4.5
- 🔶 Catálogo concreto de políticas; si son excluyentes dentro de un slot

## Fusión y crecimiento de Facciones
- ✅ Menú con 2 opciones (Anexión / Fusión), reglas de Rey y cargos resultantes
- 🔶 Aceptación mutua obligatoria vs. anexión forzable unilateralmente

## Gremios (edificios especiales, nuevo)
- ✅ 4 gremios (Comerciantes, Artesanos, Constructores, Ladrones): escasos a nivel de servidor, tirada periódica según 4 REQUISITOS simultáneos (score reputación >90, título de servidor específico, nivel de asentamiento en el máximo, Y mantenimiento >90% — confirmado AMBOS a la vez, no alternativos). RESUELTO mecanismo de pérdida: el título solo se evalúa en la tirada inicial + duración mínima garantizada (tras la cual puede transferirse a otro candidato si el dueño ya no tiene el título); los otros 3 requisitos causan pérdida inmediata sin transferencia (el gremio queda sin sede hasta que alguien vuelva a cumplir). Gremio de Ladrones confirmado (info de acuerdos/caravanas/Facciones ajenas, acotado). Resto de beneficios y números exactos pendientes.

## Roadmap / escalado
- ✅ Eje de fidelidad visual (Fase 0 → Fase 1 → Fase final); eje naval (terrestre → con mar)
- 🔶 Otros ejes de escalado (número de jugadores, complejidad de combate, etc.)
