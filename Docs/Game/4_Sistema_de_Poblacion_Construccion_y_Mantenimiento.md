# 4. Sistema de Población, Construcción Automática y Mantenimiento

## 4.1 Población NPC: 3 clases
Los HÉROES son una categoría SEPARADA de estas 3 clases.

| Clase | Rol | Aparición | Crecimiento |
|---|---|---|---|
| Pesants | Trabajan recursos genéricos (granjas, canteras, bosques); reclutan la tropa de escalón 1 y 2 (Doc 5.8) | Desde la fundación, sin condición | Rápido (tasa 0.12) |
| Artesanos | Operan edificios de producción especializada (Fundición, Curtiduría, Armería, Carpintería — ver catálogo en 4.2.1); reclutan la tropa de escalón 3 y los Falangitas (Doc 5.8) | En cuanto hay un edificio de transformación activo (Curtiduría/Armería/Fundición/Carpintería) aparece el primero | Medio (tasa 0.15), limitado por su cupo de Vivienda |
| Nobleza | Clase rica: la que más oro recauda (abajo). Recluta la tropa de escalón 4 y 5 (Doc 5.8) | Palacio construido + cantidad MÍNIMA de ciudadanos (héroes) en el asentamiento | Muy lento (tasa 0.01); acelerado por el Sacerdote (no requisito) |

**Las 3 clases crecen con la MISMA fórmula**: `comida disponible × cupo libre de la clase × tasa propia`, y difieren solo en la tasa. Estabilidad y felicidad valen 1 (placeholder). Cada clase tiene su propio cupo: los Pesants y los Artesanos, en la Vivienda (cupos SEPARADOS por clase, 4.2.1), para que los Pesants, que crecen más deprisa, no dejen sin sitio a los Artesanos; la Nobleza, en el Palacio. Por encima de todo, el nivel del asentamiento fija un techo de población total (4.5).

**Consumo de comida (trigo)**: tasa FIJA por habitante, sumada por el total de habitantes (pesants+artesanos+nobleza) — no depende de la clase. Reducible por la política Racionamiento (Sacerdote, ×0.8). Este consumo se suma al de las tropas (raciones de escuadrones, Doc 5.4) para formar el "apartado de trigo" que se muestra en el panel de Mantenimiento (ver 4.5).

**Hambruna — el coste de no sostener el consumo de trigo** (espejo deliberado de la deserción de tropas por falta de ración, Doc 5.4). Cada asentamiento lleva un medidor **`nutricionPoblacion` (0-100, empieza en 100)**, separado del medidor de Mantenimiento (4.5) y de la felicidad — la nutrición es solo comida, no bienestar general:
- Cada minuto, si el trigo disponible cubre el consumo completo, la nutrición sube +5 (tope 100); si no lo cubre, baja `20 × (1 − fracción cubierta)` — con déficit total (trigo en 0), la nutrición colapsa de 100 a 0 en **5 minutos**, el mismo ritmo que la moral militar sin ración (Doc 5.4).
- El factor de crecimiento de las 3 clases escala LINEALMENTE entre 0.2 (nutrición 0) y 1 (nutrición 100): una hambruna que se resuelve rápido apenas frena el crecimiento; una sostenida lo frena casi del todo.
- Mientras la nutrición se MANTIENE en 0 (hambre sostenida, no solo un bache puntual), cada minuto cuesta una fracción real de población — **5% de pesants+artesanos**, el mismo valor que la deserción de tropas sin moral, y **1% de la Nobleza**, la más resistente.
- Cifras placeholder (`POBLACION.hambre`).

**Recaudación de oro por clase** (`Consideraciones/Economia_Del_Oro_Definicion.md`):
- Cada minuto, el asentamiento **genera oro según su población y de qué clase es** (`recaudacionOro`) — espejo exacto del consumo de comida, con signo opuesto: `Σ(habitantes_clase × IMPUESTOS.tasa_clase)`, sumado al almacén respetando su capacidad (el sobrante se pierde, igual que la producción de mina).
- **Nobleza > Artesanos > Pesants** por cabeza (`IMPUESTOS`, placeholder: 0.06 / 0.015 / 0.004 oro/min). Base imponible por riqueza — crecer Nobleza = asentamiento rico; refuerza el valor del Palacio.
- **NO escala por distancia a la capital ni por nivel** — el eje de distancia ya es el "impuesto de cohesión" del lado del coste (Mantenimiento, 4.5). Dejar el ingreso plano hace que el impuesto premie crecer alto y cohesionado.
- Estabilidad y felicidad valen 1 (placeholder), igual que en el crecimiento.
- Se recauda antes de cobrar el mantenimiento, de modo que el oro recién recaudado paga el mantenimiento del mismo minuto.
- Modulable por la política **"Presión Fiscal"** del Tesorero (4.4).

**Reclutamiento militar**: solo se reclutan Pesants y Artesanos, pagando equipo (Doc 5.8), y sus escuadrones suben de nivel y experiencia combatiendo (Doc 5.16.3).
## 4.2 Auto-construcción por necesidad
- El jugador NO elige la UBICACIÓN de los edificios, salvo el de fundación y los estratégicos (murallas, torres, puerto), que se colocan a mano. Controlar la cola (abajo) cambia QUÉ se construye y en qué ORDEN, nunca DÓNDE.
- **Control manual de la cola de auto-construcción** (inspirado en Travian): Gobernador y Maestro de Obras (ver Doc 2.2) ven la cola completa (`en_cola`) desde su interfaz, REORDENAN las entradas ya encoladas (arriba/abajo), y AÑADEN o QUITAN proyectos manualmente. Quitar un proyecto devuelve lo que se pagó por él: la obra y, si es un edificio dañado que esperaba reconstrucción, también las mejoras que lo llevaron a su nivel (decidido el 2026-09-27).
  - **Añadir**: CUALQUIER edificio del catálogo (salvo Centro Urbano, que nunca pasa por cola) se puede añadir manualmente, incluidos Barracón/Galería de tiro/Palacio/Mercado, que solo llegan por esta vía. Todos compiten por el mismo cupo `NECESIDADES.maximoEnCola`. Respeta los requisitos de nivel/edificio previo del catálogo (4.2.1) y la reserva mínima de Mantenimiento — solo se añade "si el asentamiento puede pagarlo ahora mismo".
  - **Quitar**: SOLO un proyecto `en_cola` (no `en_construccion` — nunca se puede cancelar una vez arrancada la obra). Devuelve el COSTE COMPLETO, que se pagó al encolar.
  - **Reordenar**: mueve un proyecto `en_cola` una posición arriba/abajo respecto al resto de la cola.
- Algoritmo de colocación: reglas por tipo de edificio (ej. granja cerca de tierra fértil, herrería cerca de mina+camino), elige la mejor casilla disponible dentro de la zona de influencia. Detalle completo del trazado urbano (barrios, calles, manzanas, tamaños) en `Consideraciones/Vista_Asentamiento_Trazado_Urbano.md`.
- **Dos espacios lógicos separados** (ver `Vista_Asentamiento_Trazado_Urbano.md`): casi todo edificio se construye DENTRO del espacio plano del asentamiento (coordenadas locales, origen en el Centro Urbano) — las ÚNICAS excepciones son los extractores minerales (Cantera, Mina de oro, Mina de cobre, Mina de estaño), que se plantan sobre su nodo real en el MAPA GENERAL. Granja/Leñera/Corral son internos aunque su producción dependa de rasgos del mapa (fertilidad/bosque/livestock) dentro de la zona de influencia — el vínculo con el mapa lo lleva su `fuenteId` (Leñera/Corral) o la fertilidad de zona (Granja), no su posición.
- Crecimiento disparado por NECESIDAD REAL: más población → necesidad de comida → granja automática; excedente de recurso → mercado/almacén.
- **Escalado por demanda continua**: la construcción automática se reevalúa continuamente, no se limita a "construir una vez si no existe ninguna". Ej. Granja: se encola una adicional en cuanto haga falta, aunque ya exista al menos una.
- **Disparador de Granja: déficit real.** Se compara la **producción actual de trigo (todas las Granjas activas, con fertilidad/mano de obra/Edicto de Cosecha) con el consumo actual (población + tropas, 4.1/Doc 5.4)** — si producción < consumo, hay déficit. En déficit se permiten **hasta 3 Granjas `en_cola`/`en_construccion` a la vez**; fuera de déficit, 1 a la vez, como el resto de edificios de supervivencia.
- **Objetivo PROACTIVO además del reactivo, y prioridad a mejorar sobre construir**: el disparador de arriba mide contra la población que YA llegó, así que va un paso por detrás por diseño. Un segundo umbral, más generoso, mide contra la capacidad de Vivienda construida o en camino (el mismo criterio proactivo que la Vivienda usa con su umbral del 85%). Este umbral NO activa la ráfaga de 3 Granjas —la capacidad de Vivienda casi siempre va por delante de la población real, y tratar ese margen como emergencia sobre-construiría—: solo adelanta la siguiente Granja, de una en una. Además, si una Granja activa por debajo de su nivel máximo puede pagar YA su siguiente mejora, se prefiere mejorarla a construir otra: mejorar no pide más trabajadores (`trabajadoresRequeridos` fijo en los 4 niveles internos, solo sube el rinde), mientras que una Granja nueva diluye el `ratioManoObra`, que es un pool de mano de obra compartido por todos los edificios productores (Cantera, Leñera, minas, Corral).
- **Reemplazo de fuentes agotadas**: los extractores de recursos finitos (cantera, mina de oro, mina de cobre, mina de estaño, Corral, y la Leñera, porque los bosques también se agotan) pueden agotar su nodo fuente. Se cuentan los extractores con FUENTE VIVA, no los que existen: si un yacimiento se agota, se encola automáticamente un extractor de reemplazo (buscando un nuevo nodo del mismo recurso), hasta un MÁXIMO fijo por tipo de extractor, independiente del nivel del asentamiento (`EXTRACCION_MAXIMOS`, placeholder). Sin esto, agotar el único yacimiento condenaría al asentamiento a un déficit permanente.
- **Corral**: extractor de livestock, mismo patrón que cantera/minas — liga a un nodo finito de livestock (Doc 1.4), con reemplazo automático al agotarse.
- **Curtiduría / Armería / Fundición / Carpintería** van por AUTO-CONSTRUCCIÓN, igual que Granja/Cantera: se disparan en cuanto se cumple su requisito de nivel de asentamiento/edificio previo (4.2.1). Lo que pide una tecnología (Doc 6) espera a que la Facción la adopte.
- **Materia prima de arranque**: Curtiduría y Fundición solo se AUTO-proponen si el asentamiento ya tiene en almacén al menos uno de los insumos directos de su receta de NIVEL 1 — livestock para Curtiduría, cobre para Fundición —, llegue por extracción propia o por trueque. Sin él producirían 0: la mayoría de asentamientos no tiene cobre ni livestock en su zona. La Armería queda exenta en la práctica, porque su receta de nivel 1 incluye el Arma de Madera (solo pide madera). Este requisito solo aplica a la vía AUTOMÁTICA: la adición MANUAL de Gobernador/Maestro de Obras no lo respeta a propósito, es una decisión informada del jugador. Si la Curtiduría no lo pasa, la misma pasada sigue evaluando Armería y Fundición.
- **Líneas de producción**: la distancia dentro del asentamiento entre un edificio de transformación y la fuente más cercana de CADA insumo de su receta penaliza cuánto produce ese minuto — nunca cuánto consume por unidad, el coste de la receta no cambia. La "fuente" de un insumo crudo (cobre, estaño, livestock, madera...) es el extractor correspondiente (mina de cobre, mina de estaño, Corral, Leñera); la de un insumo INTERMEDIO (Lingote de Cobre, Cuero...) es el propio edificio que lo fabrica (ej. Armería mide su distancia a la Fundición más cercana para el insumo Lingote de Cobre, no a ninguna mina). Si un insumo no tiene ninguna fuente propia en el asentamiento (llega solo por trueque), se usa una distancia estándar en su lugar. Con varias fuentes candidatas se usa la más cercana; con varios insumos en la misma receta, manda el más penalizado (el eslabón más débil de la cadena), no un promedio. Umbrales (sin penalización por debajo de cierta distancia, suelo de producción a partir de otra, distancia sin fuente) placeholder, editables en el panel de balance (`LINEAS_PRODUCCION`).
- **El comercio no se construye solo**: las caravanas se crean y se lanzan por decisión del jugador (Doc 3.12-3.13), y las órdenes de mercado se cumplen en persona (Doc 3.3). Lo único automático es el reparto de las caravanas disponibles entre los envíos de un trueque (Doc 3.2).
- Cadenas de producción (materia prima → producto) son lógica INTERNA invisible — el jugador solo ve materiales almacenados y necesidades activas (déficits).
- Cargo MAESTRO DE OBRAS gestiona las prioridades de auto-construcción, da bonus a tiempos de construcción.
- Patrón de crecimiento: las ciudades agregan edificios desde el CENTRO hacia afuera (concéntrico).
- LAYOUT DINÁMICO según política activa: ej. política DEFENSIVA prioriza edificios defensivos y layout fácil de defender (con coste real: pierde eficiencia económica, genera "pasillos" hacia puntos de captura). Elimina la repetición visual entre asentamientos.
- **Pago al encolar y tope de cola**: el coste se paga al ENCOLAR, no al empezar a construir. Dos cupos con significado físico separado: `NECESIDADES.maximoEnCola = 4` (cuántos proyectos pueden estar pagados y a la espera de un hueco de obra, `en_cola`) y `NECESIDADES.maximoEnConstruccionSimultanea = 2` (cuántos pueden estar construyéndose a la vez: las cuadrillas de obra son limitadas). No hay cupos reservados por tipo de edificio. **Una reconstrucción tras un saqueo no ocupa cuadrilla ni espera turno** (decidido el 2026-09-26): el edificio dañado arranca su obra en cuanto puede pagarse, aunque las dos cuadrillas estén ocupadas, y no le quita el hueco a ninguna obra nueva.
- **Tiempos de obra** (decidido el 2026-09-26): las obras duran horas, no minutos, para encajar con las Eras (Doc 6) y con qué nivel de asentamiento le toca a cada una. Supervivencia: Leñera 1 h, Granja 2 h. Base de nivel 1: Vivienda, Cantera y Corral 4 h; Almacén, Granero y Mina de cobre 6 h; Minas de oro y de estaño y Mercado 8 h. Transformación y militar: 12 h. Gran Fundición 2 días, Palacio 5 días, Maravilla 4 semanas. Muralla: una celda cada 15 min (empalizada), 30 (muro de piedra) o 60 (adarve). Cifras placeholder (`EDIFICIO_CATALOGO`, `MURALLA.minutosPorCelda`).
- **Mejoras de nivel interno**: tardan más que la obra y más cuanto más alto el nivel — la obra base del edificio × 2 para el nivel 2, × 4 para el 3 y × 8 para el 4 (`MEJORA_EDIFICIO`). Se pagan al empezar; **el edificio sigue funcionando con su nivel actual** mientras se mejora, y **ocupa una de las cuadrillas de obra** mientras dura. **Las mejoras automáticas dejan siempre una cuadrilla libre** (decidido el 2026-09-26): solo arrancan si después queda al menos una para construir, así que nunca paran la construcción. Una mejora pedida a mano por el Gobernador o el Maestro de Obras sí puede tomar la última, y se rechaza solo si las dos están ocupadas. Mientras mejorar la Granja no pueda arrancar, la respuesta a un déficit de trigo es construir otra.
- **Prioridad por SCORE**: un proyecto que no puede pagarse no llega a encolarse, y los de supervivencia no pueden quedarse sin hueco — si Vivienda/Granja/Cantera coparan la cola, la Leñera (única fuente de madera) no entraría nunca. Cada candidato recibe la base de su **banda** (`SCORE_BANDAS`) + una urgencia 0-100: `supervivencia` (Granja/Leñera) = 10000, `extractorBase` (Cantera/minas/Corral) = 5000, `crecimiento` (Vivienda/Almacén) = 1000, `manual` (adiciones del Gobernador/Maestro de Obras) = 900, `transformacion` (Curtiduría/Armería/Fundición/Carpintería) = 500. La diferencia de 500+ entre bandas garantiza que la supervivencia siempre gane, sin importar cuán urgente sea lo demás. El mismo score decide qué proyecto `en_cola` pasa antes a `en_construccion` cuando las cuadrillas están ocupadas.
- **Reserva mínima de construcción, proyectada**: un proyecto solo se puede COMPROMETER (pagar al encolar) si no deja ningún recurso de Mantenimiento por debajo de una reserva mínima. Esa reserva es el coste de Mantenimiento ACTUAL del asentamiento (que ya escala con los edificios activos y la distancia a la capital, 4.5) multiplicado por un horizonte (`RESERVA_CONSTRUCCION.horizonteTicksMantenimiento = 8` minutos), más una reserva de trigo calculada igual a partir del consumo real de comida de población+tropas × `RESERVA_CONSTRUCCION.horizonteTicksComida = 8`. Excepción: la Granja no respeta la reserva de trigo, ni la Leñera la de madera, porque son la única vía real de recuperar esos recursos.
- **Los talleres respetan la misma reserva que la auto-construcción**: una receta no consume un recurso por debajo de la reserva mínima de construcción de arriba (mantenimiento y comida) más la reserva manual del Tesorero. Si no le llega, produce menos o no produce; el mantenimiento nunca se queda sin pagar por lo que gasta un taller.
- **Parar recetas** (control del jugador, `Asentamiento.recetasPausadas`): el Gobernador o el Maestro de Obras pueden parar o reanudar una receta concreta, por el recurso que produce, en todos los talleres de la plaza — por ejemplo, dejar de hacer armas de madera para guardar la madera.
- **Pausa de auto-construcción** (control del jugador, `Asentamiento.autoConstruccionPausada`): Gobernador/Maestro de Obras pueden pausar/reanudar la DETECCIÓN de nuevas necesidades desde el panel del asentamiento — mientras está pausada, no se evalúan ni comprometen proyectos nuevos, pero lo ya pagado (`en_cola`/`en_construccion`) sigue avanzando con normalidad. No afecta a la adición manual de edificios, que sigue disponible durante la pausa.
- **Capacidad de Leñeras por bosque**: un bosque grande admite más de una Leñera trabajándolo a la vez — la capacidad depende de su radio: 1 (radio < 45), 2 (45-64) o 3 (≥65). Las Leñeras que comparten un bosque se colocan en puntos distintos dentro de su radio (solo visual — la producción usa `bosque.densidad`, no la posición exacta).
- **Una Leñera puede ir a cualquier punto del bosque que caiga dentro de la zona de influencia**, no solo a su centro: un bosque cuyo borde ya está dentro de la zona es alcanzable aunque su centro quede fuera.

## 4.2.1 Catálogo completo de edificios

Algunos edificios, niveles internos y recetas piden además una **tecnología adoptada por la Facción** (Doc 6); se indica en cada uno como "pide `tecnologia`". Los requisitos de NIVEL DE ASENTAMIENTO también aplican (4.5); el nivel máximo es 5. Todas las cifras son placeholder.

### Alojamiento

**Palacio** — desbloquea la aparición de Nobleza. Adición MANUAL de Gobernador/Maestro de Obras a la cola (ver 4.2), no auto-construcción. Se construye desde el nivel 2 y se mejora hasta el nivel interno 3, que es requisito para subir a nivel 5 (4.5). Motivo: la tropa de escalón 4-5 se recluta con Nobleza (Doc 5.8), y la de las Eras I-II está en nivel 3. Cada nivel cuesta más o menos lo mismo que la subida al nivel de asentamiento en que se construye, con oro, que es el recurso de la nobleza.

| Palacio | Asentamiento | Cupo de nobles | Coste | Obra |
|---|---|---|---|---|
| 1 (construcción) | nivel 2 | 80 | 600 madera + 400 piedra + 100 oro | 3 días |
| 2 (mejora) | nivel 3 | 240 | 1.200 madera + 1.000 piedra + 300 oro | 5 días |
| 3 (mejora) | nivel 4 | 400 | 2.500 madera + 2.500 piedra + 800 oro | 1 semana |

**Vivienda** — auto-construcción. CUPO DIVIDIDO POR CLASE: cada Vivienda aporta cupos SEPARADOS, no un pool compartido — 15 espacios para Pesants + 10 espacios para Artesanos por unidad (escala linealmente: 2 Viviendas = 30+20, etc.). La Nobleza no usa Vivienda (cupo propio en el Palacio).
- Coste: 10 madera. Obra: 4 horas.
- **Tope por nivel de asentamiento** (`maximoViviendasPorNivel`): el máximo útil de Viviendas en un nivel se deriva de la población (pesants Y artesanos, se toma el mayor de los dos cupos) que exige alcanzar el SIGUIENTE nivel — construir más de las que ese cupo necesita no sirve para nada hasta subir de nivel. Cifras en `Consideraciones/Fase_0_6_Definicion_Expansion_Niveles_Asentamiento.md` §7.

### Extracción de recursos

| Edificio | Recurso | Coste | Trabajadores | Tiempo | Producción base |
|---|---|---|---|---|---|
| Cantera | Piedra | 20 madera | 4 | 5 min | 10 piedra |
| Leñera | Madera | 10 madera | 4 | 3 min | 5 madera |
| Granja | Trigo | 30 madera | 4 | 6 min | **60 trigo** (nivel 1) |
| Corral | Livestock (ganado vacuno) | 30 madera | 4 | 6 min | 3 livestock |
| Mina de oro | Oro | 40 madera | 6 | 6 min | 4 oro |
| Mina de cobre | Cobre | 30 madera | 8 | 4 min | 5 cobre |
| Mina de estaño | Estaño | 50 madera | 8 | 7 min | 3 estaño |
| Mina de hierro (pide `forja_hierro_temprana`) | Mineral de hierro | 30 madera | 8 | 4 min | 5 mineral de hierro |

Los extractores de nivel 1 se pagan solo con madera: son justo lo que hay que construir para subir a nivel 2 (4.5). Cantera, minas y Corral ligan a un nodo finito (Doc 1.4), con reemplazo automático al agotarse (4.2).

**Granja: cuatro niveles internos**, que suben el rinde sin pedir más trabajadores: **60 / 90 / 120 / 180** trigo por minuto. Mejoras: 60 madera + 20 piedra, 120+40, 240+80, sin requisito de nivel de asentamiento. Un asentamiento de nivel 1 a tope de población (300 habitantes) come 30 trigo por minuto, así que una sola Granja de nivel 1 lo sostiene con margen para alimentar tropa.

### Especiales

**Centro urbano** — edificio inicial, se construye automáticamente al fundar (Doc 1.3), no por ninguna otra vía.

**Almacén** — auto-construcción. +300 de capacidad para cada recurso. Coste: 50 madera. Obra: 6 horas.

**Granero** — almacén ESPECIALIZADO en grano: solo guarda trigo, y a cambio guarda mucho más que el Almacén general. **Uno por asentamiento**: no crece por número sino por **nivel interno**, cuatro escalones que llevan su capacidad de **2.000 a 6.000** de trigo (×1 / ×1,5 / ×2 / ×3 sobre el nivel 1, la misma forma que el rinde de la Granja).

- **Coste**: 50 madera. Obra: 6 horas. Mejoras que duplican sobre la base, con piedra a partir del nivel 2: 100+30, 200+60, 400+120; tardan 12 horas, 1 día y 2 días.
- **Requisitos de mejora**: los niveles 2 y 3 exigen asentamiento de nivel 2; el nivel 4 exige nivel 3. Se miden contra el nivel OPERATIVO, así que un asentamiento degradado deja de poder ampliar su granero hasta recuperarse.
- **Auto-construcción**: se encola cuando el trigo pasa el mismo umbral de ocupación que dispara la ampliación de Almacén, mirando SOLO el trigo. Cuando lo que desborda es el grano, 300 de capacidad general es mucho peor negocio que 2.000 de grano.

Para dimensionarlo: la reserva de comida de una ciudad de nivel 1 a tope ronda los 330, y el carro de suministros de un ejército son 500 (Doc 5.13). Un Granero de nivel 4 permite acumular una docena de campañas — es la pieza que convierte el excedente de trigo en capacidad militar en vez de perderlo contra el techo del almacén.

**Sala del Consejo** — pide `instituciones_civicas`. Donde se reúne el consejo de la ciudad; su aspecto es el de la cultura dominante (Doc 6), su nombre es neutro. Adición MANUAL de Gobernador/Maestro de Obras a la cola. **Una por asentamiento.**
- Requisito: asentamiento nivel 3. Es requisito del nivel 4 (4.5).
- Da **+1 ranura de política al Gobernador** (4.4).
- Coste: 800 madera + 1.200 piedra + 300 oro. Obra: 2 días. Huella 3×3.

**Murallas** — no son un edificio del catálogo sino un **recinto** que rodea la ciudad y se levanta por obra, desde la empalizada barata hasta la muralla de piedra (`Consideraciones/Murallas_Definicion.md`). Tener un recinto completo **de piedra** (muro de piedra o muralla con adarve; la empalizada no basta) es requisito del nivel 4 (4.5) y suma +14 al cupo de guarnición de cada héroe residente (Doc 5.15.3).

### Transformación de recursos

Van por AUTO-CONSTRUCCIÓN (igual que Granja/Cantera), disparadas en cuanto se cumple el requisito de nivel de asentamiento/edificio previo. Las cadenas de producción son lógica interna invisible de cara al jugador (solo ve inputs/outputs netos, ver 4.2). El primer edificio de este grupo que se construye dispara la aparición de Artesanos (ver 4.1). Curtiduría/Fundición además exigen la materia prima de arranque descrita en 4.2 (Armería exenta en la práctica). Las "producciones base" son el TECHO ideal por minuto — la producción real se multiplica además por el factor de líneas de producción (distancia a la fuente de cada insumo, ver 4.2), así que un edificio mal ubicado respecto a su cadena de suministro rinde menos de lo que dice esta tabla.

**Fundición** — fabricación de lingotes de metal.
- Construcción: Asentamiento nivel 2. Nivel interno 2: Asentamiento nivel 2. Nivel interno 3: Asentamiento nivel 4 y pide `forja_hierro_estandarizada`.
- Recetas nivel 1: 2 cobre → 1 Lingote de Cobre.
- Recetas nivel 2 (añade): 5 estaño → 1 Lingote de Estaño y 8 Lingote de Cobre + 2 Lingote de Estaño → 5 Lingote de Bronce (las dos piden `aleacion_bronce`); 2 mineral de hierro + 1 madera → 1 Lingote de Hierro (pide `forja_hierro_temprana`).
- Nivel 3: las mismas recetas, con **+50 %** de producción de lingotes. Es el requisito del Arma de Hierro de Calidad (Armería).
- Coste: construcción 80 madera + 40 piedra; mejora a nivel 2: 150 madera + 100 piedra; mejora a nivel 3: 300 madera + 200 piedra. Obra: 12 horas; mejoras: 1 día y 2 días.
- Trabajadores: nivel 1 → 4 artesanos; nivel 2 → 8 artesanos; nivel 3 → 12 artesanos.
- Producción base: nivel 1 → 5 Lingote de Cobre; nivel 2 → 5 Lingote de Cobre + 3 Lingote de Estaño + 1 Lingote de Bronce + 5 Lingote de Hierro; nivel 3 → ×1,5.

**Curtiduría** — tratamiento de cuero a partir de livestock (vacas).
- Construcción: Asentamiento nivel 2. Nivel interno 2: Asentamiento nivel 2. Nivel interno 3: Asentamiento nivel 3.
- Recetas nivel 1: 1 livestock → 2 Cuero.
- Recetas nivel 2: 1 livestock → 3 Cuero; 3 Cuero → 1 Cuero Curtido.
- Recetas nivel 3: 1 livestock → 4 Cuero; 3 Cuero → 1 Cuero Curtido; 2 Cuero Curtido + 1 Cuero → 1 Cuero de Calidad.
- Coste: construcción 80 madera + 30 piedra; mejora 1: 150 madera + 100 piedra; mejora 2: 450 madera + 200 piedra. Obra: 12 horas; mejoras: 1 día y 2 días.
- Trabajadores: nivel 1 → 4; nivel 2 → 6; nivel 3 → 8 artesanos.
- Producción base: nivel 1 → 8 Cuero; nivel 2 → 12 Cuero + 2 Cuero Curtido; nivel 3 → 6 Cuero + 3 Cuero Curtido + 1 Cuero de Calidad.

**Armería** — fabricación de armas y armaduras.
- Construcción: Asentamiento nivel 2. Nivel interno 2: Asentamiento nivel 2 + Fundición nivel 2. Nivel interno 3: Asentamiento nivel 3 + Fundición nivel 2 (el arma sale de la fundición; con el Palacio, el equipo de las Eras II-III llegaría en la IV).
- Recetas nivel 1: 1 Lingote de Cobre + 1 madera → 1 Arma de Cobre (pide `metalurgia_cobre`); 5 Cuero → 1 Armadura Básica; 2 madera → 1 Arma de Madera.
- Recetas nivel 2 (añade, sobre las de nivel 1): 1 Lingote de Bronce + 2 madera → 1 Arma de Bronce (pide `aleacion_bronce`); 1 Lingote de Cobre + 5 Cuero Curtido → 1 Armadura Intermedia; 1 Lingote de Hierro + 2 madera → 1 Arma de Hierro (pide `forja_hierro_temprana`).
- Recetas nivel 3 (añade, sobre las de nivel 2): 5 Lingote de Bronce + 5 madera → 1 Arma de Bronce de Calidad y 1 Lingote de Bronce + 5 Cuero de Calidad → 1 Armadura de Bronce (las dos piden `bronce_calidad_militar`); 2 Lingote de Hierro + 3 madera → 1 Arma de Hierro de Calidad (pide `forja_hierro_estandarizada` y una Fundición de nivel 3 en el asentamiento, así que llega en el nivel 4); 2 Lingote de Bronce + 5 Cuero de Calidad → 1 Armadura de Bronce de Calidad (pide `bronce_laminado`).
- **El hierro temprano no es mejor que el bronce**: es abundante y barato. El equipo no tiene poder propio; la diferencia la pone el poder de cada tropa (Doc 5.8).
- **Arma de Madera**: escalón de entrada sin metalurgia, disponible en LOS 3 NIVELES. Las recetas se REEMPLAZAN al mejorar el edificio, no se acumulan solas, así que esta se repite explícitamente en cada nivel: sin ella, mejorar la Armería quitaría la capacidad de armar la tropa de entrada. Cifras deliberadamente modestas (producción base 2, a 2 madera por unidad): la producción de recetas no respeta la reserva mínima de Mantenimiento, y una tasa alta la convertiría en una vía de colapso por falta de madera.
- Coste: construcción 80 madera + 30 piedra; mejora 1: 150 madera + 100 piedra; mejora 2: 450 madera + 200 piedra. Obra: 12 horas; mejoras: 1 día y 2 días.
- Trabajadores: nivel 1 → 4; nivel 2 → 8; nivel 3 → 20 artesanos.
- Producción base: nivel 1 → 3 Arma de Cobre + 3 Armadura Básica + 2 Arma de Madera; nivel 2 → + 2 Arma de Bronce + 2 Armadura Intermedia + 2 Arma de Hierro (Arma de Madera se mantiene); nivel 3 → + 1 Arma de Bronce de Calidad + 1 Armadura de Bronce + 1 Arma de Hierro de Calidad + 1 Armadura de Bronce de Calidad (acumulativo sobre el nivel anterior, Arma de Madera se mantiene).

### Militares

**Carpintería** — su existencia habilita el nivel 2 de Barracón y Galería de tiro, y su nivel 2 el nivel 3 de Barracón, Galería de tiro y Caballerizas. Auto-construcción (ver arriba). Fabrica equipo, no recluta: el ariete en nivel 1 y la torre de asedio en nivel 2, que todavía no tienen efecto en combate, y la pieza de **Carro de Guerra** en nivel 2.
- Construcción: Asentamiento nivel 2. Nivel interno 2: Asentamiento nivel 3.
- Receta nivel 2: 20 madera + 2 Cuero Curtido + 1 Lingote de Bronce → 1 Carro de Guerra (pide `carros_guerra`). Producción base 1.
- Coste: construcción 60 madera + 20 piedra; mejora: 120 madera + 60 piedra. Obra: 12 horas; mejora: 1 día. Trabajadores: nivel 1 → ninguno; nivel 2 → 4 artesanos.

**Barracón** — reclutamiento de tropas cuerpo a cuerpo (unidades y coste en equipo: ver Doc 5.8). Adición MANUAL de Gobernador/Maestro de Obras a la cola (ver 4.2), no auto-construcción.
- Construcción: Asentamiento nivel 2 (ver nota del requisito militar abajo). Nivel interno 2: Asentamiento nivel 2 + poseer Carpintería. Nivel interno 3: Asentamiento nivel 3 + Carpintería nivel 2.
- Coste: construcción 30 madera; mejora 1: 100 madera + 60 piedra; mejora 2: 300 madera + 200 piedra. Obra: 12 horas; mejoras: 1 día y 2 días.
- Cupo de guarnición de cada héroe residente (Doc 5.15.3): 7 / 14 / 22 según su nivel interno.

**Galería de tiro** — reclutamiento de tropas a distancia (unidades y coste en equipo: ver Doc 5.8). Adición MANUAL de Gobernador/Maestro de Obras a la cola (ver 4.2), no auto-construcción.
- Construcción: Asentamiento nivel 2 (ver nota del requisito militar abajo). Nivel interno 2: Asentamiento nivel 2 + poseer Carpintería. Nivel interno 3: Asentamiento nivel 3 + Carpintería nivel 2.
- Coste: construcción 50 madera; mejora 1: 140 madera + 20 piedra; mejora 2: 400 madera + 100 piedra. Obra: 12 horas; mejoras: 1 día y 2 días.
- Cupo de guarnición de cada héroe residente (Doc 5.15.3): 7 / 14 / 22 según su nivel interno.

**Caballerizas** — pide `cria_caballar`. Reclutamiento de caballería y carros de guerra (Doc 5.8). No se llaman Establos porque el ganado vacuno ya vive en el Corral. Adición MANUAL de Gobernador/Maestro de Obras a la cola, no auto-construcción.
- Mismos requisitos y coste que el Barracón: construcción en asentamiento nivel 2; nivel interno 2: nivel 2 + poseer Carpintería; nivel interno 3: nivel 3 + Carpintería nivel 2. Huella 3×2.
- Cupo de guarnición de cada héroe residente (Doc 5.15.3): 0 / 14 / 32 según su nivel interno — nada en el nivel 1, el coste de Liderazgo de la caballería más barata en el 2 y el de la caballería más cara del roster en el 3.

> **Requisito militar de nivel 2 — por qué existe.** Barracón, Galería de tiro y Caballerizas son los únicos tipos capaces de abrir el grupo militar en el trazado urbano, y el primero que se construye arrastra consigo la Plaza de Armas (ver `Consideraciones/Vista_Asentamiento_Trazado_Urbano.md` §5.7.1). Al fundar, el disco urbano mide 5 celdas y no existe ningún hueco que respete la separación mínima entre anclas: el núcleo militar nacería pegado al Centro Urbano y se quedaría ahí el resto de la partida, porque **ningún ancla se muda nunca**.
>
> **No retrasa nada al jugador**: las tropas de nivel 1 de Barracón y Galería de tiro piden Arma de Madera, Arma de Cobre o Armadura Básica, y las tres las fabrica **solo la Armería**, que también exige nivel 2.
>
> **El requisito va en la CONSTRUCCIÓN, nunca en el reclutamiento.** Reclutar no comprueba el nivel del asentamiento: uno que sube a nivel 2, construye Armería y Barracón y después se **degrada** a nivel 1 sigue pudiendo reclutar mientras tenga materiales. Los edificios y el equipo ya están físicamente ahí; perder nivel no borra lo que ya levantaste.

**Gran Fundición** — edificio de élite. Requiere nivel de Facción 3. Coste: 150 madera + 100 piedra + 50 oro. Obra: 2 días.

### Comercio

**Mercado** (Doc 3.12) — gatea colocar órdenes de mercado y crear caravanas comerciales propias. Sin recetas: no fabrica nada, sus niveles internos administran cupos, no producción. Adición MANUAL de Gobernador/Maestro de Obras a la cola (ver 4.2), no auto-construcción. Al alcanzar cada nivel interno crea solo sus puestos de mercado, gratis.
- Sin requisito de nivel de asentamiento para la construcción. Nivel interno 2: Asentamiento nivel 2. Nivel interno 3: Asentamiento nivel 3.
- **Coste: construcción 100 madera, SIN piedra.** Un asentamiento sin mineral alcanzable en su zona nunca junta más de los 20 de piedra de la reserva inicial (Doc 1.3), y sin Mercado no hay caravana propia con la que entregar su lado de ningún trueque: el comercio no puede depender de tener ya el recurso que el comercio existe para resolver. Mejora 1: 150 madera + 100 piedra; mejora 2: 450 madera + 200 piedra (para entonces el asentamiento ya tuvo tiempo de conseguir piedra, por extracción propia o por el comercio que el Mercado nivel 1 acaba de destrabar). Obra: 8 horas; mejoras: 16 y 32 horas.
- Por nivel interno (1 / 2 / 3): **cupo de flota** 2 / 4 / 6 caravanas (más el +1 de la política "Ampliación de Flota", ver 4.4) y **cupo de escolta** 1 / 2 / 3 escuadrones por caravana (Doc 3.13.4).
- El Mercado no regala ninguna caravana al completarse (Doc 3.13.2).

### Trofeo

**Maravilla** (`Roadmap_Escalado.md` Eje 4) — edificio único de coste extremo, sin recetas ni producción: es un trofeo, no un edificio productivo. Disponible vía control manual de cola (Gobernador/Maestro de Obras, ver 4.2), no auto-construcción. Completarla cierra el ciclo del servidor (Doc 2.9).
- Requisito: Asentamiento en nivel MÁXIMO (nivel 5) y el servidor en la Era V (Doc 6.2).
- Coste: 5000 madera + 5000 piedra + 500 oro + 300 cobre + 200 estaño + 200 livestock (todos los recursos EN BRUTO del catálogo, varias veces el coste del Palacio). Obra: 4 semanas.

## 4.3 Almacenamiento
Límites de almacenaje por recurso, ampliables construyendo más capacidad. El superávit que excede el límite dispara construcción automática de más almacenamiento.

Un asentamiento **nace con 400 de capacidad por recurso**. Con menos, la ciudad quedaría encerrada: la reserva de construcción que protege el mantenimiento le impediría gastar los 50 de madera del Almacén, que es justo el edificio que sube ese techo.

Dos edificios lo amplían, y no compiten: el **Almacén** sube la capacidad de TODOS los recursos por igual (+300 cada uno, varios por asentamiento según nivel), y el **Granero** sube solo la del **trigo** (+2.000 a +6.000 según su nivel interno, uno por asentamiento). Ver 4.2.1.

## 4.4 Políticas (mecanismo de influencia del jugador)
- Interfaz: decisiones DISCRETAS tipo menú (no sliders continuos).
- SLOTS Y POOLS POR CARGO:
  - Gobernador: 2 slots (escala hasta 5 según nivel de Facción), +1 con la Sala del Consejo (4.2.1), pool COMPLETA (todas las políticas).
  - Tesorero: 2 slots, pool de economía/comercio/mercados/precios/comisiones/caravanas/acuerdos.
  - Maestro de Obras: 1 slot, pool de auto-construcción/edificios.
  - General: 1 slot, pool de defensa/reclutamiento/fabricación de armas y armaduras.
  - Sacerdote: 1 slot, pool de felicidad/cultura/población.
  - TOTAL con todos los cargos ocupados: 7 políticas simultáneas máximo — incentiva mecánicamente la colaboración multijugador.
- Reglas: duración determinada, renovable indefinidamente, NO cancelable antes de tiempo (hay que esperar a que termine para renovar o cambiar).
- **"Edicto de Cosecha"** (Gobernador): multiplica ×1.5 la producción de trigo de todas las Granjas activas del asentamiento. No afecta a madera ni piedra. Ver 4.2.1 (Granja).
- **"Presión Fiscal"** (Tesorero): sube la recaudación de oro (`factorRecaudacion` ×1.6, ver 4.1) **a cambio de** frenar el crecimiento de las 3 clases de población (`factorCrecimientoPoblacion` ×0.8). Sin política activa = baseline (factor 1). Es el mando de "más oro ahora ↔ menos gente mañana" — no hace falta un sistema de felicidad para que tenga un coste real. El Gobernador, con pool completa, también puede activarla.
- **"Racionamiento"** (Sacerdote): reduce el consumo de trigo de la población (×0.8). No afecta a la ración de tropas (Doc 5.4). Ver 4.1.
- **"Vía Rápida de Construcción"** (Maestro de Obras): multiplica ×0.75 el tiempo de cualquier obra o mejora que arranque mientras esté activa (25% más rápido) — no afecta al coste en recursos, solo al tiempo.
- **Políticas de flota de caravanas** (Tesorero, ver Doc 3.12): "Ampliación de Flota" suma +1 al cupo de caravanas propias (aditivo, no multiplicativo — a diferencia del resto de políticas de este catálogo); "Carga Ampliada" multiplica ×1.5 la capacidad de carga de las caravanas propias; "Rutas Rápidas" multiplica ×1.5 su velocidad.
- **"Levas de guarnición"** (General): mientras esté activa, suma +14 al cupo de guarnición de cada héroe residente (Doc 5.15.3).

## 4.5 Mantenimiento de asentamientos (sistema unificado, incluye ex-"Coste de Gobernanza")
- **NIVEL DE ASENTAMIENTO — por requisitos y a petición del Gobernador**: el nivel NO sube solo. Cumplir A LA VEZ los requisitos de población y de edificios es lo que permite PEDIR la subida al nivel siguiente (ver "Subida de nivel" abajo). Nivel máximo: 5.
  - Nivel 2: 200 pesants + 3 de los 7 tipos de extracción (Cantera, Leñera, Granja, Mina de oro, Mina de cobre, Mina de estaño, Corral). No pide artesanos: sin nivel 2 no hay edificios de transformación, y sin ellos no aparecen artesanos.
  - Nivel 3: 500 pesants + 400 artesanos + Armería, Curtiduría, Fundición, Barracón y Galería de tiro.
  - Nivel 4: 1.000 pesants + 800 artesanos + un recinto de muralla completo **de piedra** (muro de piedra o muralla con adarve, no empalizada) + Sala del Consejo (4.2.1).
  - Nivel 5: 2.000 pesants + 1.600 artesanos + Palacio de nivel 3.

  Poblaciones placeholder (`NIVEL_ASENTAMIENTO`). El nivel alimenta: el techo de zona de influencia (Doc 1.2), qué materiales cobra el Mantenimiento (abajo) y el **techo de población total** — 300 / 1.500 / 6.000 / 12.000 / 20.000 habitantes para los niveles 1-5: por encima, la Vivienda y el Palacio dejan de dar cupo aunque tengan espacio. Es el nivel de ASENTAMIENTO — distinto del nivel de FACCIÓN (Doc 2.2.1), que sube por experiencia.
- **Subida de nivel** (decidido el 2026-09-26): la pide el **Gobernador** del asentamiento, se sube de uno en uno y se hace con una **obra de ascenso**. Para pedirla hacen falta cuatro cosas a la vez:
  1. **Los requisitos** de población y edificios del nivel siguiente (arriba).
  2. **Cupo libre en la Facción** para ese nivel (Doc 2.2.1, `CUPO_NIVEL_ASENTAMIENTO`). Se reserva al pedir.
  3. **El coste de la obra en el almacén del asentamiento.** Se paga entero al empezar, del almacén común — el Gobernador decide el gasto, no lo pone de su bolsillo (los héroes no tienen almacén de recursos; sus monedas no son el oro recurso, Doc 5.16.1).
  4. **Solvencia en el nivel objetivo**: lo que el asentamiento produce y recauda hoy por minuto tiene que cubrir, recurso a recurso, el mantenimiento que pagaría en el nivel nuevo (abajo). Un almacén lleno no basta: un stock no arregla un ingreso que no llega.

  | Subida | Coste de la obra | Duración |
  |---|---|---|
  | 1 → 2 | 600 madera, 400 piedra, 150 oro | 3 días |
  | 2 → 3 | 1.200 madera, 1.000 piedra, 400 oro, 100 lingotes de bronce | 1 semana |
  | 3 → 4 | 2.500 madera, 2.500 piedra, 1.000 oro, 150 lingotes de bronce | 2 semanas |
  | 4 → 5 | 5.000 madera, 5.000 piedra, 2.500 oro (más bienes de lujo si se aprueban los talleres de lujo) | 3 semanas |

  El coste pide lo que el nivel nuevo va a cobrar (piedra y oro) y, desde el nivel 3, algo que solo existe a partir de cierta Era: así el ritmo de niveles sigue a las Eras sin ninguna regla que lo prohíba (Doc 6). El lingote de bronce del 2 → 3 pide `aleacion_bronce` (Era I) y estaño, que casi siempre llega por comercio; la Sala del Consejo del 3 → 4 pide `instituciones_civicas` (Era III); el 4 → 5 lo desbloquea una tecnología de la Era V (Doc 6.2). Cada coste cabe en el almacén máximo del nivel de partida. La duración de la obra es parte del ritmo de la partida (Doc 6, Eras) y no depende de la economía. Cifras placeholder (`ASCENSO_ASENTAMIENTO`).
  - Mientras dura la obra el asentamiento funciona con normalidad. **Si lo conquistan a mitad de obra, la obra se pierde** sin devolución, y con ella la reserva de cupo.
  - La interfaz enseña, dentro de la plaza, si se puede pedir la subida y, si no, todos los motivos a la vez: requisitos que faltan, cupo, coste y la solvencia recurso a recurso.
- MEDIDOR 0-100 por asentamiento, empieza en 100.
- **Período de gracia al fundar** (Doc 1.3): durante **el primer día** tras la fundación NO se cobra mantenimiento. Sin esto, todo asentamiento nuevo caería en ruinas antes de tener su economía en marcha (las primeras obras tardan horas, 4.2.1).
- COSTE PERIÓDICO en recursos + oro. **El mantenimiento es de los edificios**: la cantidad escala con los **edificios activos** —cada Vivienda cuenta la mitad; no con la población, que ya paga lo suyo comiendo (4.1)— y con la DISTANCIA al centro de poder de la Facción (más lejos = más caro; mecanismo anti-snowball). Qué materiales se cobran depende del **nivel efectivo** (`nivelActual`): una plaza que cae de nivel por mantenimiento deja de pagar lo del nivel perdido y puede recuperarse, en vez de acabar en ruinas:
  - Nivel 1: madera.
  - Nivel 2: madera + piedra.
  - Nivel 3 en adelante: madera + piedra + **oro**. El oro se cobra desde el nivel en que aparece quien lo paga: los artesanos, que tributan casi cuatro veces más que un pesant (decidido 2026-09-26; en nivel 2 la recaudación no llegaba a un tercio del coste y era la primera causa de ruina).
- **El trigo no forma parte del coste periódico.** El "apartado de trigo" del panel de Mantenimiento es el consumo real de comida de la población (4.1) + la ración de tropas (Doc 5.4), y se descuenta UNA sola vez, donde se consume. Un déficit de trigo NUNCA degrada este medidor —que depende solo de madera, piedra y oro—, pero no es inofensivo: tiene su propio medidor de nutrición (4.1), que primero frena el crecimiento y, sostenido, cuesta población real.
- Si NO se cumple algún pago (madera/piedra/oro), el medidor BAJA de 100 a 0 de forma PROPORCIONAL al déficit (degradación gradual, no corte binario); con el pago íntegro, se regenera.
- Al llegar a 0 el asentamiento **pierde un nivel efectivo** y el medidor vuelve a empezar; con el pago íntegro sostenido (`MANTENIMIENTO.minutosSanosParaRecuperarNivel`) lo recupera. Solo si ya está en nivel efectivo 1, al llegar a 0 se DESTRUYE y cae en RUINAS → se limpia la zona → queda disponible para otro jugador/grupo. Esta es la MISMA ruta mecánica que el caso de abandono total (sea el asentamiento literalmente abandonado o simplemente mal gestionado mientras sigue activo).
- Cantidades por nivel, velocidad de degradación/regeneración y duración del período de gracia: placeholder (`MANTENIMIENTO`).

## 4.6 Entrada tardía y mundo lleno
El mapa es deliberadamente difícil de saturar por completo. Cuando un servidor se llena lo suficiente, se abre uno nuevo. El deterioro por mal mantenimiento/abandono libera continuamente zonas para nuevos jugadores.
