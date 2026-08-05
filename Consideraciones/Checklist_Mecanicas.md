# Checklist de Mecánicas

Lista completa de mecánicas diseñadas, organizadas por sistema. ✅ = cerrado/decidido. 🔶 = en discusión, con hueco pendiente. ❌ = descartado explícitamente.

Ver `Docs/0_Glosario_de_Entidades_Politicas.md` para las definiciones centrales.

## Concepto y ambientación
- ✅ Ambientación: Edad Oscura de la Edad de Bronce
- ✅ Mapa enorme y desconectado al inicio, crecimiento deliberadamente lento

## Fundación y zonas de influencia
- ✅ Fundación libre, zona de influencia automática, reglas de construcción, fronteras con límite duro
- ✅ Onboarding: spawn aleatorio + fundación grupal (hasta 5 jugadores)
- ✅ Chokepoints estratégicos (fases con relieve)
- ✅ Cap de fundación por Facción (escala con nivel de Facción, 1→3 fácil, hasta 7 tardío)
- 🔶 Qué hace subir el "nivel de Facción" exactamente; curva exacta entre cap 3 y cap 7
- 🔶 Protección temporal para asentamientos recién fundados

## Tecnología
- ✅ Tres vías de acceso (comercio, desarrollo propio, Aedas); sistema de Aedas completo; tecnología se compra con oro
- 🔶 Árbol tecnológico del desarrollo propio: estructurado o libre/emergente

## Comercio y economía
- ✅ Comercio siempre manual; caravanas terrestres; comercio marítimo (fuera de Fase 0)
- ✅ Economía dual (oro + trueque); oro = metal precioso, origen en minas
- ✅ Acuerdos de trueque; órdenes de mercado; precios dinámicos; comisiones más bajas intra-Facción
- ✅ Categorías de caravana; transporte individual; bonificación por distancia; dependencia logística
- ✅ Caminos comerciales automáticos
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
- 🔶 Fórmula exacta de crecimiento de Artesanos; criterio exacto de cola de prioridad de reclutamiento

## Construcción automática
- ✅ Jugador no elige ubicación/tipo (excepto fundación y edificios estratégicos)
- ✅ Algoritmo por reglas; crecimiento por necesidad; Maestro de Obras; layout dinámico por política

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
- ✅ Sistema unificado: medidor 0-100, coste escalonado por nivel y distancia, degradación proporcional, destrucción a 0
- 🔶 Cantidades exactas por nivel, velocidad de degradación, posibilidad de recuperación antes de 0

## Entrada tardía y mundo lleno
- ✅ Mapa difícil de saturar; nuevos servidores; deterioro libera zonas

## Guerra, diplomacia y mundo
- ✅ Guerra: combate héroe+tropa, formaciones/cohesión, 4 modalidades, permadeath+squad, doble carril, attack timer (pospuesto), exilio. Adaptación temática completa (cobre/estaño) y roster de 4 tiers
- 🔶 Guerra (detalle): declaración formal, conquista exacta tras asedio, unidades navales
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
- 🔶 Catálogo concreto de políticas; si son excluyentes dentro de un slot

## Fusión y crecimiento de Facciones
- ✅ Menú con 2 opciones (Anexión / Fusión), reglas de Rey y cargos resultantes
- 🔶 Aceptación mutua obligatoria vs. anexión forzable unilateralmente

## Roadmap / escalado
- ✅ Eje de fidelidad visual (Fase 0 → Fase 1 → Fase final); eje naval (terrestre → con mar)
- 🔶 Otros ejes de escalado (número de jugadores, complejidad de combate, etc.)
