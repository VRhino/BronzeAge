# 4. Sistema de Población, Construcción Automática y Mantenimiento

## 4.1 Población NPC: 3 clases
Los JUGADORES son una categoría SEPARADA de estas 3 clases.

| Clase | Rol | Aparición | Crecimiento |
|---|---|---|---|
| Pesants | Trabajan recursos genéricos (granjas, canteras, bosques); tropas básicas | Desde la fundación, sin condición | Rápido, fórmula propia |
| Artesanos | Operan edificios de producción especializada (Fundición/Broncista, Curtidor-Armero, Carpintería); tropas Tier 2 | Automática, al construirse el primer edificio que los requiere | Fórmula propia independiente (pendiente detalle numérico) |
| Nobleza | Tropas de élite (vía equipo, sin grindeo) | Cantidad MÍNIMA de ciudadanos (jugadores) en el asentamiento (único requisito) | Muy lento; acelerado por el Sacerdote (no requisito) |

Cada clase tiene FÓRMULA DE CRECIMIENTO INDEPENDIENTE (no comparten los mismos pesos). Fórmula base de referencia (Pesants): comida disponible + vivienda disponible + estabilidad + felicidad.

**Reclutamiento militar (doble carril):**
- Carril COMBATE REAL (Pesants + Artesanos): deben salir a combatir (PvP o PvE) para "veteranizar" y subir de tier.
- Carril PROGRESIÓN PLANA (Nobleza): conversión INSTANTÁNEA a unidades de élite si hay equipo disponible.
- Cola de PRIORIDAD cuando la demanda excede el pool disponible (criterio exacto pendiente).

## 4.2 Auto-construcción por necesidad
- El jugador NO elige ubicación ni tipo de edificio, EXCEPTO: edificio de fundación, y edificios estratégicos (murallas, torres, puerto) que sí se colocan manualmente.
- Algoritmo de colocación: reglas por tipo de edificio (ej. granja cerca de tierra fértil, herrería cerca de mina+camino), elige la mejor casilla disponible dentro de la zona de influencia.
- Crecimiento disparado por NECESIDAD REAL: más población → necesidad de comida → granja automática; excedente de recurso → mercado/almacén.
- **Escalado por demanda continua** (confirmado durante implementación de Fase 0): la construcción automática no se limita a "construir una vez si no existe ninguna" — vuelve a evaluarse continuamente. Ej. Granja: si la reserva de trigo proyectada (trigo disponible ÷ consumo actual) cae por debajo de un umbral, se encola una Granja ADICIONAL, incluso si ya existe al menos una (bug detectado en Sprint 2: sin esto, la población entraba en hambruna silenciosa al crecer más allá de lo que una sola Granja podía sostener).
- **Reemplazo de fuentes agotadas** (nueva mecánica, confirmada durante implementación): los extractores de recursos finitos (cantera, mina de oro, mina de cobre, **mina de estaño**) pueden agotar su nodo fuente. La lógica de disparo cuenta cuántos extractores tienen FUENTE VIVA (no solo cuántos existen en total) — si un yacimiento se agota, se encola automáticamente un extractor de reemplazo (buscando un nuevo nodo del mismo recurso), hasta un MÁXIMO ligado al NIVEL DEL ASENTAMIENTO. Mismo criterio aplicado a cantera, mina de oro, mina de cobre, mina de estaño y lenera (bosques también se agotan). Sin esto, agotar el único yacimiento condenaba al asentamiento a un déficit permanente e irreversible.
- **Mina de estaño** (rebalance posterior a Sprint 6): el estaño se genera en el mundo con el mismo criterio de rareza que cobre/oro (Doc 1.1) desde el Sprint 1, pero no tuvo edificio de extracción propio hasta esta corrección — hasta entonces solo era obtenible por trueque. Mismo patrón que cantera/mina/mina de cobre.
- Excepción permanente: el COMERCIO/CARAVANAS nunca se crea automáticamente, siempre es acción manual del jugador.
- Cadenas de producción (materia prima → producto) son lógica INTERNA invisible — el jugador solo ve materiales almacenados y necesidades activas (déficits).
- Cargo MAESTRO DE OBRAS gestiona las prioridades de auto-construcción, da bonus a tiempos de construcción.
- Patrón de crecimiento: las ciudades agregan edificios desde el CENTRO hacia afuera (concéntrico).
- LAYOUT DINÁMICO según política activa: ej. política DEFENSIVA prioriza edificios defensivos y layout fácil de defender (con coste real: pierde eficiencia económica, genera "pasillos" hacia puntos de captura). Elimina la repetición visual entre asentamientos.
- **Tope de cola** (rebalance posterior a Sprint 6): máximo 3 edificios `en_cola` simultáneos por asentamiento, compartido entre auto-construcción y construcción manual (Fundición/Gran Fundición).
- **Orden de prioridad e interbloqueo de recursos de supervivencia** (bug real detectado jugando, ver `Correcciones_Durante_Desarrollo.md`): con el tope de cola, un proyecto que no puede pagarse se queda parado en cola indefinidamente (nada lo saca si nunca junta recursos). Si Vivienda/Granja/Cantera —que se re-disparan casi todos los ticks— se evaluaban antes que la Leñera, podían copar los 3 slots con proyectos atascados y dejar a la Leñera (única fuente de madera) sin hueco para encolarse NUNCA, condenando al asentamiento a un déficit de madera permanente que Mantenimiento acababa castigando hasta la destrucción. Corrección: (a) Granja/Leñera (recursos de supervivencia) se evalúan primero, extractores secundarios después, y Vivienda/Almacén/Taller (crecimiento) al final; (b) de los 3 slots de cola, 1 queda reservado exclusivamente para Granja/Leñera, así el resto nunca puede dejarlas sin hueco.
- **Reserva mínima de construcción** (nueva mecánica): un edificio en cola solo empieza a construirse si, además de poder pagar su costo, no deja ningún recurso que en ese momento cobre Mantenimiento (madera+trigo siempre; +piedra desde nivel 3; +oro desde nivel 8) por debajo de una reserva mínima (placeholder: madera 30, trigo 20, piedra 20, oro 10). Excepción: Granja no respeta la reserva de trigo, ni Leñera la de madera, porque son la única vía real de recuperar esos recursos — bloquearlas por la misma escasez que deben resolver sería otro interbloqueo sin salida.
- **Política "Protección de Riesgos"** (Maestro de Obras, catálogo de 4.4): mientras esté activa, bloquea toda auto-construcción que no sea Leñera hasta llegar a un mínimo de 3 (activas o en curso/cola) — salvo que no haya ningún bosque alcanzable todavía, en cuyo caso deja pasar la evaluación normal para no bloquearse sin salida.

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
- PENDIENTE: catálogo concreto de políticas dentro de cada pool (más allá de los ejemplos ya implementados); si son excluyentes entre sí dentro de un slot.

## 4.5 Mantenimiento de asentamientos (sistema unificado, incluye ex-"Coste de Gobernanza")
- MEDIDOR 0-100 por asentamiento, empieza en 100.
- **Período de gracia al fundar** (confirmado durante implementación, resuelve pregunta antes pendiente — ver Doc 1.3): durante un número de ticks tras la fundación, NO se cobra mantenimiento. Sin esto, todo asentamiento nuevo caía en ruinas de forma sistemática (~9 ticks) antes de tener Granja/trigo, sin importar la gestión.
- COSTE PERIÓDICO en recursos + oro, que escala por (a) NIVEL del asentamiento y (b) DISTANCIA al centro de poder de la Facción (más lejos = más caro; mecanismo anti-snowball).
- Escalado del coste por nivel (se van SUMANDO materiales, no reemplazando):
  - Niveles iniciales: madera + comida.
  - Niveles medios: sube cantidad + se añade piedra.
  - Niveles avanzados: sube cantidad de nuevo + se añade oro. **Ajustado durante implementación**: el umbral de nivel en que empieza a exigirse oro se retrasó de NIVEL 6 a NIVEL 8, para evitar condenar de forma temprana a asentamientos aislados sin mina de oro propia (recurso raro por diseño).
  - Niveles tardíos: todos los materiales anteriores simultáneos, escalando.
- Si NO se cumple algún pago, el medidor BAJA de 100 a 0 de forma PROPORCIONAL al déficit (degradación gradual, no corte binario).
- Al llegar a 0: el asentamiento se DESTRUYE y cae en RUINAS → se limpia la zona → queda disponible para otro jugador/grupo. Esta es la MISMA ruta mecánica que el caso de abandono total (sea el asentamiento literalmente abandonado o simplemente mal gestionado mientras sigue activo).
- **Calibración** (ajustada durante implementación, sigue siendo placeholder): el coste base de trigo/madera y la velocidad de degradación se redujeron respecto a la versión inicial (que generaba espiral de déficit incluso en asentamientos bien gestionados); se subió también la velocidad de regeneración cuando el pago es íntegro.
- PENDIENTE: cantidades exactas finales por nivel, velocidad exacta de degradación/regeneración, duración exacta del período de gracia inicial — todo sigue siendo ajustable, validado solo como jugable en pruebas de 150-300 ticks.

## 4.6 Entrada tardía y mundo lleno
El mapa es deliberadamente difícil de saturar por completo. Cuando un servidor se llena lo suficiente, se abre uno nuevo. El deterioro por mal mantenimiento/abandono libera continuamente zonas para nuevos jugadores.
