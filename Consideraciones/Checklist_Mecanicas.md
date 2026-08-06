# Checklist de Mecánicas

Lista completa de mecánicas diseñadas, organizadas por sistema. ✅ = cerrado/decidido. 🔶 = en discusión, con hueco pendiente. ❌ = descartado explícitamente.

**ESTADO:** Fase 0 (Sprints 1-6) ya fue **implementada** en TypeScript y validada jugando en el navegador. Ver `Correcciones_Durante_Desarrollo.md` para los ajustes de diseño que surgieron durante la implementación (varios resolvieron preguntas antes pendientes).

Ver `Docs/0_Glosario_de_Entidades_Politicas.md` para las definiciones centrales.

## Concepto y ambientación
- ✅ Ambientación: Edad Oscura de la Edad de Bronce
- ✅ Mapa enorme y desconectado al inicio, crecimiento deliberadamente lento

## Fundación y zonas de influencia
- ✅ Fundación libre, zona de influencia automática, reglas de construcción, fronteras con límite duro
- ✅ Onboarding: spawn aleatorio + fundación grupal (hasta 5 jugadores)
- ✅ Chokepoints estratégicos (fases con relieve)
- ✅ Cap de fundación por Facción (escala con nivel de Facción, 1→3 fácil, hasta 7 tardío)
- ✅ Protección temporal: período de gracia sin cobro de Mantenimiento al fundar (RESUELTO durante implementación Fase 0)
- 🔶 Qué hace subir el "nivel de Facción" exactamente; curva exacta entre cap 3 y cap 7

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
- ✅ Algoritmo por reglas; crecimiento por necesidad, con reevaluación CONTINUA (no solo "construir una vez"): granjas adicionales si la reserva proyectada cae bajo el umbral; Maestro de Obras; layout dinámico por política
- ✅ Reemplazo automático de extractores (cantera/mina de oro/mina de cobre/lenera) cuando su fuente se agota, hasta un máximo según nivel del asentamiento

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
- ✅ Sistema unificado: medidor 0-100, coste periódico escalonado por nivel (madera+comida → +piedra → +oro desde nivel 8, todos simultáneos en niveles tardíos) y por distancia al centro de poder de la Facción
- ✅ Período de gracia al fundar (sin cobro de mantenimiento los primeros ticks) — validado en implementación, resuelve protección temporal de asentamientos nuevos
- ✅ Degradación proporcional del medidor si no se cumple el pago; al llegar a 0 el asentamiento se destruye y cae en ruinas (conecta con abandono total)
- ✅ Cifras recalibradas durante implementación (coste base y velocidad de degradación reducidos, umbral de oro movido de nivel 6 a 8) para evitar espirales de déficit en asentamientos bien gestionados
- 🔶 Cantidades exactas finales por nivel, velocidad exacta de degradación/regeneración, duración exacta del período de gracia — siguen siendo placeholder

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

## Gremios (edificios especiales, nuevo)
- 🔶 4 gremios (Comerciantes, Artesanos, Constructores, Ladrones): escasos a nivel de servidor, tirada periódica según 3 requisitos (score >90, título de servidor, nivel/mantenimiento >90%), se pierden si se incumple algún requisito. Gremio de Ladrones confirmado (info de acuerdos/caravanas/Facciones ajenas, acotado). Resto de beneficios y números exactos pendientes.

## Roadmap / escalado
- ✅ Eje de fidelidad visual (Fase 0 → Fase 1 → Fase final); eje naval (terrestre → con mar)
- 🔶 Otros ejes de escalado (número de jugadores, complejidad de combate, etc.)
