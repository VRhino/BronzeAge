# 4. Sistema de Población, Construcción Automática y Mantenimiento

## 4.1 Población NPC: 3 clases
Los JUGADORES son una categoría SEPARADA de estas 3 clases.

| Clase | Rol | Aparición | Crecimiento |
|---|---|---|---|
| Pesants | Trabajan recursos genéricos (granjas, canteras, bosques); tropas básicas | Desde la fundación, sin condición | Rápido, fórmula propia |
| Artesanos | Operan edificios de producción especializada (Fundición, Curtiduría, Armería, Carpintería — ver catálogo en 4.2); tropas vía Barracón/Galería de tiro (Doc 5) | Automática: dispara el PRIMERO que se construya entre Curtiduría/Armería/Fundición/Carpintería | Fórmula propia independiente (pendiente detalle numérico); tope de población limitado (ver nota abajo) |
| Nobleza | Tropas de élite (vía equipo, sin grindeo) | Cantidad MÍNIMA de ciudadanos (jugadores) en el asentamiento (único requisito) | Muy lento; acelerado por el Sacerdote (no requisito) |

Cada clase tiene FÓRMULA DE CRECIMIENTO INDEPENDIENTE (no comparten los mismos pesos). Fórmula base de referencia (Pesants): comida disponible + vivienda disponible + estabilidad + felicidad.

**Disparador y tope de Artesanos (rediseño de progreso, Fase 0)**: se retira el antiguo edificio genérico "Taller". La aparición de Artesanos la dispara el PRIMERO que se construya entre Curtiduría, Armería, Fundición o Carpintería (ver catálogo completo en 4.2); a partir de ahí, el tope de población de Artesanos es la suma de `trabajadoresRequeridos` de todos los edificios de transformación activos según su nivel interno — mismo espíritu que el ratio de mano de obra de Pesants sobre granja/cantera/minas. Sin ningún edificio de transformación activo, no crecen artesanos nuevos.

**Reclutamiento militar (doble carril):**
- Carril COMBATE REAL (Pesants + Artesanos): deben salir a combatir (PvP o PvE) para "veteranizar" y subir de tier.
- Carril PROGRESIÓN PLANA (Nobleza): conversión INSTANTÁNEA a unidades de élite si hay equipo disponible.
- Cola de PRIORIDAD cuando la demanda excede el pool disponible (criterio exacto pendiente).

## 4.2 Auto-construcción por necesidad
- El jugador NO elige ubicación ni tipo de edificio, EXCEPTO: edificio de fundación, y edificios estratégicos (murallas, torres, puerto) que sí se colocan manualmente.
- Algoritmo de colocación: reglas por tipo de edificio (ej. granja cerca de tierra fértil, herrería cerca de mina+camino), elige la mejor casilla disponible dentro de la zona de influencia.
- Crecimiento disparado por NECESIDAD REAL: más población → necesidad de comida → granja automática; excedente de recurso → mercado/almacén.
- **Escalado por demanda continua** (confirmado durante implementación de Fase 0): la construcción automática no se limita a "construir una vez si no existe ninguna" — vuelve a evaluarse continuamente. Ej. Granja: si la reserva de trigo proyectada (trigo disponible ÷ consumo actual) cae por debajo de un umbral, se encola una Granja ADICIONAL, incluso si ya existe al menos una (bug detectado en Sprint 2: sin esto, la población entraba en hambruna silenciosa al crecer más allá de lo que una sola Granja podía sostener).
- **Reemplazo de fuentes agotadas** (nueva mecánica, confirmada durante implementación): los extractores de recursos finitos (cantera, mina de oro, mina de cobre, **mina de estaño**, **Corral**) pueden agotar su nodo fuente. La lógica de disparo cuenta cuántos extractores tienen FUENTE VIVA (no solo cuántos existen en total) — si un yacimiento se agota, se encola automáticamente un extractor de reemplazo (buscando un nuevo nodo del mismo recurso), hasta un MÁXIMO por tipo de extractor. Mismo criterio aplicado a cantera, mina de oro, mina de cobre, mina de estaño, lenera y Corral (bosques también se agotan). Sin esto, agotar el único yacimiento condenaba al asentamiento a un déficit permanente e irreversible. **Rediseño Fase 0**: este máximo se DESACOPLA del nivel del asentamiento (antes escalaba 1:1 con él) — con el tope de nivel bajando a 3 (ver 4.5), un máximo ligado al nivel se quedaría corto; pasa a ser un número fijo por tipo de extractor, cifra exacta PENDIENTE de calibración por simulación.
- **Corral** (nuevo edificio, rediseño Fase 0): extractor de livestock, mismo patrón que cantera/minas — liga a un nodo finito de livestock (Doc 1.4, ya generado por el mundo), con reemplazo automático al agotarse. Ver costo/producción en el catálogo de 4.2.1.
- **Curtiduría / Armería / Fundición / Carpintería** (rediseño Fase 0): van por AUTO-CONSTRUCCIÓN, igual que Granja/Cantera — se disparan en cuanto se cumple su requisito de nivel de asentamiento/edificio previo (ver catálogo en 4.2.1), sin exigir Planos/Aedas en Fase 0 (Doc 6 pospone ese sistema por completo). No van vía política. La Fundición aquí reemplaza y amplía la Fundición manual ya existente (antes sin producción propia, solo gate de ascenso de tropa — Doc 5.8): mismo edificio, ahora con recetas reales y disparo automático en vez de manual.
- **Barracón / Galería de tiro / Palacio** (rediseño Fase 0): a diferencia del resto, van vía POLÍTICA dedicada (ver 4.4) — mientras la política correspondiente esté activa, la construcción de ese edificio salta la cola normal de 3 slots y usa un cluster de cola aparte.
- **Cantera y Gran Fundición**: sin cambios respecto a la versión ya implementada — Gran Fundición queda para iteraciones posteriores (integración con la nueva Fundición fuera de alcance de este rediseño).
- **Mina de estaño** (rebalance posterior a Sprint 6): el estaño se genera en el mundo con el mismo criterio de rareza que cobre/oro (Doc 1.1) desde el Sprint 1, pero no tuvo edificio de extracción propio hasta esta corrección — hasta entonces solo era obtenible por trueque. Mismo patrón que cantera/mina/mina de cobre.
- Excepción permanente: el COMERCIO/CARAVANAS nunca se crea automáticamente, siempre es acción manual del jugador.
- Cadenas de producción (materia prima → producto) son lógica INTERNA invisible — el jugador solo ve materiales almacenados y necesidades activas (déficits).
- Cargo MAESTRO DE OBRAS gestiona las prioridades de auto-construcción, da bonus a tiempos de construcción.
- Patrón de crecimiento: las ciudades agregan edificios desde el CENTRO hacia afuera (concéntrico).
- LAYOUT DINÁMICO según política activa: ej. política DEFENSIVA prioriza edificios defensivos y layout fácil de defender (con coste real: pierde eficiencia económica, genera "pasillos" hacia puntos de captura). Elimina la repetición visual entre asentamientos.
- **Tope de cola** (rebalance posterior a Sprint 6): máximo 3 edificios `en_cola` simultáneos por asentamiento, compartido entre auto-construcción y construcción manual (Gran Fundición — la única que queda de colocación manual clásica). Barracón/Galería de tiro/Palacio (rediseño Fase 0, ver arriba) NO comparten este cupo: usan su propio cluster de cola aparte, condicionado a la política correspondiente (ver 4.4).
- **Orden de prioridad e interbloqueo de recursos de supervivencia** (bug real detectado jugando, ver `Correcciones_Durante_Desarrollo.md`): con el tope de cola, un proyecto que no puede pagarse se queda parado en cola indefinidamente (nada lo saca si nunca junta recursos). Si Vivienda/Granja/Cantera —que se re-disparan casi todos los ticks— se evaluaban antes que la Leñera, podían copar los 3 slots con proyectos atascados y dejar a la Leñera (única fuente de madera) sin hueco para encolarse NUNCA, condenando al asentamiento a un déficit de madera permanente que Mantenimiento acababa castigando hasta la destrucción. Corrección: (a) Granja/Leñera (recursos de supervivencia) se evalúan primero, extractores secundarios después, y Vivienda/Almacén/Taller (crecimiento) al final; (b) de los 3 slots de cola, 1 queda reservado exclusivamente para Granja/Leñera, así el resto nunca puede dejarlas sin hueco.
- **Reserva mínima de construcción** (nueva mecánica): un edificio en cola solo empieza a construirse si, además de poder pagar su costo, no deja ningún recurso que en ese momento cobre Mantenimiento (madera+trigo siempre; +piedra desde nivel 3; +oro desde nivel 8) por debajo de una reserva mínima (placeholder: madera 30, trigo 20, piedra 20, oro 10). Excepción: Granja no respeta la reserva de trigo, ni Leñera la de madera, porque son la única vía real de recuperar esos recursos — bloquearlas por la misma escasez que deben resolver sería otro interbloqueo sin salida.
- **Política "Protección de Riesgos"** (Maestro de Obras, catálogo de 4.4): mientras esté activa, bloquea toda auto-construcción que no sea Leñera hasta llegar a un mínimo de 3 (activas o en curso/cola) — salvo que no haya ningún bosque alcanzable todavía, en cuyo caso deja pasar la evaluación normal para no bloquearse sin salida.

## 4.2.1 Catálogo completo de edificios (rediseño Fase 0)

Fase 0 descarta por completo los requisitos de "Planos de X" (vía Aedas, Doc 6) en cualquier mejora de nivel de edificio — el sistema de Aedas no tiene implementación real todavía (ver Doc 6). Los requisitos de NIVEL DE ASENTAMIENTO sí aplican (modelo de gates, ver 4.5); tope de Fase 0 = nivel 3.

### Alojamiento

**Palacio** — desbloquea la aparición de Nobleza. Construcción vía política del Gobernador (ver 4.4), no auto-construcción.
- Requisito: Asentamiento nivel 3.
- Capacidad: 200 nobles. Costo: 1500 madera + 1000 piedra. Tiempo: 20 ticks.

**Vivienda** — auto-construcción, sin cambios de diseño respecto a la versión ya implementada.
- Costo: 10 madera. Capacidad: 15 (Pesants y Artesanos). Tiempo: 4 ticks.

### Extracción de recursos

| Edificio | Recurso | Costo | Trabajadores | Tiempo | Producción base |
|---|---|---|---|---|---|
| Corral (nuevo) | Livestock | 30 madera | 4 | 6 ticks | 3 livestock |
| Granja | Trigo | 30 madera | 4 | 6 ticks | 5 trigo |
| Leñera | Madera | 10 madera | 4 | 3 ticks | 5 madera |
| Mina de oro | Oro | 40 madera + 10 piedra | 6 | 6 ticks | 2 oro |
| Mina de cobre | Cobre | 30 madera + 5 piedra | 8 | 4 ticks | 5 cobre |
| Mina de estaño | Estaño | 60 madera + 20 piedra | 8 | 7 ticks | 1.5 estaño |

Corral sigue el mismo patrón que Cantera/minas (ver 4.2): liga a un nodo finito de livestock (Doc 1.4), con reemplazo automático al agotarse. Cantera (piedra) no cambia respecto a la versión ya implementada — no está en esta tabla porque sus cifras siguen igual.

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
- **"Protección de Riesgos"** (Maestro de Obras, implementada como ejemplo del pool de auto-construcción tras un rebalance posterior a Sprint 6): mientras esté activa, la auto-construcción ignora cualquier otra necesidad hasta tener 3 Leñeras (activas o en curso/cola) — modo de emergencia ante escasez de madera, ver 4.2.
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
- Escalado del coste por nivel (se van SUMANDO materiales, no reemplazando):
  - Niveles iniciales: madera + comida.
  - Niveles medios: sube cantidad + se añade piedra.
  - Niveles avanzados: sube cantidad de nuevo + se añade oro. **Recalibrado para el tope de Fase 0 (nivel 3, rediseño de progreso)**: con el nivel máximo bajando de 10 a 3, los umbrales de piedra y oro (antes nivel 3 y nivel 8 respectivamente, pensados para un rango 1-10) se recalibran al rango 1-3 para que los 3 niveles tengan una escalada de coste real — en qué nivel exacto empieza cada uno queda PENDIENTE de calibración por simulación.
  - Niveles tardíos (fuera de alcance de Fase 0 con el tope actual de nivel 3): todos los materiales anteriores simultáneos, escalando.
- Si NO se cumple algún pago, el medidor BAJA de 100 a 0 de forma PROPORCIONAL al déficit (degradación gradual, no corte binario).
- Al llegar a 0: el asentamiento se DESTRUYE y cae en RUINAS → se limpia la zona → queda disponible para otro jugador/grupo. Esta es la MISMA ruta mecánica que el caso de abandono total (sea el asentamiento literalmente abandonado o simplemente mal gestionado mientras sigue activo).
- **Calibración** (ajustada durante implementación, sigue siendo placeholder): el coste base de trigo/madera y la velocidad de degradación se redujeron respecto a la versión inicial (que generaba espiral de déficit incluso en asentamientos bien gestionados); se subió también la velocidad de regeneración cuando el pago es íntegro.
- PENDIENTE: cantidades exactas finales por nivel, en qué nivel exacto (1-3) empiezan a exigirse piedra y oro, velocidad exacta de degradación/regeneración, duración exacta del período de gracia inicial, y el tope exacto de extractores por tipo (ver 4.2 — desacoplado del nivel de asentamiento, número fijo aún sin cerrar) — todo sigue siendo ajustable, pendiente de nueva calibración por simulación tras el rediseño de progreso (el validado en pruebas de 150-300 ticks corresponde al modelo anterior).

## 4.6 Entrada tardía y mundo lleno
El mapa es deliberadamente difícil de saturar por completo. Cuando un servidor se llena lo suficiente, se abre uno nuevo. El deterioro por mal mantenimiento/abandono libera continuamente zonas para nuevos jugadores.
