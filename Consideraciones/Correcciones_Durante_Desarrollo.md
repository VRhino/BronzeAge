# Correcciones Realizadas Durante el Desarrollo

Registro de los bugs reales encontrados —la mayoría probando en el navegador, no solo leyendo el código— durante la implementación de los Sprints 1-6, junto con la causa y la solución aplicada. No incluye errores de tipado rutinarios de TypeScript (por ejemplo, rellenar una entrada que falta en un `Record<EdificioTipo, string>` al añadir un edificio nuevo), solo problemas de diseño/lógica que afectaban al comportamiento de la simulación, más un par de errores de código detectados por revisión antes de que llegaran a probarse.

---

## Sprint 2 — Población y construcción automática

### 1. Bloqueo de arranque: deadlock de madera
- **Error:** ningún edificio podía construirse nunca. La lenera (único edificio que produce madera) también **cuesta** madera para construirse, y el asentamiento fundado no tenía madera inicial. El asentamiento quedaba congelado en el tick 0 para siempre.
- **Cómo se detectó:** probando en el navegador — tras 15 ticks, ningún edificio en cola pasaba a "en construcción".
- **Solución:** la caravana de fundación (Doc 1.3) ahora entrega una reserva inicial de materiales (`FUNDACION.materialesIniciales` = madera y piedra) suficiente para arrancar la primera construcción.

### 2. Hambruna silenciosa: la Granja nunca escalaba con la demanda
- **Error:** solo se construía una Granja en toda la vida del asentamiento. Ya existía la constante `NECESIDADES.umbralComidaTicksReserva` pensada para disparar granjas adicionales, pero el código nunca la usaba — el trigo caía a 0 según crecía la población, sin que se generara ninguna respuesta automática.
- **Cómo se detectó:** tras 200 ticks de prueba, el trigo se desplomó a 0/800 mientras la población seguía intentando crecer.
- **Solución:** se añadió la comprobación que faltaba: si la reserva de trigo proyectada (trigo disponible ÷ consumo actual) cae por debajo del umbral, se encola una granja adicional (no solo cuando no existe ninguna).

---

## Sprint 4 — Estructura política

### 3. Secciones HTML de Trueque/Mercado borradas sin querer al reescribir `main.ts`
- **Error:** al añadir los nuevos paneles de cargos, ciudadanía, políticas, diplomacia y fusión/anexión se reescribió `main.ts` completo, pero el HTML de las secciones "Trueque" y "Orden de Mercado" (del Sprint 3) no se volvió a incluir — aunque sí quedaron las referencias JavaScript a sus elementos.
- **Cómo se detectó:** la consola del navegador mostró `TypeError: Cannot read properties of null (reading 'addEventListener')` y `ReferenceError: truequeASelect is not defined` al cargar la página.
- **Solución:** se reinsertaron ambos bloques de HTML y las declaraciones de elementos (`const truequeASelect = ...`, etc.) que faltaban.

### 4. Mutación de estado compartido en el motor de mercado
- **Error:** `avanzarMercado` mutaba directamente los objetos `Asentamiento` y `OrdenMercado` recibidos como parámetro (vía un `Map` construido sin clonar), rompiendo el estilo inmutable usado en el resto del motor y arriesgando efectos secundarios invisibles sobre el estado del llamador.
- **Cómo se detectó:** revisión propia del código antes de continuar, no un fallo en pruebas.
- **Solución:** se reescribió la función para trabajar sobre copias explícitas (`{...objeto}` al insertarlas en el `Map`) y nunca tocar los arrays/objetos que llegan como argumento.

### 5. Variables con caracteres cirílicos por error de tipeo
- **Error:** en `diplomacia.ts` y `combate.ts` aparecieron nombres de variable con una "с" cirílica en vez de la "c" latina (`facционesActualizadas`, `facционesFinal`), indistinguibles a simple vista pero que habrían roto la compilación o, peor, creado una variable "fantasma" si por casualidad hubiera compilado.
- **Cómo se detectó:** revisión manual del código recién escrito, confirmado con un barrido `grep -P '[а-яА-ЯёЁ]'` sobre todo `src/` antes de cerrar el Sprint 6.
- **Solución:** renombradas a `faccionesActualizadas` / `faccionesFinal`.

---

## Sprint 5 — Guerra simplificada

### 6. No existía ninguna forma de extraer cobre
- **Error:** el reclutamiento militar exige cobre para todo tipo de tropa, pero desde el Sprint 2 solo se habían implementado extractores para piedra (cantera) y oro (mina) — nunca se construyó el equivalente para cobre. Ningún asentamiento podía acumular cobre jamás, así que el reclutamiento era imposible de probar (y de usar).
- **Cómo se detectó:** al intentar reclutar la primera tropa de la partida, el cobre estaba —y estaría siempre— en 0.
- **Solución:** se añadió el edificio `minaCobre`, con el mismo patrón de autoconstrucción, colocación junto al nodo más cercano y producción/agotamiento que cantera y mina.

---

## Sprint 6 — Mantenimiento, reputación y progresión

### 7. Muerte instantánea de todo asentamiento nuevo
- **Error:** el coste de Mantenimiento exige trigo desde el primer tick, pero un asentamiento recién fundado no tiene trigo ni Granja construida todavía (la Granja tarda varios ticks en completarse). El medidor de mantenimiento caía de 100 a 0 en unos 9 ticks **siempre**, sin importar qué tan bien se gestionara el asentamiento, destruyéndolo antes de que la economía tuviera oportunidad de arrancar.
- **Cómo se detectó:** al probar el mantenimiento en el navegador, un asentamiento recién fundado caía en ruinas en el tick 9 de forma sistemática y reproducible.
- **Solución:** se añadió una protección temporal (`MANTENIMIENTO.graciaTicks`) durante la cual no se cobra mantenimiento — que de paso resuelve un punto que estaba marcado explícitamente como pendiente en el diseño (Doc 1.3: "protección temporal para asentamientos recién fundados").

### 8. Bloqueo permanente tras agotar el único yacimiento de un recurso
- **Error:** una vez que el nodo de piedra/oro/cobre explotado por la única cantera/mina existente se agotaba, la lógica de auto-construcción nunca volvía a evaluar esa necesidad — la condición solo comprobaba "¿existe al menos una cantera?", sin importar si su fuente seguía viva. Combinado con el punto 7 (mantenimiento exige piedra desde nivel 3), esto condenaba al asentamiento a un déficit permanente e irreversible en cuanto el yacimiento se secaba.
- **Cómo se detectó:** el registro mostraba "El yacimiento de piedra... se ha agotado" y, a partir de ahí, el asentamiento entraba en déficit de mantenimiento continuo hasta caer en ruinas, sin que nunca se intentara construir un reemplazo.
- **Solución:** se reescribió la condición de disparo para contar cuántos extractores con fuente **viva** existen (no solo cuántos existen en total), permitiendo encolar nuevas instancias —hasta un máximo ligado al nivel del asentamiento— cuando las existentes se agotan. Se aplicó el mismo criterio a cantera, mina, minaCobre y lenera.

### 9. Recalibración de las cifras de Mantenimiento
- **Error:** incluso con las correcciones 7 y 8 aplicadas, el coste base de trigo/madera y la velocidad de degradación seguían siendo demasiado altos en relación a lo que un asentamiento con una sola Granja/Lenera podía producir de forma sostenida, provocando una espiral de déficit incluso en asentamientos bien gestionados y con buen acceso a recursos.
- **Cómo se detectó:** pruebas de larga duración (150-300 ticks) en las que el asentamiento seguía muriendo pese a las dos correcciones anteriores.
- **Solución:** se redujo el coste base de trigo y madera, se bajó la velocidad de degradación por déficit, se subió la regeneración cuando el pago es íntegro, y se retrasó de nivel 6 a nivel 8 el umbral en que el mantenimiento empieza a exigir oro (recurso raro por diseño, Doc 1.1) — evitando que un asentamiento aislado sin mina de oro propia quede condenado de forma temprana. Estas cifras siguen siendo placeholder, como el resto de valores numéricos del proyecto (ver `Preguntas_Abiertas.md`).

---

## Post-Sprint 6 — Rebalance de progresión, cola de construcción y estabilidad económica

Sesión de rebalance posterior al cierre de Sprint 6, motivada por partidas de prueba: los asentamientos subían de nivel casi instantáneamente y muchos morían por falta de madera pese a poder construir más de una Leñera.

### 10. Asentamientos subían de nivel casi instantáneamente
- **Error:** con `poblacionPorPunto=40` y `puntosPorNivel=3`, los 5 edificios activos con los que nace todo asentamiento (Centro Urbano + Granja + 3 Viviendas, Doc 1.3) ya sumaban 5 puntos de nivel — el asentamiento arrancaba en nivel 2 antes del primer tick, muy por delante de lo que tarda una construcción (4-10 ticks) o una caravana en cruzar el mapa (velocidad 5-12 unidades/tick sobre un mapa de 1000x1000). La población, con una tasa de crecimiento de apenas 0.05, nunca llegaba a "sostener" ese nivel.
- **Cómo se detectó:** reportado por el usuario jugando; confirmado simulando 60 ticks (el asentamiento ya nacía en nivel 2, sin haber avanzado ningún tick).
- **Solución:** `NIVEL_ASENTAMIENTO.puntosPorNivel` subido de 3 a 8, `poblacionPorPunto` bajado de 40 a 20 (la población pesa más en el cómputo), y la tasa de crecimiento base de Pesants subida de 0.05 a 0.12 (Artesanos de 0.03 a 0.05). Verificado en 60 ticks simulados: el asentamiento ahora nace en nivel 1 y tarda ~22 ticks en subir a nivel 2, con la población creciendo de forma visible cada 1-2 ticks en vez de estancarse.

### 11. Interbloqueo de prioridad: la Leñera nunca llegaba a construirse, asentamientos morían por falta de madera
- **Error:** `evaluarNecesidades` comprobaba las necesidades en un orden fijo (Vivienda → Granja → Cantera → Leñera → Almacén → Taller → minas) y la cola global tenía un tope de 3 edificios `en_cola` (ver punto sobre el tope de cola más abajo). Un proyecto que no podía pagarse se quedaba parado en cola **para siempre** — nada lo saca de ahí si nunca junta los recursos. Como Vivienda/Granja/Cantera se re-disparaban casi todos los ticks y se evaluaban primero, podían copar los 3 slots con proyectos atascados por falta de madera, dejando a la Leñera —la única fuente de madera del asentamiento— sin hueco para encolarse nunca. Sin Leñera nueva entrando en juego, el déficit de madera no se corregía y Mantenimiento (que cobra madera desde el primer tick tras el período de gracia) acababa destruyendo el asentamiento.
- **Cómo se detectó:** reportado por el usuario ("muchos de los asentamientos se destruyen por la mala gestión de la construcción automática... siempre la dejan de última"); confirmado en pruebas: asentamientos cayendo en ruinas con la Leñera nunca construida pese a haber bosques disponibles en su zona.
- **Solución:** dos cambios en `evaluarNecesidades` (`engine/construction.ts`): (a) se reordenó la evaluación para comprobar primero Granja/Leñera (recursos de supervivencia: comida y madera), luego los extractores secundarios (Cantera, Cobre, Oro, Estaño), y al final Vivienda/Almacén/Taller (crecimiento); (b) de los 3 slots de la cola, 1 queda reservado EXCLUSIVAMENTE para Granja/Leñera (`NECESIDADES.slotsReservadosSupervivencia`), así Vivienda/Almacén/Taller nunca pueden copar los 3 y dejarlas sin hueco.

### 12. Nueva política "Protección de Riesgos" (Maestro de Obras) — con su propio interbloqueo detectado al probarla
- **Contexto:** a petición del usuario, se añadió al catálogo una política de emergencia (Doc 4.4): mientras esté activa, la auto-construcción ignora cualquier otra necesidad hasta tener 3 Leñeras (activas o en curso/cola).
- **Error detectado al probarla:** si no había NINGÚN bosque libre en la zona de influencia todavía, la política bloqueaba TODO indefinidamente — 0 progreso posible durante los 150 ticks que dura la política, peor que no activarla.
- **Cómo se detectó:** probando en el navegador con un asentamiento fundado sin bosque cercano: 50 ticks seguidos sin construir absolutamente nada.
- **Solución:** la política solo bloquea el resto de necesidades si hay progreso real hacia el objetivo (una Leñera ya en cola/construcción, o se puede encolar una nueva ese mismo tick); si no hay ningún sitio de bosque disponible todavía, deja pasar la evaluación normal y retoma la prioridad en cuanto la zona de influencia crezca lo suficiente para alcanzar un bosque.

### 13. Reserva mínima de recursos para construcción (nueva mecánica, a petición del usuario)
- **Contexto:** aunque el punto 11 reduce el riesgo, seguía siendo posible que Vivienda/Almacén/Taller gastaran hasta el último punto de madera o trigo disponible, dejando a Mantenimiento sin nada que cobrar justo ese tick.
- **Solución:** un edificio en cola solo puede empezar a construirse si, además de poder pagar el costo completo, no deja ningún recurso protegido por Mantenimiento en ese momento (madera+trigo siempre; +piedra desde nivel 3; +oro desde nivel 8 — mismo criterio que usa el propio Mantenimiento) por debajo de una reserva mínima (`RESERVA_CONSTRUCCION`: madera 30, trigo 20, piedra 20, oro 10). Excepción deliberada: Granja no respeta la reserva de trigo, ni Leñera la de madera — son la única vía real de recuperar esos recursos, así que bloquearlas por la misma escasez que deben resolver habría sido otro interbloqueo sin salida (rompería en seco a la política del punto 12).
- **Cómo se verificó:** un asentamiento sin bosque alcanzable mantuvo la madera clavada exactamente en 30 durante más de 40 ticks (Vivienda/Taller parados en cola sin tocarla) con el medidor de Mantenimiento en 100/100 todo el tiempo — el mismo caso que antes terminaba en ruinas.

### 14. Tope de la cola de construcción
- **Contexto:** antes de los puntos 11-13, la cola de edificios `en_cola` de un asentamiento no tenía límite superior.
- **Solución:** se fijó un máximo de 3 edificios `en_cola` simultáneos por asentamiento (`NECESIDADES.maximoEnCola`), compartido entre auto-construcción y construcción manual (Fundición/Gran Fundición) — construir manualmente por encima del tope se rechaza con un mensaje explícito en vez de fallar en silencio.

### 15. No existía ninguna forma de extraer estaño
- **Error:** el estaño se genera en el mundo con el mismo criterio de rareza que cobre/oro (Doc 1.1), es comerciable, y es coste de las tropas de Nobleza (Doc 5) — pero nunca se implementó un edificio de extracción para él (solo `mina` para oro y `minaCobre` para cobre, ver punto 6). Cualquier asentamiento con un nodo de estaño en su propia zona de influencia no podía aprovecharlo jamás; la única forma de conseguir estaño era comerciándolo con otra Facción.
- **Cómo se detectó:** reportado por el usuario revisando la simulación.
- **Solución:** se añadió el edificio `minaEstano`, con el mismo patrón de autoconstrucción, colocación junto al nodo más cercano y producción/agotamiento que `cantera`/`mina`/`minaCobre`.

---

## Post-Sprint 6 — Implementación del rediseño de progreso/arranque de asentamientos

Sesión de implementación del rediseño documentado en Docs/1, 4, 5, 6 (nuevo modelo de nivel por gates, catálogo de edificios de transformación con crafting multi-nivel, Corral, disparador de Artesanos, políticas de desbloqueo de edificios especiales). Bugs detectados jugando la simulación durante la implementación, no solo leyendo el código.

### 16. Interbloqueo: Curtiduría/Armería copaban la cola general y dejaban a Cantera sin hueco
- **Error:** con el rediseño, Curtiduría/Armería/Fundición se evaluaban en el mismo cupo general de la cola que Cantera/minas. Curtiduría y Armería (siembran su sitio con `sitioConcentrico`, no dependen de un nodo de recurso) conseguían encolarse en el tick 1, antes de que la zona de influencia creciera lo suficiente para alcanzar un nodo de piedra — ocupando los 2 slots generales disponibles. Como ambas necesitan piedra para completarse y Cantera (su única fuente) nunca conseguía un hueco para encolarse, el asentamiento quedaba parado para siempre sin piedra.
- **Cómo se detectó:** simulando 150-300 ticks, un asentamiento se quedó con piedra clavada en 20 y Curtiduría/Armería en cola indefinidamente.
- **Solución:** mismo patrón que el interbloqueo de Granja/Leñera (punto 11): nuevo slot reservado EXCLUSIVAMENTE para extractores base (cantera/minaCobre/mina/minaEstano/corral) — `NECESIDADES.slotsReservadosExtractores`, con `maximoEnCola` subido de 3 a 4 para no restar concurrencia al resto.

### 17. Segundo interbloqueo: Curtiduría+Armería+Fundición también podían dejar a Vivienda sin hueco
- **Error:** el fix del punto 16 no bastaba — en 2-3 ticks, las 3 transformación podían terminar ocupando igualmente los slots generales (uno por tick), y si quedaban atascadas esperando piedra en un punto de fundación pobre en ese recurso, permanecían en cola para siempre, dejando a Vivienda (que no necesita piedra) sin hueco y a la población estancada permanentemente en el tope de vivienda inicial.
- **Cómo se detectó:** simulando 300-600 ticks en un punto sin piedra alcanzable: población clavada en 45 (capacidad de 3 Viviendas) durante cientos de ticks pese a cumplirse el umbral de ocupación.
- **Solución:** Vivienda/Almacén se evalúan ANTES que Curtiduría/Armería/Fundición, y como máximo UNA de las tres puede estar `en_cola` A LA VEZ (no solo "una nueva por tick") — se comprueba `hayProyectoPendiente` sobre las tres como grupo antes de intentar encolar cualquiera.

### 18. Recalibración: tope de extractores demasiado bajo para la tasa de crecimiento de población existente
- **Error:** `EXTRACCION_MAXIMOS.porTipo` se fijó en 5 como placeholder inicial. Con la tasa de crecimiento de Pesants ya existente (12%/tick, sin tope salvo Vivienda) y Vivienda ya sin bloqueo (punto 17), la población crecía sin freno mientras la extracción de recursos quedaba capada en 5 instancias por tipo — todo asentamiento probado colapsaba por déficit de Mantenimiento entre el tick 74 y el 674 (variable por el redondeo estocástico del crecimiento poblacional), incluso en ubicaciones con buen acceso a recursos.
- **Cómo se detectó:** simulando 300-600 ticks en múltiples ubicaciones (con y sin piedra cercana): todas cayeron en ruinas.
- **Solución:** subido a 10 (mismo techo que permitía el `nivelMaximo` anterior de 10, antes de que el rediseño lo bajara a 3) — sigue siendo PLACEHOLDER, pendiente de más calibración.

### 19. Panel de Almacén no mostraba los recursos intermedios de crafting
- **Error:** `main.ts` listaba el Almacén iterando `CATALOGOS.recursosTrueque` (7 recursos comerciables) en vez de los recursos realmente presentes en `asentamiento.almacen` — los 12 tipos nuevos (lingotes, cuero, armas, armaduras) nunca aparecían en el panel aunque se estuvieran produciendo, haciendo invisible el resultado del nuevo sistema de crafting.
- **Cómo se detectó:** tras confirmar que Curtiduría/Armería/Fundición se construían y activaban Artesanos, el panel de Almacén seguía mostrando solo los 7 recursos de siempre.
- **Solución:** el panel de Almacén ahora itera `Object.keys(asentamiento.almacen)` (todo lo que exista, sea comerciable o no); `CATALOGOS.recursosTrueque`/`recursosMercado` se dejan intactos para los formularios de trueque/mercado, que sí deben quedarse limitados a los recursos base.

### 20. Edificios de transformación: gate de construcción BASE vs. gate de mejora de nivel interno
- **Aclaración de diseño durante la implementación (no bug, releyendo la spec original con más cuidado):** los "Requisitos nivel 2/3" de Curtiduría/Armería/Fundición en el diseño original son gates para MEJORAR el edificio a su nivel interno 2/3, no para construirlo por primera vez — solo Carpintería y Palacio gatean su construcción base. Es coherente: el propio gate de nivel 2 de asentamiento exige TENER construidas Armería/Curtiduría/Fundición, así que tienen que poder construirse antes de alcanzar ese nivel.

Verificado en el navegador tras estas correcciones: Corral/Curtiduría/Armería/Fundición se auto-construyen sin bloquear Granja/Leñera/Vivienda; Artesanos aparecen al completarse el primer edificio de transformación; el gate de nivel de asentamiento refleja correctamente población y edificios pendientes; Barracón se encola y completa vía política en su cluster de cola aparte sin desplazar la cola general; sin errores de consola ni de compilación (`tsc --noEmit` limpio) en ningún punto.

---

## Post-Sprint 6 — Cola de prioridades al gastar recursos y consolidación del consumo de trigo

Sesión de debugging a partir de una partida real exportada por el usuario (JSON de estado en tick 0), motivada por dos reportes: un "deadlock nuevo" en la auto-construcción y un consumo de trigo percibido como demasiado rápido.

### 21. Orden de GASTO de recursos por inserción, no por prioridad — Curtiduría/Armería podían dejar a Granja/Leñera sin madera pese a sobrar
- **Error:** los puntos 11 y 16 ya priorizan qué se ENCOLA (supervivencia > extractores > general), pero el `for` de `avanzarConstruccion` que decide quién ARRANCA construcción (gasta el almacén compartido) recorría `asentamiento.edificios` en orden de INSERCIÓN, sin relación con esa categoría. Curtiduría/Armería/Fundición no tienen gate de nivel y se encolan casi desde el tick 1; cuestan 80 madera cada una frente a 30/10 de Granja/Leñera. Si quedaban antes en el array que una Granja/Leñera nueva encolada más tarde (por crecimiento de población), se llevaban la madera disponible primero — dejando el asentamiento sin margen para sostenerse a sí mismo aunque hubiera madera de sobra para pagar ambas.
- **Cómo se detectó:** reportado por el usuario jugando ("hay deadlock nuevo porq intenta construir edificios de transformacion, antes de asegurar los recursos base en surplus para mantenerse a si mismo"); confirmado leyendo el orden del `for` en `engine/construction.ts` y con una prueba matemática del caso exacto (madera=125, reserva=30: Curtidería arrancando primero dejaba a Granja sin margen para pasar su propio chequeo de reserva, aunque había madera de sobra para las dos).
- **Solución:** `avanzarConstruccion` ahora resuelve el arranque de construcciones `en_cola` en dos pasos: primero producción/progreso (orden original), luego el gasto de recursos en orden de PRIORIDAD por categoría (supervivencia > extractores > general) usando un `sort` estable, no el orden de inserción. Verificado en el navegador (900 ticks, mundo con bosques localizados vía export del world): asentamientos que antes colapsaban sistemáticamente entre el tick 74-90 sobrevivieron sanos (medidor 100/100) con Curtidería atascada `en_cola` 600+ ticks esperando piedra sin volver a bloquear nunca a Granja/Leñera.

### 22. Mecánica repetida: Mantenimiento cobraba un trigo fijo ADEMÁS del consumo real de comida
- **Error:** `calcularCostoMantenimiento` incluía un coste fijo de trigo (`MANTENIMIENTO.costoBase.trigo`) que se descontaba en `avanzarMantenimiento`, TOTALMENTE APARTE del consumo real de comida de la población (`consumirComida`, cada tick) y de las raciones de tropas (`avanzarMantenimientoTropas`, cada tick) — dos mecanismos distintos drenando el mismo recurso por la misma razón de fondo ("alimentar al asentamiento"), sin relación entre sí. El "apartado de trigo" mostrado en el panel de Mantenimiento (`gameStore.mantenimientoInfo`) reflejaba ese valor fijo desconectado, no el consumo real.
- **Cómo se detectó:** reportado por el usuario tras notar que el trigo se consumía "muy rápido"; confirmado simulando con el JSON de partida real que aportó (3 asentamientos, tick 0→80): el trigo caía a 0 sistemáticamente por la combinación de este doble descuento con una Granja de producción fija incapaz de seguirle el ritmo a una población creciendo 12%/tick compuesto.
- **Solución:** se retiró `trigo` del coste que calcula/descuenta `calcularCostoMantenimiento` (queda solo madera, +piedra/oro por nivel) — el trigo se sigue descontando exactamente donde ya se descontaba (`consumirComida`/`avanzarMantenimientoTropas`), una sola vez. El panel de Mantenimiento ahora muestra como "apartado de trigo" la suma real `consumoComidaPoblacion(asentamiento) + consumoRacionTropas(asentamiento)` (nuevos helpers extraídos en `population.ts`/`tropas.ts`), no un placeholder. Efecto colateral documentado: un déficit de trigo ya no degrada el medidor de Mantenimiento directamente (solo frena el crecimiento de población vía `comidaFactor`) — el medidor pasa a depender solo de madera/piedra/oro. Como calibración adicional (mismo pedido del usuario), se duplicó `EDIFICIO_CATALOGO.granja.produccionBaseTrigo` (5→10): mejora real pero no elimina del todo el desajuste de fondo entre población exponencial sin techo de producción y una Granja de salida fija — verificado reimportando el mismo JSON de partida: el asentamiento con peor fertilidad pasó de colapsar en el tick 45 a colapsar hacia el tick 65-70 y recuperarse más rápido, en vez de quedarse en 0 durante 20+ ticks.

Verificado en ambos casos: `tsc --noEmit` limpio, sin errores de consola, comportamiento confirmado reimportando la misma partida de prueba antes/después de cada cambio.

---

## Post-Sprint 6 — Calibración adicional de trigo, políticas nuevas y fix de alcance de bosques

Sesión de seguimiento tras la corrección #22: el usuario pidió calibrar mejor el disparador de Granja, añadir una política de refuerzo de trigo, ampliar "Protección de Riesgos" y permitir más de una Leñera por bosque — y, jugando con esos cambios ya activos, reportó un colapso por falta de madera que llevó a un bug real de más peso que los cambios pedidos.

### 23. Disparador de Granja: de "reserva estimada" a déficit real, y hasta 3 a la vez
- **Contexto (a petición del usuario):** el disparador de una Granja adicional (corrección #2/Sprint 2) comparaba `trigo disponible ÷ consumo actual` contra un umbral de 5 ticks — una estimación que no tenía en cuenta que el consumo sigue subiendo con la población mientras la Granja se construye (6 ticks), ni las raciones de tropas (`consumoRacionTropas`), así que podía disparar tarde.
- **Solución:** se reemplazó por una comparación directa: `producción actual de trigo (todas las Granjas activas) < consumo actual (población + tropas)`. Además, se permite tener hasta 3 Granjas `en_cola`/`en_construccion` a la vez mientras haya déficit (antes solo 1, obligando a corregir un déficit severo en serie).
- **Verificado:** simulando con un asentamiento sin política de refuerzo, en tick 70 se observaron 2 Granjas no activas simultáneas (`en_construccion` + `en_cola`) reaccionando a un déficit real antes de que el trigo llegara a 0 — el asentamiento nunca colapsó.

### 24. Nueva política "Edicto de Cosecha" (Gobernador) — a petición del usuario
- **Contexto:** además de recalibrar el disparador, el usuario pidió una política que suba la producción de trigo ×1.5, sin tocar madera ni piedra.
- **Solución:** nuevo campo multiplicativo `factorProduccionTrigo` en el sistema de políticas (mismo patrón que Racionamiento/Vía Rápida), aplicado solo en la fórmula de producción de Granja.
- **Verificado:** midiendo la producción de la misma Granja antes/después de activar la política en el mismo asentamiento (aislando el resto de variables): 3.0 → 4.5 trigo/tick, exactamente ×1.5.

### 25. "Protección de Riesgos" ampliada a 2 Leñeras + 3 Granjas — a petición del usuario
- **Contexto:** la política (corrección #12) solo exigía 3 Leñeras. El usuario pidió que exigiera 2 Leñeras Y 3 Granjas, bloqueando cualquier otra auto-construcción hasta cumplir ambas.
- **Solución:** se generalizó la lógica a ambos tipos, evaluados en la misma pasada (no hace falta terminar uno para empezar el otro), con la misma válvula de escape que ya existía (si NINGUNO de los dos objetivos puede avanzar ese tick, se deja pasar la evaluación normal para no congelar el asentamiento sin salida).
- **Verificado (a petición del usuario, tarea de verificación explícita):** con dos bosques cercanos entre sí, la política bloqueó todo lo demás hasta llegar a 2/3 exactos y soltó el bloqueo en el mismo tick que se cumplió; con un solo bosque alcanzable, el objetivo de Leñeras quedó permanentemente en 1 y la política soltó el bloqueo igual en cuanto Granja llegó a 3 (válvula de escape) — el usuario confirmó que ese comportamiento es el deseado.

### 26. Capacidad de Leñeras por tamaño de bosque (1-3) — a petición del usuario
- **Contexto:** hasta ahora un bosque solo admitía una Leñera, sin importar su tamaño. El usuario pidió que un bosque grande admita más de una, hasta 3.
- **Solución:** capacidad por bosque = función de su radio (`BOSQUE.capacidadLenerasPorRadio`: <45 → 1, 45-64 → 2, ≥65 → 3). Las Leñeras que comparten un bosque se colocan en puntos distintos dentro de su radio (solo para no dibujarse superpuestas — la producción sigue usando `bosque.densidad`, no la posición).
- **Verificado:** un asentamiento rodeado de 3 bosques de distinto tamaño terminó con exactamente 3 Leñeras en el bosque grande y 2 en cada uno de los medianos, estable desde el tick 10, cada uno parado en su propia capacidad.

### 27. Bug real: el alcance de un bosque solo comprobaba su centro, no su solapamiento con la zona
- **Error:** reportado por el usuario ("he visto simulaciones que un asentamiento... crece su zona de influencia y entra en contacto con un bosque, pero aun así no crea la leñera y se destruye"). La búsqueda de sitio para Leñera (`sitioEnBosque`) solo comprobaba si el CENTRO exacto del bosque caía dentro del polígono de zona — pero los bosques tienen radio (30-80) y el radio de zona tiene un TOPE por nivel (60/90/120). Un bosque grande cuyo borde ya estaba bien dentro de la zona, pero cuyo centro quedaba un poco más allá del tope, era invisible PARA SIEMPRE — el asentamiento nunca conseguía Leñera pese a que la zona ya tocaba el bosque físicamente, agotaba su madera inicial en cuanto Mantenimiento empezaba a cobrarla (fin del período de gracia) y caía en ruinas. La Leñera inicial de fundación no tenía este problema porque usa un chequeo distinto y ya correcto (`bosqueCercano`, por radios).
- **Cómo se detectó:** reproducido a propósito: se fundó un asentamiento a una distancia del bosque más cercano tal que su borde quedaba dentro del radio inicial de zona pero su centro nunca entraba dentro del tope de radio de nivel 1 (60). Resultado: `radioPotencial` se estancó en 60 desde el tick 20, la Leñera se quedó en 0 para siempre, la madera inicial se agotó a partir del tick 60 (fin de gracia) y el asentamiento colapsó hacia el tick 85-90 — reproducción exacta del reporte del usuario.
- **Solución:** `sitioEnBosque` ahora acepta cualquier punto del bosque que caiga dentro de la zona (no solo el centro) — primero prueba el punto "ideal" (centro, o el punto con offset si el bosque ya tiene otras Leñeras) y, si ese queda fuera, muestrea puntos en anillos crecientes dentro del propio bosque hasta encontrar uno que sí esté dentro de la zona.
- **Verificado:** repitiendo el mismo escenario exacto con el fix aplicado, la Leñera aparece en el tick 20 (en cuanto la zona toca el bosque) y el medidor de Mantenimiento se mantiene en 100/100 hasta el tick 100, sin colapso.

Verificado en conjunto: `tsc --noEmit` limpio en cada paso, sin errores de consola, todas las verificaciones anteriores confirmadas en el navegador con escenarios reproducibles (no solo lectura de código).

---

## Post-Sprint 6 — Sincronización con la reestructuración de Notion (Docs 1-6) y cambios de mecánica derivados

El usuario reestructuró y limpió los 6 documentos numerados directamente en Notion (fuente de verdad para esa sesión). Al sincronizar el repo contra esa versión aparecieron varias contradicciones reales entre el texto nuevo y el código existente — el usuario decidió, para cada una, si el código debía cambiar para alinearse con el nuevo diseño o si el texto de Notion tenía un error. Las correcciones #28-30 son los cambios de código resultantes de esas decisiones; la #31 es un riesgo real detectado al verificarlos que el usuario decidió aceptar tal cual.

### 28. Zona de influencia: de crecimiento temporal a ligado a construcción activa — decisión real del usuario
- **Contexto:** Notion redefinió el crecimiento de la zona de influencia (Doc 1.2) como ligado a CONSTRUCCIÓN ACTIVA ("cada edificio completado empuja el radio"), contradiciendo el código real (`avanzarCrecimientoZonas`, un incremento FIJO por tick sin relación con construcción). Consultado, el usuario confirmó que era una decisión real de diseño, no un error de Notion.
- **Solución:** se retiró `avanzarCrecimientoZonas` (crecimiento temporal) y se movió la lógica al punto donde `avanzarConstruccion` marca un edificio como `activo` — cada uno completado ese tick suma `ZONA_INFLUENCIA.crecimientoPorEdificioCompletado` (placeholder: 5) al radio, con el mismo tope por nivel de siempre (60/90/120).
- **Verificado:** un asentamiento de prueba se quedó en radio 30 mientras nada se completaba (varios ticks con una Cantera todavía `en_construccion`), y saltó a exactamente 35 en el tick en que la Cantera terminó — confirma que el disparador es la finalización, no el paso del tiempo.

### 29. Leñera inicial condicional retirada — decisión real del usuario
- **Contexto:** Notion marcó la Leñera inicial condicional (Doc 1.3, corrección #1 original) como "DEPRECADA / YA NO NECESARIA": la reserva de materiales iniciales (madera+piedra) ya es suficiente por sí sola para evitar el deadlock de madera que la originó. El código todavía la construía activamente. El usuario confirmó retirarla del código.
- **Solución:** se eliminó `bosqueCercano` y la construcción condicional de Leñera en `edificiosIniciales` (`engine/settlement.ts`) — todo asentamiento nuevo nace solo con Centro Urbano + Granja + 3 Viviendas.
- **Verificado:** un asentamiento recién fundado no tiene ninguna Leñera en su lista de edificios iniciales, incluso fundado junto a un bosque.

### 30. Reclutamiento: Artesanos se suma al carril de equipo, Nobleza deja de reclutar tropas — decisión real del usuario
- **Contexto:** Notion (Doc 4.1) afirmaba que "ambos edificios [Barracón/Galería] reclutan de los dos pools [Pesants y Artesanos]", contradiciendo el código (`reclutarTropa` fijado a `origen:'pesants'`) y al propio Doc 5.8 de Notion (tablas tituladas "carril Pesants"). Por separado, Notion (Doc 5.8) marcaba el reclutamiento de Nobleza como "fuera de alcance de Fase 0", contradiciendo el reclutamiento de Nobleza vía Gran Fundición ya implementado y funcional. Consultado sobre ambas, el usuario confirmó: (a) es una decisión real — Artesanos también debe reclutar por equipo, la etiqueta "carril Pesants" es lo que está mal en Notion; (b) es una decisión real — pero aclaró explícitamente que NO se elimina Nobleza como clase de población (crecimiento, requisito de Palacio, ciudadanos mínimos, etc. siguen intactos), solo se retira la posibilidad de reclutar tropas que consuman el pool de Nobleza.
- **Solución:** `reclutarTropa` (`engine/tropas.ts`) ahora acepta `origen: 'pesants' | 'artesanos'` en vez de estar fijado a Pesants. Se eliminó por completo la función `reclutar()` (el reclutamiento antiguo por tier directo) y la constante `RECLUTAMIENTO`, ya que tras sacar a Artesanos de ese camino y retirar Nobleza, no les quedaba ningún origen válido — limpieza en cascada en `balanceConfig.ts` (grupo "Reclutamiento" y rutas excluidas asociadas). En la UI, el formulario viejo "Reclutar de [origen] / Cantidad / Reclutar" se retiró; el formulario "Reclutar tropa" (equipo) ganó un selector de Origen (Pesants/Artesanos).
- **Verificado:** el selector de Origen en "Reclutar tropa" ofrece exactamente `pesants`/`artesanos`; reclutamiento de Pesants probado de punta a punta (Barracón nivel 1, "Lanceros con escudo de mimbre") tras construir el edificio vía política — funciona igual que antes del refactor. El camino de Artesanos usa la misma función parametrizada, sin lógica especial por origen más allá de qué pool de población descuenta.

### 31. Riesgo aceptado: un asentamiento sin bosque en su radio inicial puede quedar sin madera y sin forma de crecer la zona
- **Hallazgo (al verificar #28+#29 juntas):** un asentamiento fundado sin ningún bosque dentro del radio inicial (30) puede quedar completamente bloqueado: sin Leñera inicial no hay ingreso de madera; la única Leñera posible depende de que la zona alcance un bosque; pero la zona solo crece al completar edificios, y sin madera no puede completarse nada más allá de lo que alcance la reserva de materiales iniciales. Es un bucle sin salida — antes no ocurría porque cualquiera de las dos redes de seguridad (Leñera inicial o crecimiento de zona puramente temporal) bastaba para evitarlo por separado.
- **Reproducido:** un asentamiento fundado a distancia 97 del bosque más cercano (radio inicial 30, tope de zona en nivel 1 = 60) completó una Cantera (dejando la madera exactamente en la reserva mínima, 30) y a partir de ahí no pudo completar nada más — ni una Leñera (bosque inalcanzable) ni el Barracón encolado vía política (madera insuficiente por encima de la reserva). Sin producción de madera, Mantenimiento (que no respeta esa reserva) acabaría agotándola tras el período de gracia, condenando al asentamiento.
- **Decisión del usuario:** riesgo ACEPTADO tal cual — un asentamiento mal ubicado (sin bosque cercano) debe poder fracasar como consecuencia real de una mala elección de fundación, sin ninguna red de seguridad adicional. No se aplicó ningún cambio de código para esto.

Verificado en conjunto: `tsc --noEmit` limpio en cada paso, sin errores de consola, todo confirmado en el navegador con escenarios reproducibles.

---

## Hallazgo posterior: competencia de Vivienda entre Pesants y Artesanos (sometido al Consejo LLM)

Tras un rediseño de crecimiento poblacional (las 3 clases pasan a usar la misma fórmula proporcional `comida × cupo libre × tasa propia`, tasas 0.12/0.05/0.01; se retira `capacidadArtesanos`), un batch de 100 simulaciones + una traza de 8000 ticks reveló un efecto colateral real: con Pesants y Artesanos compartiendo el mismo cupo de Vivienda, y Pesants creciendo ~2.4× más rápido, Artesanos quedaba varado en 1 unidad indefinidamente una vez saturada la vivienda (verificado 7600 ticks sin moverse de 1.0 mientras Pesants estaba fijo en 599). No era un bug — era consecuencia fiel de las reglas dadas, pero contradecía el propósito funcional de Artesanos en el resto del diseño (Doc 4/5: Tier 2 del roster militar, operación de edificios de transformación).

**Decisión (sometida a consejo LLM, ver transcripción de la sesión de diseño en Notion)**: Vivienda pasa a otorgar cupos SEPARADOS por clase (15 Pesants + 5 Artesanos por unidad de Vivienda, escalando linealmente con más Viviendas) en vez de un pool compartido único. Resuelve el problema de raíz sin necesitar una reserva mínima "parche" ni rediseñar cuánta Vivienda total existe.

**Implementado:** `capacidadHabitacional` (`engine/asentamientoQuery.ts`) se divide en `capacidadViviendaPesants`/`capacidadViviendaArtesanos`; `crecerPoblacion` (`engine/population.ts`) calcula el factor de cupo libre por separado para cada clase; el disparador de auto-construcción de Vivienda (`engine/construction.ts`) se dispara si CUALQUIERA de los dos sub-cupos supera el umbral de ocupación, ya que una Vivienda nueva amplía ambos a la vez. `EDIFICIO_CATALOGO.vivienda.capacidadHabitantes` (constants.ts) se divide en `capacidadPesants: 15` + `capacidadArtesanos: 5`.

**Verificado:** re-corrida la misma batería de 100 simulaciones tras el cambio — el `tsc --noEmit` sigue limpio y aparecen subidas de nivel de asentamiento reales por primera vez (34 eventos "sube a nivel" en el batch, 0 antes del fix; algunos asentamientos llegan a colapsar ya en nivel 2 o 3, algo que antes era matemáticamente imposible). Efecto colateral observado, no corregido en esta sesión: al desbloquearse el progreso, aparece una segunda ola de colapsos más tardía (ticks 100-500) en asentamientos que suben de nivel pero no aseguran a tiempo el coste de Mantenimiento adicional (piedra desde nivel 2, oro desde nivel 3) — la tasa de colapso total del batch sube de 52.3% a 63.3%. Queda como posible foco de una futura sesión de calibración.

**Idea derivada, no implementada todavía**: una política que permita redistribuir esa proporción 15/5 por Vivienda (ver Doc 4.4, "Redistribución de Vivienda").

---

## Post-Sprint 6 — Escalón de entrada militar y aviso de viabilidad de fundación (a petición del usuario, verificado por simulación)

El usuario pidió analizar por qué el loop de combate tardaba en arrancar y proponer que, desde nivel 1, se pudiera reclutar tropa "relativamente rápido" para que las primeras escaramuzas ocurrieran pronto. Diagnóstico con un script de prueba externo al repo (mismo método que los diarios de batch — 200 runs × 900 ticks, un agente que solo intenta reclutar la tropa de nivel 1 más barata cada tick, sin tocar el motor): **solo el 5.7% de los asentamientos llegaba a reclutar una sola tropa en 900 ticks, mediana tick 196**. Causa: las 3 tropas de nivel 1 exigían la cadena metalúrgica (Mina de Cobre → Fundición → Armería) o la del cuero (Corral → Curtiduría → Armería) completas, y solo ~6% de los asentamientos nace con un nodo de cobre o livestock dentro de su zona de influencia inicial (radio 30, medido: 6.2%/6.0% respectivamente — coincide con la esperanza de Poisson para 20 nodos de cobre repartidos en un mapa de 1000×1000). Del 94.3% que nunca reclutaba: 57% ni conseguía Barracón, 28% tenía Barracón pero no Armería, 12% tenía ambos pero la Armería nunca producía equipo por falta de insumo. Se detectó además una incoherencia temática que delataba el problema: "Lanceros con escudo de **mimbre**" costaba un arma de **cobre**, y **Honderos** (honda y piedra) costaba armadura de **cuero** — dos tropas de miseria pagadas con tecnología de élite.

Se probaron 3 parches en caliente sobre las constantes (sin tocar la lógica del motor) para aislar qué palanca movía la aguja: (A) una tropa de milicia pagada con madera en bruto + receta de arma de madera en Armería nivel 1, (B) ampliar el radio de zona inicial (30→45), (C) subir la prioridad de score de los edificios de transformación. Resultado: **A por sí sola bajó la mediana de la primera tropa de tick 196 a tick 21** (10× antes) sin empeorar el colapso general (69.8%→71.7%, dentro del ruido). B mejoraba el acceso a recursos pero SUBÍA el colapso a 80.2% (más nodos alcanzables → la auto-construcción compromete más proyectos → se come la reserva de Mantenimiento) — se descartó. C apenas movió nada (6.2% con tropa) — confirma que el cuello de botella nunca fue el orden de la cola de construcción, era la materia prima. Un cuarto factor, ortogonal a los 3 parches, resultó ser el más determinante de todos: forzar que la fundación ocurra en un punto con al menos un bosque alcanzable (la Leñera es la única fuente renovable de madera, y madera paga Mantenimiento desde el tick 1) subió la cobertura de tropa del escenario A del 49.7% al 98.5%, y bajó su colapso del 71.7% al 47.3% — Barracón resultó tener una correlación casi exacta con Leñera (46.2% vs 45.3% en la línea base; 98.8% vs 99.0% con emplazamiento viable), es decir, el gate real del Barracón era la madera, no su propio costo. El hallazgo completo (incluyendo por qué B es una trampa) queda en el diario de batch [`Diario_Simulaciones_Batch_500_Transformacion.md`](Diario_Simulaciones_Batch_500_Transformacion.md) — aunque el título es sobre transformación, el diagnóstico de tropas se hizo con el mismo script de metodología, referenciado ahí.

### 32. Nueva tropa "Milicia de lanceros" + receta "arma de madera" en Armería — escalón de entrada sin metalurgia
- **Contexto:** confirmado con el usuario (opción "milicia nueva + arma de madera" sobre las 3 alternativas presentadas) tras el diagnóstico anterior — el cobre/cuero pasan a ser la MEJORA del roster militar, no el requisito de entrada.
- **Solución:** `EDIFICIO_CATALOGO.armeria` gana una receta nueva en sus 3 niveles internos (`armaMadera`, produccionBase 2, consume `madera: 2` — cifras deliberadamente modestas: `avanzarRecetas` no respeta la reserva dinámica de Mantenimiento, así que una tasa alta convertiría la Armería en una vía de colapso por falta de madera). Va la ÚLTIMA en la lista de recetas de cada nivel a propósito: las recetas se ejecutan en orden sobre el mismo almacén, así que `armaCobre`/`armaBronce` (que también consumen madera) se sirven primero y `armaMadera` absorbe el sobrante. `TROPAS_RECLUTABLES` gana `milicia_lanceros` (Barracón nivel 1, cuesta `madera: 2` en bruto desde el propio Barracón, sin pasar por Armería, `poderBase: 2` — deliberadamente el más débil del roster). `lanceros_mimbre` y `honderos` se recostean de `armaCobre`/`armaduraBasica` a `armaMadera` (corrige la incoherencia temática detectada). Nuevo tipo de recurso `armaMadera` añadido a `RecursoTipo` (`domain/types.ts`), sembrado en el almacén inicial (`settlement.ts`, misma lista que el resto de intermedios de crafting — sin esto, `agregarRecurso` lo crearía con capacidad 0 la primera vez que se produjera, capando la producción en silencio para siempre, mismo footgun ya documentado ahí) y con color propio en `RECURSO_COLOR` (`ui/canvas.ts`).
- **Verificado:** re-corrido el mismo diagnóstico con las cifras finalmente shipeadas (no las probadas en el experimento, que eran más agresivas): 44.3% de los asentamientos llega a reclutar tropa (vs 5.7% antes), mediana tick 21, sin degradar el colapso general (68.8% vs 69.8% en la misma línea base — dentro del ruido de muestreo). `tsc --noEmit` limpio.

### 33. Aviso de viabilidad de emplazamiento al fundar (no bloqueante) — a petición del usuario
- **Contexto:** confirmado con el usuario (opción "solo señalar en la UI, sin bloquear" sobre las 3 alternativas presentadas) — se preserva la libertad de fundar donde se quiera, pero el jugador deja de hacerlo a ciegas. `fundarAsentamiento` sigue aceptando cualquier posición legal, sin cambios.
- **Solución:** nueva función de solo lectura `evaluarViabilidadFundacion` (`engine/settlement.ts`) que, dado un punto, comprueba si está dentro del mapa, si no solapa otra zona, si hay un bosque alcanzable dentro del radio inicial (mismo criterio de "borde, no centro" que ya usa `puntoEnBosqueDentroDeZona` para colocar la Leñera) y qué nodos minerales caen en ese radio — expuesta vía `gameStore.viabilidadFundacion()`. En la UI (`main.ts`), mover el ratón sobre el mapa dibuja un círculo del radio inicial en verde (fundable + bosque alcanzable), ámbar (fundable pero sin madera al alcance) o rojo (posición inválida), más un texto explicando por qué, en el panel de fundación (`drawPreviewFundacion` en `ui/canvas.ts`).
- **Verificado:** `tsc --noEmit` limpio; la previsualización sigue el cursor y cambia de color exactamente en el borde del radio inicial (30 unidades) alrededor de un bosque de prueba, y el mismo criterio (`evaluarViabilidadFundacion`) es el que el diagnóstico de la corrección #32 usó para medir el efecto de "emplazamiento viable" (98.5% de cobertura de tropa vs 49.7% sin filtrar) — no se verificó aparte con un batch nuevo porque es puramente informativo, no cambia el motor.

### 34. Reclutamiento en bloque de tamaño fijo (`unidadesPorDefecto`) en vez de cantidad libre — a petición del usuario
- **Contexto:** el jugador podía reclutar cualquier cantidad de soldados de una tropa (input libre en la UI), contradiciendo la propia definición de "tropa" del diseño (Doc 0/Glosario, Doc 5.8: "el tipo de escuadrón que se recluta DE UNA VEZ"). El usuario aportó una tabla con el tamaño de escuadrón fijo por tropa (25/20/18/15 según nivel, más 25/20 en Galería de tiro); un valor de la tabla (Lanceros pesados micénicos = 115) se confirmó con el usuario que era una errata — queda en 15, igual que Hacheros armados (misma fila de nivel 3).
- **Solución:** `TROPAS_RECLUTABLES` (`constants.ts`) gana el campo `unidadesPorDefecto` por tropa. `reclutarTropa` (`engine/tropas.ts`) pierde el parámetro `cantidad` — la cantidad reclutada es siempre `tropa.unidadesPorDefecto`, `costoEquipo` se sigue multiplicando por ese número (sigue siendo costo POR SOLDADO). `gameStore.reclutarTropa` y la UI (`main.ts`) se actualizan en cascada: se retira el input "Cantidad de unidades".
- **Verificado:** `tsc --noEmit` limpio; reclutar "Milicia de lanceros" en el navegador consume exactamente 50 madera (2/soldado × 25) y crea/amplía el escuadrón en bloques de 25, sin ningún input de cantidad visible.

### 35. Pestaña "Guerra" separada de "Acciones" + segmento "Info:" de tropa + tabla de roster completo — a petición del usuario
- **Contexto:** todos los controles militares (reclutar tropa, Gran Fundición, asedio, combate en campo abierto, interceptar caravana) vivían mezclados con el resto de acciones en una única pestaña "Acciones", y elegir una tropa en el combo de reclutamiento no mostraba ningún detalle (costo, edificio/nivel exigido, poder) más allá de una línea comprimida dentro de la propia opción del `<select>`.
- **Solución:** nueva pestaña "Guerra" (`main.ts`, junto a Acciones/Asentamientos/Jugadores/Políticas/Valores de simulación) con 3 bloques: Reclutamiento, Combate, y Roster de tropas. El bloque de Reclutamiento añade un segmento `.tropa-info` (`ui/style.css`) que se actualiza en cada cambio del combo (`actualizarInfoTropa`) mostrando edificio+nivel requerido, unidades por escuadrón, costo por soldado, costo total del escuadrón y poderBase. El bloque de Roster (`renderRosterTropas`) muestra las 11 tropas del catálogo en 2 tablas (Barracón/Galería de tiro) — se recalcula solo al entrar a la pestaña, no en cada tick.
- **Verificado:** `tsc --noEmit` limpio; en el navegador, la pestaña Guerra aparece junto a las demás, el segmento Info actualiza sus 5 campos al cambiar de tropa en el combo (probado con "Milicia de lanceros": Barracón nivel 1, 25 unidades, 2 Madera/soldado, 50 Madera total, poderBase 2), y la tabla de roster lista las 7 tropas de Barracón + 4 de Galería de tiro con su costo/soldado y unidades.

### 36. Tabla de roster desbordaba el panel de la pestaña Guerra — bug real reportado por el usuario
- **Error:** `#roster-tropas` vivía dentro de una celda del `controls-grid` (ancho mínimo 230px, `grid-template-columns: repeat(auto-fill, minmax(230px, 1fr))`) junto con Reclutamiento y Combate. Con 5 columnas y nombres de tropa largos ("Espadachines con espadas y escudos de bronce"), la tabla desbordaba esa celda y se salía visualmente hacia el mapa.
- **Cómo se detectó:** reportado por el usuario probando la pestaña Guerra recién creada.
- **Solución:** el bloque "Roster de tropas" sale del `controls-grid` de 2 columnas y pasa a un `<div class="detail-section roster-section">` de ancho completo debajo, envuelto además en `.table-scroll` (`overflow-x: auto`) como red de seguridad si el contenido sigue sin caber. Se probó primero forzar `table-layout: fixed` + `word-break` para eliminar el scroll residual, pero eso partía encabezados cortos ("poderBase", "Unidades") letra por letra en columnas muy angostas — se revirtió a favor de acortar los encabezados ("Poder", "Uds.") y dejar el ancho de columna automático.
- **Verificado:** confirmado con JS en el navegador (`document.body.scrollWidth > document.documentElement.clientWidth` → `false` antes y después) que la página ya no desborda horizontalmente; la tabla envuelve el texto largo en varias líneas dentro de su propio contenedor.

### 37. "Milicia de lanceros" pasa de Barracón a Centro Urbano — a petición del usuario
- **Contexto:** aunque la corrección #32 ya desacopló la tropa de entrada de la cadena metalúrgica/cuero, seguía dependiendo de Barracón — que tiene su propio gate indirecto: requiere que el General active la política "Construir Barracón" (Doc 4.4) y que el proyecto pase por la cola de auto-construcción (`tiempoConstruccionTicks: 6`, más el tiempo de espera de cupo). El usuario pidió que la defensa mínima no dependa de ningún edificio con gate — debe depender de Centro Urbano, que nace `activo` con el asentamiento desde el tick de fundación (Doc 1.3), para que CUALQUIER asentamiento pueda defenderse desde el principio así sea con la unidad más débil.
- **Solución:** `TROPAS_RECLUTABLES.milicia_lanceros.edificio` (`constants.ts`) cambia de `'barracon'` a `'centroUrbano'`; el tipo del catálogo se amplía a `'centroUrbano' | 'barracon' | 'galeriaDeTiro'`. `reclutarTropa` (`engine/tropas.ts`) no necesitó cambios — ya resolvía el edificio genéricamente vía `edificiosPorTipoYEstado(asentamiento, tropa.edificio)`, y Centro Urbano siempre tiene exactamente 1 instancia `activa`. En la UI (`main.ts`), el roster (`renderRosterTropas`) gana una tercera sección "Centro Urbano" antes de Barracón/Galería de tiro, y se añade `edificioRequeridoTxt()` para omitir el "nivel N" en el segmento Info y en las opciones del combo cuando el edificio es Centro Urbano (no tiene niveles internos, mostrar "nivel 1" ahí sería ruido).
- **Verificado:** en el navegador, tras fundar y asignar un General (sin construir NADA más), el intento de reclutar en el tick 0 se rechaza únicamente por población ("No hay suficientes pesants disponibles, hacen falta 25" — no por falta de Barracón); avanzando a tick 11 (población ya en 25 pesants) el reclutamiento se completa: `[t11] asentamiento-...: recluta 25 de la tropa "milicia_lanceros" (pesants).` `tsc --noEmit` limpio.

---

## Nota general

Todas las correcciones anteriores son de **diseño/balance**, no de sintaxis: el proyecto compiló sin errores de TypeScript en todo momento salvo en los pasos intermedios normales de refactor (añadir un campo a un tipo y luego actualizar todos los lugares que lo instancian), que se resolvieron sobre la marcha y no se listan aquí por ser rutinarios.
