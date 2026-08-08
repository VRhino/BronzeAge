# 4. Sistema de Población, Construcción Automática y Mantenimiento

## 4.1 Población NPC: 3 clases
Los JUGADORES son una categoría SEPARADA de estas 3 clases.

| Clase | Rol | Aparición | Crecimiento |
|---|---|---|---|
| Pesants | Trabajan recursos genéricos (granjas, canteras, bosques); tropas básicas | Desde la fundación, sin condición | Rápido, fórmula propia |
| Artesanos | Operan edificios de producción especializada (Fundición, Curtiduría, Armería, Carpintería — ver catálogo en 4.2); reclutables junto con Pesants vía Barracón/Galería de tiro (Tier 2, Doc 5) — ambos edificios reclutan de los dos pools, no son exclusivos de Artesanos | Automática: dispara el PRIMERO que se construya entre Curtiduría/Armería/Fundición/Carpintería | Fórmula propia independiente (pendiente detalle numérico); tope de población limitado (ver nota abajo) |
| Nobleza | Tropas de élite (vía equipo, sin grindeo) | Cantidad MÍNIMA de ciudadanos (jugadores) en el asentamiento (único requisito) | Muy lento; acelerado por el Sacerdote (no requisito) |

Cada clase tiene FÓRMULA DE CRECIMIENTO INDEPENDIENTE (no comparten los mismos pesos). Fórmula base de referencia (Pesants): comida disponible + vivienda disponible + estabilidad + felicidad.

**Disparador y crecimiento de las 3 clases (rediseño, detectado en batch de 100 simulaciones + traza de 8000 ticks, ver `Correcciones_Durante_Desarrollo.md`)**: las 3 clases usan la MISMA fórmula proporcional — `comida disponible × cupo libre de la clase × tasa propia` — difiriendo solo en tasa: Pesants 0.12, Artesanos 0.05, Nobleza 0.01. Se retira la función `capacidadArtesanos` (tope ligado a `trabajadoresRequeridos` de edificios de transformación, versión anterior de este párrafo) — Artesanos ahora solo necesita el PRIMER edificio de transformación activo (Curtiduría/Armería/Fundición/Carpintería) para que aparezca el primero, y desde ahí crece a su propia tasa, limitado únicamente por su cupo de Vivienda (ver 4.2.1). **Problema detectado y resuelto**: con cupo de Vivienda antes COMPARTIDO entre Pesants y Artesanos, y Pesants creciendo ~2.4× más rápido, Pesants saturaba el cupo total y Artesanos quedaba varado en 1 unidad indefinidamente (verificado 7600 ticks sin moverse). No era un bug, era consecuencia fiel de las reglas dadas. **Solución aplicada**: Vivienda pasa a dar cupos SEPARADOS por clase (15 Pesants + 5 Artesanos por unidad, ver 4.2.1) en vez de un pool único — Artesanos ya no compite directamente con Pesants por el mismo espacio.

**Consumo de comida (trigo)**: tasa FIJA por habitante, sumada por el total de habitantes (pesants+artesanos+nobleza) — no depende de la clase. Reducible por la política Racionamiento (Sacerdote, ×0.8). Este consumo se suma al de las tropas (raciones de escuadrones, Doc 5.4) para formar el "apartado de trigo" que se muestra en el panel de Mantenimiento (ver 4.5) — corrección de una mecánica repetida donde Mantenimiento cobraba ADEMÁS un valor fijo de trigo desconectado del consumo real (ver `Correcciones_Durante_Desarrollo.md`).

**Reclutamiento militar (doble carril):**
- Carril COMBATE REAL (Pesants + Artesanos): deben salir a combatir (PvP o PvE) para "veteranizar" y subir de tier.
- Carril PROGRESIÓN PLANA (Nobleza): conversión INSTANTÁNEA a unidades de élite si hay equipo disponible.
- Cola de PRIORIDAD cuando la demanda excede el pool disponible (criterio exacto pendiente).

## 4.2 Auto-construcción por necesidad
- El jugador NO elige ubicación ni tipo de edificio, EXCEPTO: edificio de fundación, y edificios estratégicos (murallas, torres, puerto) que sí se colocan manualmente.
- Algoritmo de colocación: reglas por tipo de edificio (ej. granja cerca de tierra fértil, herrería cerca de mina+camino), elige la mejor casilla disponible dentro de la zona de influencia.
- Crecimiento disparado por NECESIDAD REAL: más población → necesidad de comida → granja automática; excedente de recurso → mercado/almacén.
- **Escalado por demanda continua** (confirmado durante implementación de Fase 0): la construcción automática no se limita a "construir una vez si no existe ninguna" — vuelve a evaluarse continuamente. Ej. Granja: se encola una adicional en cuanto haga falta, incluso si ya existe al menos una (bug detectado en Sprint 2: sin esto, la población entraba en hambruna silenciosa al crecer más allá de lo que una sola Granja podía sostener).
- **Disparador de Granja: déficit real, no reserva estimada** (rediseño a petición del usuario, ver `Correcciones_Durante_Desarrollo.md`): la versión anterior disparaba una Granja nueva cuando "trigo disponible ÷ consumo actual" caía por debajo de 5 ticks — una estimación optimista, porque el consumo sigue creciendo con la población mientras la Granja se construye (6 ticks), y no contaba las raciones de tropas. Ahora se compara directamente **producción actual de trigo (todas las Granjas activas, con fertilidad/mano de obra/Edicto de Cosecha) contra consumo actual (población + tropas, Doc 4.1/5.4)** — si producción < consumo, hay déficit. Además, en déficit se permite tener **hasta 3 Granjas `en_cola`/`en_construccion` a la vez** (antes solo 1, obligando a corregir un déficit severo en serie, una Granja cada 6 ticks); fuera de déficit sigue siendo 1 a la vez, como el resto de edificios de supervivencia.
- **Reemplazo de fuentes agotadas** (nueva mecánica, confirmada durante implementación): los extractores de recursos finitos (cantera, mina de oro, mina de cobre, **mina de estaño**, **Corral**) pueden agotar su nodo fuente. La lógica de disparo cuenta cuántos extractores tienen FUENTE VIVA (no solo cuántos existen en total) — si un yacimiento se agota, se encola automáticamente un extractor de reemplazo (buscando un nuevo nodo del mismo recurso), hasta un MÁXIMO por tipo de extractor. Mismo criterio aplicado a cantera, mina de oro, mina de cobre, mina de estaño, lenera y Corral (bosques también se agotan). Sin esto, agotar el único yacimiento condenaba al asentamiento a un déficit permanente e irreversible. **Rediseño Fase 0**: este máximo se DESACOPLA del nivel del asentamiento (antes escalaba 1:1 con él) — con el tope de nivel bajando a 3 (ver 4.5), un máximo ligado al nivel se quedaría corto; pasa a ser un número fijo por tipo de extractor, cifra exacta PENDIENTE de calibración por simulación.
- **Corral** (nuevo edificio, rediseño Fase 0): extractor de livestock, mismo patrón que cantera/minas — liga a un nodo finito de livestock (Doc 1.4, ya generado por el mundo), con reemplazo automático al agotarse. Ver costo/producción en el catálogo de 4.2.1.
- **Curtiduría / Armería / Fundición / Carpintería** (rediseño Fase 0): van por AUTO-CONSTRUCCIÓN, igual que Granja/Cantera — se disparan en cuanto se cumple su requisito de nivel de asentamiento/edificio previo (ver catálogo en 4.2.1), sin exigir Planos/Aedas en Fase 0 (Doc 6 pospone ese sistema por completo). No van vía política. La Fundición aquí reemplaza y amplía la Fundición manual ya existente (antes sin producción propia, solo gate de ascenso de tropa — Doc 5.8): mismo edificio, ahora con recetas reales y disparo automático en vez de manual.
- **Barracón / Galería de tiro / Palacio** (rediseño Fase 0): a diferencia del resto, van vía POLÍTICA dedicada (ver 4.4) — mientras la política correspondiente esté activa, la construcción de ese edificio salta la cola normal de 3 slots y usa un cluster de cola aparte.
- **Cantera y Gran Fundición**: sin cambios respecto a la versión ya implementada — Gran Fundición queda para iteraciones posteriores (integración con la nueva Fundición fuera de alcance de este rediseño).
- **Mina de estaño** (rebalance posterior a Sprint 6): el estaño se genera en el mundo con el mismo criterio de rareza que cobre/oro (Doc 1.1) desde el Sprint 1, pero no tuvo edificio de extracción propio hasta esta corrección — hasta entonces solo era obtenible por trueque. Mismo patrón que cantera/mina/mina de cobre.
- Excepción permanente EN EL DISEÑO OBJETIVO (con simplificación en Fase 0, ver Doc 3.2/3.3): el COMERCIO/CARAVANAS no debería crearse automáticamente, debería ser siempre acción manual del jugador. EN LA PRÁCTICA DE FASE 0, el trueque y las órdenes de mercado SÍ se despachan/liquidan automáticamente por simplificación intencional — el plan es pasar a movimiento manual en Fase 1+ (ver Doc 3 para el detalle completo y estado real).
- Cadenas de producción (materia prima → producto) son lógica INTERNA invisible — el jugador solo ve materiales almacenados y necesidades activas (déficits).
- Cargo MAESTRO DE OBRAS gestiona las prioridades de auto-construcción, da bonus a tiempos de construcción.
- Patrón de crecimiento: las ciudades agregan edificios desde el CENTRO hacia afuera (concéntrico).
- LAYOUT DINÁMICO según política activa: ej. política DEFENSIVA prioriza edificios defensivos y layout fácil de defender (con coste real: pierde eficiencia económica, genera "pasillos" hacia puntos de captura). Elimina la repetición visual entre asentamientos.
- **Tope de cola** (rebalance posterior a Sprint 6): máximo 3 edificios `en_cola` simultáneos por asentamiento, compartido entre auto-construcción y construcción manual (Gran Fundición — la única que queda de colocación manual clásica). Barracón/Galería de tiro/Palacio (rediseño Fase 0, ver arriba) NO comparten este cupo: usan su propio cluster de cola aparte, condicionado a la política correspondiente (ver 4.4).
- **Orden de prioridad e interbloqueo de recursos de supervivencia** (bug real detectado jugando, ver `Correcciones_Durante_Desarrollo.md`): con el tope de cola, un proyecto que no puede pagarse se queda parado en cola indefinidamente (nada lo saca si nunca junta recursos). Si Vivienda/Granja/Cantera —que se re-disparan casi todos los ticks— se evaluaban antes que la Leñera, podían copar los 3 slots con proyectos atascados y dejar a la Leñera (única fuente de madera) sin hueco para encolarse NUNCA, condenando al asentamiento a un déficit de madera permanente que Mantenimiento acababa castigando hasta la destrucción. Corrección: (a) Granja/Leñera (recursos de supervivencia) se evalúan primero, extractores secundarios después, y Vivienda/Almacén/Taller (crecimiento) al final; (b) de los 3 slots de cola, 1 queda reservado exclusivamente para Granja/Leñera, así el resto nunca puede dejarlas sin hueco.
- **Orden de GASTO de recursos al arrancar construcciones (bug real detectado jugando, post rediseño de progreso, ver `Correcciones_Durante_Desarrollo.md`)**: la corrección anterior prioriza qué se ENCOLA, pero no quién GASTA primero el almacén compartido cuando varios edificios ya están `en_cola` a la vez — eso lo decidía el orden de INSERCIÓN en el array, no la categoría. Curtiduría/Armería/Fundición (sin gate de nivel, se encolan casi desde el tick 1, cuestan 80 madera cada una frente a 30/10 de Granja/Leñera) podían quedar antes en el array que una Granja/Leñera nueva encolada más tarde por crecimiento de población, y llevarse la madera disponible primero — dejando al asentamiento sin margen para sostenerse a sí mismo pese a tener madera de sobra para ambas. Corrección: el arranque de construcciones en cola ahora se resuelve en orden de PRIORIDAD por categoría (supervivencia > extractores > general), no de inserción — Granja/Leñera siempre ganan la carrera por el almacén frente a Curtiduría/Armería/Fundición/Vivienda/Almacén cuando hay que elegir.
- **Reserva mínima de construcción** (nueva mecánica, corregida): un edificio en cola solo empieza a construirse si no deja ningún recurso de Mantenimiento por debajo de una reserva mínima, según el nivel actual del asentamiento — **madera desde nivel 1, +piedra desde nivel 2, +oro desde nivel 3** (placeholder: madera 30, piedra 20, oro 10; corrige una referencia desactualizada a "piedra desde nivel 3/oro desde nivel 8" que ya no aplicaba desde el rediseño de progreso). El TRIGO ya NO forma parte de esta lista como recurso de Mantenimiento (ver fix en 4.5: el trigo se movió al consumo real de población+tropas, no es coste periódico de Mantenimiento). Sigue existiendo la constante `RESERVA_CONSTRUCCION.trigo` (placeholder: 20) pensada como reserva mínima de comida aparte — pero en el código actual ningún edificio tiene trigo en su costo, así que ese chequeo nunca se llega a ejecutar en la práctica; queda como limpieza pendiente (constante sin efecto real). Excepción: Granja no respeta la reserva de trigo, ni Leñera la de madera, porque son la única vía real de recuperar esos recursos.
- **Política "Protección de Riesgos"** (Maestro de Obras, catálogo de 4.4, ampliada a petición del usuario): mientras esté activa, bloquea toda auto-construcción que no sea Leñera/Granja hasta llegar a **2 Leñeras y 3 Granjas** (activas o en curso/cola, ambos objetivos independientes) — ambas se evalúan en la misma pasada, cada una progresa mientras la otra también lo hace. Excepción: si NINGUNA de las dos puede avanzar ese tick (sin sitio disponible para la que falte, y ninguna ya en camino), deja pasar la evaluación normal para no bloquearse sin salida — verificado que esto puede dejar el objetivo de Leñeras permanentemente incompleto si el asentamiento no tiene un segundo bosque alcanzable nunca (comportamiento aceptado, ver `Correcciones_Durante_Desarrollo.md`).
- **Capacidad de Leñeras por bosque** (nueva mecánica, a petición del usuario): un bosque grande admite más de una Leñera trabajándolo a la vez, no solo una — la capacidad depende de su radio: 1 (radio < 45), 2 (45-64) o 3 (≥65). Las Leñeras que comparten un bosque se colocan en puntos distintos dentro de su radio (solo visual — la producción sigue usando `bosque.densidad`, no la posición exacta).
- **Bug real: alcance de un bosque mal calculado** (detectado jugando, reportado por el usuario, ver `Correcciones_Durante_Desarrollo.md`): la búsqueda de sitio para Leñera solo comprobaba si el CENTRO exacto del bosque caía dentro de la zona de influencia — pero los bosques tienen radio (30-80) y el radio de zona tiene un TOPE por nivel (60/90/120, ver Doc 1.2). Un bosque grande cuyo borde ya estaba bien dentro de la zona, pero cuyo centro exacto quedaba un poco más allá del tope, era invisible para siempre — el asentamiento nunca conseguía Leñera pese a que la zona ya "tocaba" el bosque físicamente, y acababa cayendo en ruinas por falta de madera. Corregido: ahora se acepta cualquier punto del bosque que caiga dentro de la zona, no solo su centro.

## 4.2.1 Catálogo completo de edificios (rediseño Fase 0)

Fase 0 descarta por completo los requisitos de "Planos de X" (vía Aedas, Doc 6) en cualquier mejora de nivel de edificio — el sistema de Aedas no tiene implementación real todavía (ver Doc 6). Los requisitos de NIVEL DE ASENTAMIENTO sí aplican (modelo de gates, ver 4.5); tope de Fase 0 = nivel 3.

### Alojamiento

**Palacio** — desbloquea la aparición de Nobleza. Construcción vía política del Gobernador (ver 4.4), no auto-construcción.
- Requisito: Asentamiento nivel 3.
- Capacidad: 200 nobles. Costo: 1500 madera + 1000 piedra. Tiempo: 20 ticks.

**Vivienda** — auto-construcción. CUPO DIVIDIDO POR CLASE (rediseño tras detección en simulación, ver 4.1 y `Correcciones_Durante_Desarrollo.md`): cada Vivienda aporta cupos SEPARADOS, no un pool compartido — 15 espacios para Pesants + 5 espacios para Artesanos, por unidad de Vivienda (escala linealmente: 2 Viviendas = 30+10, etc.). Nobleza no usa Vivienda (cupo propio en Palacio).
- Costo: 10 madera. Tiempo: 4 ticks.

### Extracción de recursos

| Edificio | Recurso | Costo | Trabajadores | Tiempo | Producción base |
|---|---|---|---|---|---|
| Corral (nuevo) | Livestock | 30 madera | 4 | 6 ticks | 3 livestock |
| Granja | Trigo | 30 madera | 4 | 6 ticks | 15 trigo |
| Leñera | Madera | 10 madera | 4 | 3 ticks | 5 madera |
| Mina de oro | Oro | 40 madera + 10 piedra | 6 | 6 ticks | 2 oro |
| Mina de cobre | Cobre | 30 madera + 5 piedra | 8 | 4 ticks | 5 cobre |
| Mina de estaño | Estaño | 60 madera + 20 piedra | 8 | 7 ticks | 1.5 estaño |

Corral sigue el mismo patrón que Cantera/minas (ver 4.2): liga a un nodo finito de livestock (Doc 1.4), con reemplazo automático al agotarse. Cantera (piedra) no cambia respecto a la versión ya implementada — no está en esta tabla porque sus cifras siguen igual.

**Producción de Granja recalibrada** (post rediseño de progreso, ver `Correcciones_Durante_Desarrollo.md` y 4.5): con una sola Granja por asentamiento y población creciendo un 12%/tick compuesto (sin techo — más población no aumenta la producción una vez cubiertos los `trabajadoresRequeridos`, ver `ratioManoObra`), la producción fija original de 5 trigo/tick no alcanzaba a sostener el consumo — el trigo llegaba a 0 sistemáticamente entre los ticks 45 y 90. Subida primero a 10 y recalibrada de nuevo a **15 trigo/tick**: mejora real, combinada con el disparador de Granja rediseñado (ver 4.2) para reaccionar antes de que el trigo se agote.

### Especiales

**Centro urbano** — edificio inicial, se construye automáticamente al fundar (Doc 1.3), no por ninguna otra vía.

**Almacén** — auto-construcción, sin cambios de diseño. Costo: 50 madera + 30 piedra. Tiempo: 6 ticks.

**Murallas** — edificación defensiva, fuera de alcance de Fase 0 (sin cambios).

### Transformación de recursos

Van por AUTO-CONSTRUCCIÓN (igual que Granja/Cantera), disparadas en cuanto se cumple el requisito de nivel de asentamiento/edificio previo — no van vía política ni requieren Planos/Aedas en Fase 0. Las cadenas de producción siguen siendo lógica interna invisible de cara al jugador (solo ve inputs/outputs netos, ver 4.2). El primer edificio de este grupo que se construye dispara la aparición de Artesanos (ver 4.1).

**Fundición** — fabricación de lingotes de metal. Reemplaza y amplía la Fundición manual ya existente (antes sin producción propia, solo gate de ascenso de tropa Tier 2→3, Doc 5.8) — mismo edificio, ahora con recetas reales y disparo automático.
- Requisito nivel 2: Asentamiento nivel 2.
- Recetas nivel 1: 2 cobre → 1 Lingote de Cobre.
- Recetas nivel 2 (añade): 5 estaño → 1 Lingote de Estaño; 8 Lingote de Cobre + 2 Lingote de Estaño → 5 Lingote de Bronce.
- Costo: construcción 80 madera + 40 piedra; mejora a nivel 2: 150 madera + 100 piedra. Tiempo de construcción: 6 ticks.
- Trabajadores: nivel 1 → 4 artesanos; nivel 2 → 8 artesanos.
- Producción base: nivel 1 → 5 Lingote de Cobre; nivel 2 → 5 Lingote de Cobre + 3 Lingote de Estaño + 1 Lingote de Bronce.

**Curtiduría** — tratamiento de cuero a partir de livestock (vacas).
- Requisito nivel 2: Asentamiento nivel 2. Requisito nivel 3: Asentamiento nivel 3.
- Recetas nivel 1: 1 livestock → 2 Cuero.
- Recetas nivel 2 (añade): 1 livestock → 3 Cuero; 3 Cuero → 1 Cuero Curtido.
- Recetas nivel 3 (añade): 1 livestock → 4 Cuero; 3 Cuero → 1 Cuero Curtido; 2 Cuero Curtido + 1 Cuero → 1 Cuero de Calidad.
- Costo: construcción 80 madera + 30 piedra; mejora 1: 150 madera + 100 piedra; mejora 2: 450 madera + 200 piedra. Tiempo de construcción: 8 ticks.
- Trabajadores: nivel 1 → 4; nivel 2 → 6; nivel 3 → 8 artesanos.
- Producción base: nivel 1 → 4 Cuero; nivel 2 → 4 Cuero + 2 Cuero Curtido; nivel 3 → 6 Cuero + 3 Cuero Curtido + 1 Cuero de Calidad.

**Armería** — fabricación de armas y armaduras.
- Requisito nivel 2: Asentamiento nivel 2 + poseer Carpintería. Requisito nivel 3: Asentamiento nivel 3 + poseer Palacio.
- Recetas nivel 1: 1 Lingote de Cobre + 1 madera → 1 Arma de Cobre; 5 Cuero → 1 Armadura Básica.
- Recetas nivel 2 (añade): 1 Lingote de Bronce + 2 madera → 1 Arma de Bronce; 1 Lingote de Cobre + 5 Cuero Curtido → 1 Armadura Intermedia.
- Recetas nivel 3 (añade): 5 Lingote de Bronce + 5 madera → 1 Arma de Bronce de Calidad; 1 Lingote de Bronce + 5 Cuero de Calidad → 1 Armadura de Bronce.
- Costo: construcción 80 madera + 30 piedra; mejora 1: 150 madera + 100 piedra; mejora 2: 450 madera + 200 piedra. Tiempo de construcción: 6 ticks.
- Trabajadores: nivel 1 → 4; nivel 2 → 8; nivel 3 → 20 artesanos.
- Producción base: nivel 1 → 3 Lingote de Cobre + 3 Armadura Básica; nivel 2 → + 2 Arma de Bronce + 2 Armadura Intermedia; nivel 3 → + 1 Arma de Bronce de Calidad + 1 Armadura de Bronce (acumulativo sobre el nivel anterior).

### Militares

**Carpintería** — recluta armas de asedio (ariete, torre de asedio); su existencia habilita construir/mejorar otros edificios (Palacio, Armería nivel 2, Barracón nivel 2, Galería de tiro nivel 2). Auto-construcción (ver arriba).
- Requisito construcción: Asentamiento nivel 2. Requisito nivel 2: Asentamiento nivel 3.
- Nivel 1: recluta Arietes. Nivel 2: recluta Torre de asedio.
- PENDIENTE: costo/tiempo/trabajadores de construcción — sin cifra en el diseño original, solo el gate de nivel.

**Barracón** — reclutamiento de tropas cuerpo a cuerpo (unidades y costo en equipo: ver Doc 5). Construcción vía política del General (ver 4.4), no auto-construcción.
- Requisito nivel 2: Asentamiento nivel 2 + poseer Carpintería. Requisito nivel 3: Asentamiento nivel 3 + poseer Palacio.
- Costo: construcción 30 madera; mejora 1: 100 madera + 60 piedra; mejora 2: 300 madera + 200 piedra. Tiempo de construcción: 6 ticks.

**Galería de tiro** — reclutamiento de tropas a distancia (unidades y costo en equipo: ver Doc 5). Construcción vía política del General (ver 4.4), no auto-construcción.
- Requisito nivel 2: Asentamiento nivel 2 + poseer Carpintería. Requisito nivel 3: Asentamiento nivel 3 + Carpintería nivel 2 (asimetría INTENCIONAL respecto a Armería/Barracón, que piden Palacio — Galería de tiro sigue su propio camino de progresión, no se uniforma).
- Costo: construcción 50 madera; mejora 1: 140 madera + 20 piedra; mejora 2: 400 madera + 100 piedra. Tiempo de construcción: 6 ticks.

## 4.3 Almacenamiento
Límites de almacenaje por recurso, ampliables construyendo más capacidad. El superávit que excede el límite dispara construcción automática de más almacenamiento.

## 4.4 Políticas (mecanismo de influencia del jugador)
- Interfaz: decisiones DISCRETAS tipo menú (no sliders continuos).
- SLOTS Y POOLS POR CARGO:
  - Gobernador: 2 slots (escala hasta 5 según nivel de Facción), pool COMPLETA (todas las políticas).
  - Tesorero: 2 slots, pool de economía/comercio/mercados/precios/comisiones/caravanas/acuerdos.
  - Maestro de Obras: 1 slot, pool de auto-construcción/edificios.
  - General: 1 slot, pool de defensa/reclutamiento/fabricación de armas y armaduras.
  - Sacerdote: 1 slot, pool de felicidad/cultura/población.
  - TOTAL con todos los cargos ocupados: 7 políticas simultáneas máximo — incentiva mecánicamente la colaboración multijugador.
- Reglas: duración determinada, renovable indefinidamente, NO cancelable antes de tiempo (hay que esperar a que termine para renovar o cambiar).
- **"Protección de Riesgos"** (Maestro de Obras, implementada como ejemplo del pool de auto-construcción tras un rebalance posterior a Sprint 6): mientras esté activa, la auto-construcción ignora cualquier otra necesidad hasta tener **2 Leñeras y 3 Granjas** (activas o en curso/cola) — modo de emergencia ante escasez de madera/comida, ver 4.2.
- **"Edicto de Cosecha"** (Gobernador, nueva, a petición del usuario): multiplica ×1.5 la producción de trigo de todas las Granjas activas del asentamiento. No afecta a madera ni piedra. Ver 4.2.1 (Granja) y 4.5 (panel de producción).
- **"Racionamiento"** (Sacerdote, implementada, pendiente de haber sido documentada aquí): reduce el consumo de trigo de la población (×0.8). No afecta al consumo/ración de tropas (Doc 5.4). Ver 4.1.
- **Redistribución de Vivienda** (IDEA nueva, a petición del usuario, NO implementada todavía): política que permitiría modificar la proporción fija 15/5 de cupo Pesants/Artesanos dentro de cada Vivienda (ej. favorecer más Artesanos en un asentamiento orientado a producción especializada). Pool y cargo responsable sin definir todavía — candidato natural: Maestro de Obras (mismo pool que Vivienda) o Sacerdote (mismo pool que gestiona población, ver 4.1). PENDIENTE: mecánica exacta (¿desplaza cupo de una clase a otra manteniendo el total, o añade cupo extra?), valores numéricos, y a qué cargo pertenece.
- **Desbloqueo de edificios especiales** (nuevo, rediseño Fase 0): 3 políticas nuevas, una por edificio, cada una en el pool del cargo indicado (ver catálogo completo en 4.2.1). Mientras la política esté activa, la construcción del edificio correspondiente salta la cola normal de auto-construcción/manual (tope de 3 slots, ver 4.2) y usa un CLUSTER DE COLA APARTE dedicado solo a estos 3 edificios:
  - "Construir Barracón" — pool General.
  - "Construir Galería de tiro" — pool General.
  - "Construir Palacio" — pool Gobernador.
  Curtiduría/Armería/Fundición/Carpintería NO usan este mecanismo — son auto-construcción normal (ver 4.2), compitiendo por la cola de 3 slots como cualquier otro edificio.
- PENDIENTE: catálogo concreto de políticas dentro de cada pool (más allá de los ejemplos ya implementados); si son excluyentes entre sí dentro de un slot.

## 4.5 Mantenimiento de asentamientos (sistema unificado, incluye ex-"Coste de Gobernanza")
- **NIVEL DE ASENTAMIENTO — modelo por gates (rediseño Fase 0, reemplaza el placeholder anterior de fórmula de puntos por población+edificios)**: tope de Fase 0 = NIVEL 3. Sube por condiciones combinadas (población Y edificios específicos construidos), no por una fórmula continua:
  - Nivel 2: 200 pesants + 50 artesanos + tener construidas Armería, Curtiduría y Fundición.
  - Nivel 3: 500 pesants + 200 artesanos + tener construidas Carpintería, Barracón y Galería de tiro.
  El nivel alimenta: el techo de zona de influencia (Doc 1.2), el coste de Mantenimiento de este apartado, y el tope de extractores de 4.2 (ahora desacoplado del nivel, ver 4.2). Nota: esto es el nivel de ASENTAMIENTO — distinto del nivel de FACCIÓN (Doc 1.7), que sigue con su propio criterio placeholder sin tocar.
- MEDIDOR 0-100 por asentamiento, empieza en 100.
- **Período de gracia al fundar** (confirmado durante implementación, resuelve pregunta antes pendiente — ver Doc 1.3): durante un número de ticks tras la fundación, NO se cobra mantenimiento. Sin esto, todo asentamiento nuevo caía en ruinas de forma sistemática (~9 ticks) antes de tener Granja/trigo, sin importar la gestión.
- COSTE PERIÓDICO en recursos + oro, que escala por (a) NIVEL del asentamiento y (b) DISTANCIA al centro de poder de la Facción (más lejos = más caro; mecanismo anti-snowball).
- Escalado del coste por nivel (se van SUMANDO materiales, no reemplazando) — **umbrales confirmados**:
  - Nivel 1: madera.
  - Nivel 2: madera + piedra.
  - Nivel 3 (tope real de Fase 0): madera + piedra + oro.
  - Niveles 4-5 (planeados, NO implementados en Fase 0, ver Doc 1.2 y `Correcciones_Durante_Desarrollo.md`): fuera de alcance, sin escalado definido todavía.
- **Trigo — fix de mecánica repetida (detectado jugando, ver `Correcciones_Durante_Desarrollo.md`)**: el trigo YA NO forma parte del coste periódico anterior. Antes, Mantenimiento cobraba un valor fijo de trigo (placeholder desconectado de la realidad) ADEMÁS del consumo real de comida de población + tropas, que ya se descontaba por separado cada tick — un doble descuento sobre el mismo recurso por dos razones que en el fondo eran la misma ("alimentar al asentamiento"). Ahora el "apartado de trigo" que se muestra en el panel de Mantenimiento es directamente `consumo de comida de la población (4.1) + ración de tropas (Doc 5.4)` — el valor real, sin placeholder — pero se sigue descontando UNA sola vez, donde siempre se descontó (población/tropas), no aquí. Efecto colateral a tener en cuenta: un déficit de trigo YA NO degrada este medidor directamente — solo frena el crecimiento de población (factor de comida en 4.1); el medidor de Mantenimiento ahora depende solo de madera (+piedra/oro por nivel).
- Si NO se cumple algún pago (madera/piedra/oro), el medidor BAJA de 100 a 0 de forma PROPORCIONAL al déficit (degradación gradual, no corte binario).
- Al llegar a 0: el asentamiento se DESTRUYE y cae en RUINAS → se limpia la zona → queda disponible para otro jugador/grupo. Esta es la MISMA ruta mecánica que el caso de abandono total (sea el asentamiento literalmente abandonado o simplemente mal gestionado mientras sigue activo).
- **Calibración** (ajustada durante implementación, sigue siendo placeholder): el coste base de madera y la velocidad de degradación se redujeron respecto a la versión inicial (que generaba espiral de déficit incluso en asentamientos bien gestionados); se subió también la velocidad de regeneración cuando el pago es íntegro. Producción base de Granja recalibrada dos veces (5→10→15 trigo/tick, ver 4.2.1), combinada con el disparador de Granja basado en déficit real (producción < consumo, hasta 3 Granjas a la vez en déficit, ver 4.2) y la política "Edicto de Cosecha" (×1.5, ver 4.4) — conjunto que mitiga bastante el desajuste entre población exponencial y producción de Granja, aunque no lo elimina del todo en fundaciones con fertilidad baja.
- PENDIENTE: cantidades exactas finales por nivel, velocidad exacta de degradación/regeneración, duración exacta del período de gracia inicial, y el número fijo de extractores por tipo (RESUELTO el umbral piedra/oro por nivel, ver escalado arriba; sigue pendiente el número fijo de extractores) — todo sigue siendo ajustable, pendiente de nueva calibración por simulación tras el rediseño de progreso (el validado en pruebas de 150-300 ticks corresponde al modelo anterior).

## 4.6 Entrada tardía y mundo lleno
El mapa es deliberadamente difícil de saturar por completo. Cuando un servidor se llena lo suficiente, se abre uno nuevo. El deterioro por mal mantenimiento/abandono libera continuamente zonas para nuevos jugadores.
