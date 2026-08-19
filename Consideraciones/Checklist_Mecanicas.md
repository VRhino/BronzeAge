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
- ✅ Hambruna (a petición del usuario): no sostener el consumo de trigo ya tiene efecto negativo real, no solo frena el crecimiento. Medidor `nutricionPoblacion` (0-100, espejo de la moral de tropas por ración) sube/baja según la fracción de consumo cubierta cada tick; por encima de 0 escala linealmente el factor de crecimiento (0.2-1, reemplaza el viejo booleano trigo>0?1:0.2); en 0 sostenido cuesta 5%/tick de pesants+artesanos (nobleza protegida). Ver Doc 4.1.
- 🔶 Fórmula exacta de crecimiento de Artesanos; criterio exacto de cola de prioridad de reclutamiento; calibración de las cifras de Hambruna (`POBLACION.hambre`) por simulación

## Construcción automática
- ✅ Jugador no elige ubicación/tipo (excepto fundación y edificios estratégicos)
- ✅ Algoritmo por reglas; crecimiento por necesidad, con reevaluación CONTINUA (no solo "construir una vez"): granjas adicionales si hay déficit; Maestro de Obras; layout dinámico por política
- ✅ Reemplazo automático de extractores (cantera/mina de oro/mina de cobre/mina de estaño/lenera/Corral) cuando su fuente se agota, hasta un máximo por tipo — rediseño Fase 0: desacoplado del nivel del asentamiento (antes escalaba 1:1 con él), ahora número fijo pendiente de calibración (ver Doc 4.2)
- ✅ Rediseño Fase 0: catálogo ampliado con 8 edificios nuevos (Corral, Armería, Curtiduría, Carpintería, Palacio, Barracón, Galería de tiro; Murallas sigue fuera de alcance) con recetas multi-nivel — ver Doc 4.2.1. Curtiduría/Armería/Fundición/Carpintería son auto-construcción; Barracón/Galería de tiro/Palacio/Mercado van vía adición MANUAL de Gobernador/Maestro de Obras (ya NO política dedicada — retirada, ver control manual de cola arriba y Doc 4.4)
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

## Nuevas mecánicas (post-análisis comparativo con Travian, a petición del usuario)
- ✅ Control manual de cola de auto-construcción (Gobernador/Maestro de Obras) — IMPLEMENTADO EN MOTOR Y EN INTERFAZ:
  - Motor (`engine/construction.ts`): `anadirEdificioManualmente` (cualquier edificio del catálogo salvo Centro Urbano, respeta gates de nivel y reserva de Mantenimiento), `quitarDeCola` (solo `en_cola`, devuelve costo completo), `moverEnCola` (intercambia prioridad con el vecino). Ubicación SIEMPRE automática, nunca elegible. Las 4 políticas "Construir Barracón/Galería de tiro/Palacio/Mercado" y su cluster de cola aparte se retiraron por completo.
  - Interfaz (`app/gameStore.ts`, `main.ts`): panel "Cola de construcción — control manual" en la pestaña Asentamientos — tabla de la cola con botones ▲/▼/Quitar por fila, selector de cargo (Gobernador/Maestro de Obras), selector de tipo + botón "Añadir a la cola", y segmento "Info:" (`#cola-tipo-info`) que muestra costo de construcción, tiempo y requisitos de nivel del tipo seleccionado antes de confirmar (mismo patrón que el segmento "Info:" de reclutamiento de tropas).
  - Verificado en el navegador sin errores de consola: rechazo correcto por fondos insuficientes y por falta de sitio, Barracón/Palacio disponibles sin política, segmento Info actualizándose en vivo (Vivienda, Palacio).
  - Ver Doc 2.2/4.2/4.4, `Preguntas_Abiertas.md` #14b.
- ✅ Filtro de Facción por combobox en las pestañas Asentamientos/Jugadores (mejora de usabilidad de la interfaz de depuración, NO es mecánica de juego) — reemplaza el listado simultáneo de todos los grupos de Facción, que dejaba de caber en pantalla con muchas Facciones creadas. Sigue por defecto a la Facción dueña del elemento ya seleccionado; acoplamiento 0 preservado (estado de vista local a `main.ts`, sin tocar `gameStore`/`engine`). Verificado en el navegador con 2 Facciones.
- ✅ Selector de escuadrones propios en Combate por chips (mejora de usabilidad de la interfaz de depuración, NO es mecánica de juego, a petición del usuario) — reemplaza el input de texto libre "ids separados por coma" por una cascada Facción → Asentamiento (filtrado por esa Facción) → chips de escuadrones (clic para seleccionar/deseleccionar, borde blanco sin seleccionar / verde seleccionado). Alimenta las 4 acciones de combate (asedio, campo abierto, interceptar caravana, atacar campamento), que ahora usan el asentamiento elegido en esta cascada como atacante en vez del selector "Asentamiento" de Reclutamiento (que sigue existiendo, solo para reclutar/Gran Fundición). La selección se limpia sola si cambia Facción/Asentamiento o si un escuadrón deja de existir. Verificado en el navegador: cascada correcta, chip cambia de borde blanco a verde al seleccionarlo, ataque a campamento exitoso usando el chip seleccionado, selección y cantidad actualizada tras el combate, sin errores de consola (`main.ts`, `ui/style.css`).
- ✅ Campamentos de bandidos en bosques no reclamados — IMPLEMENTADO EN MOTOR Y EN INTERFAZ: spawn/respawn y ataque a caravanas automáticos cada tick (`engine/bandidos.ts`); ataque manual del jugador con recompensa (`atacarCampamentoBandidos`, `engine/combate.ts`); marcador en el mapa (diamante rojo) y selector dedicado en la pestaña Guerra. Tope UNO por asentamiento (rediseño a petición del usuario), en su bosque no reclamado más cercano. Cifras placeholder: poder 30, radio de ataque 40, radio de cobertura 600, recompensa 40 madera+20 piedra+15 oro, respawn 60 ticks. Verificado en el navegador de punta a punta con 2 asentamientos lejanos (cada uno con su propio campamento cercano, tope respetado), incluida la reaparición exacta en el tick esperado (ver Doc 1.9/3.10, `Preguntas_Abiertas.md` #14c)
- ✅🔶 Maravilla: el EDIFICIO está IMPLEMENTADO (a petición del usuario, "solo la parte básica"); el CICLO de servidor sigue sin implementar:
  - ✅ Edificio (`EDIFICIO_CATALOGO.maravilla`, constants.ts): único, nivel de asentamiento 3 requerido, disponible vía control manual de cola (Doc 4.2, Gobernador/Maestro de Obras) — mismo patrón que Gran Fundición/Palacio. Coste placeholder con recursos ya existentes (5000 madera + 5000 piedra + 500 oro + 300 cobre + 200 estaño + 200 livestock, 200 ticks) — los materiales EXÓTICOS del diseño original no están implementados (ese tipo de recurso no existe todavía en el juego). Verificado en el navegador: Info correcta en el selector, rechazo correcto por nivel de asentamiento insuficiente, sin errores de consola.
  - 🔶 Ciclo de 12 meses + cierre al completarla + Facción ganadora persiste como legado NPC — DISEÑO CERRADO, SIN IMPLEMENTAR: requiere infraestructura de servidor/multi-instancia que Fase 0 no tiene (ver `Roadmap_Escalado.md` Eje 4, Doc 2.9, `Preguntas_Abiertas.md` #14d)
- 🔶 Curva de progresión inicial/onboarding más gradual — SIN DECIDIR qué sistemas se desbloquean y cuándo; pospuesto hasta que exista una interfaz de jugador individual (la actual es de GM, ver `Preguntas_Abiertas.md` #14e)
- ✅ Fix de tropas — IMPLEMENTADO EN DISEÑO Y EN CÓDIGO: tropas de equipo NUNCA cambian de `tropaId` al ganar veteranía, solo suben de poder dentro del mismo tipo (ver Doc 5.8). Código muerto retirado por completo: `TROPA_CATALOGO`, `ASCENSO_TROPA`, `ascenderTierSiCorresponde` y el campo `Escuadron.tier` ya no existen; `Escuadron.tropaId` pasó a obligatorio. UI corregida: la tabla de escuadrones (Asentamientos) y el panel militar (Guerra) mostraban "Tier 1" siempre (bug, nunca cambiaba) — ahora muestran el `nivelRequerido` real de la tropa. Verificado en el navegador reclutando una tropa, sin errores de consola (`domain/types.ts`, `engine/tropas.ts`, `engine/combate.ts`, `constants.ts`, `app/balanceConfig.ts`, `main.ts`).

## Roadmap / escalado
- ✅ Eje de fidelidad visual (Fase 0 → Fase 1 → Fase final); eje naval (terrestre → con mar)
- 🔶 Eje 4 (nuevo): ciclo de servidor de 12 meses + Maravilla + legado NPC de la Facción ganadora (ver arriba)
- 🔶 Otros ejes de escalado (número de jugadores, complejidad de combate, etc.)
