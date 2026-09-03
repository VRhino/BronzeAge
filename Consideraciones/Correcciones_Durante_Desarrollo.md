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

Se probaron 3 parches en caliente sobre las constantes (sin tocar la lógica del motor) para aislar qué palanca movía la aguja: (A) una tropa de milicia pagada con madera en bruto + receta de arma de madera en Armería nivel 1, (B) ampliar el radio de zona inicial (30→45), (C) subir la prioridad de score de los edificios de transformación. Resultado: **A por sí sola bajó la mediana de la primera tropa de tick 196 a tick 21** (10× antes) sin empeorar el colapso general (69.8%→71.7%, dentro del ruido). B mejoraba el acceso a recursos pero SUBÍA el colapso a 80.2% (más nodos alcanzables → la auto-construcción compromete más proyectos → se come la reserva de Mantenimiento) — se descartó. C apenas movió nada (6.2% con tropa) — confirma que el cuello de botella nunca fue el orden de la cola de construcción, era la materia prima. Un cuarto factor, ortogonal a los 3 parches, resultó ser el más determinante de todos: forzar que la fundación ocurra en un punto con al menos un bosque alcanzable (la Leñera es la única fuente renovable de madera, y madera paga Mantenimiento desde el tick 1) subió la cobertura de tropa del escenario A del 49.7% al 98.5%, y bajó su colapso del 71.7% al 47.3% — Barracón resultó tener una correlación casi exacta con Leñera (46.2% vs 45.3% en la línea base; 98.8% vs 99.0% con emplazamiento viable), es decir, el gate real del Barracón era la madera, no su propio costo. El hallazgo completo (incluyendo por qué B es una trampa) queda en el diario de batch [`Diario_Simulaciones_Batch_500_Transformacion.md`](Diarios_Simulaciones_Batch/Diario_Simulaciones_Batch_500_Transformacion.md) — aunque el título es sobre transformación, el diagnóstico de tropas se hizo con el mismo script de metodología, referenciado ahí.

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

## Post-Sprint 6 — Ampliación de comercio: Mercado como edificio y flota de caravanas propias (a petición del usuario)

El usuario pidió discutir tres carencias del sistema de comercio antes de tocar código: caravanas demasiado lentas, ningún concepto de "cuántas caravanas activas tiene un asentamiento", y que "Mercado" nunca llegó a existir como edificio pese a que el diseño original lo asignaba al Tesorero. Diagnóstico previo (sin cambios de código): con la velocidad original (comercial 8) y el plazo de trueque por defecto (200 ticks), un envío de tamaño moderado a distancia media podía necesitar varios viajes en serie (una sola caravana por lado de acuerdo a la vez, capacidad 60/viaje) que sumaban más ticks que el propio plazo — el acuerdo expiraba matemáticamente antes de poder completarse. Además, `colocarOrdenMercado` no exigía ningún edificio y se liquidaba al instante, mientras que Trueque (el sistema que sí simula viaje y arriesga reputación) era el camino lento — el diseño premiaba por accidente la ruta que menos simula el mundo.

Se discutieron 6 parámetros con el usuario antes de implementar nada (ver también Docs/3.12 y Docs/4.2.1): Mercado como edificio, cupo de caravanas, capacidad de carga, costo de construir una caravana, asignación automática a trueque activo ("solo simulación" — en el diseño objetivo el jugador la elige y carga a mano), y políticas de flota. El usuario confirmó: (1) Mercado gatea tanto la flota como las órdenes de mercado; (2) la asignación cuando hay menos caravanas que envíos se resuelve por scoring ponderado, no FIFO; (3) las caravanas propias NO se pueden desmantelar, solo se pierden capturadas en combate; (4) las cifras propuestas están bien, más un ×2 a la velocidad base de las 4 categorías de caravana.

### 38. Mercado como edificio + flota de caravanas propias + scoring de asignación — a petición del usuario
- **Contexto:** ver discusión de arriba. Cambio grande, multi-archivo, respetando la política de 0 acoplamiento motor/interfaz (recordada explícitamente por el usuario): toda la lógica nueva vive en `engine/*`/`constants.ts`, `main.ts` solo llama a `gameStore`.
- **Solución:**
  - `domain/types.ts`: nuevo `EdificioTipo` `'mercado'`; `Caravana` gana `estado?: 'disponible' | 'en_transito'` (solo para la flota comercial propia, el resto de tipos sigue siendo efímero).
  - `constants.ts`: `EDIFICIO_CATALOGO.mercado` (vía política, sin recetas, niveles con `cupoCaravanas`: 2/4/6); `CARAVANA_CATALOGO` — velocidad ×2 en las 4 categorías, `comercial.costoConstruccion: { madera: 50 }`; `ASIGNACION_CARAVANA` (pesos del scoring: 50% urgencia por expiración, 30% urgencia por volumen, 20% cercanía, distanciaReferencia 600); 4 políticas nuevas del Tesorero (`construir_mercado`, `cupo_caravana_extra` +1 aditivo, `carga_ampliada` ×1.5, `rutas_rapidas` ×1.5).
  - `engine/asentamientoQuery.ts`: `tieneMercadoActivo`, `cupoCaravanas` (nivel de Mercado + bonus de política). `engine/politicas.ts`: `factorCapacidadCaravana`, `factorVelocidadCaravana` (multiplicativos, mismo patrón que el resto) y `sumaFactorPolitica`/`cupoCaravanaExtra` (aditivo, patrón nuevo).
  - `engine/construction.ts`: Mercado se suma a `evaluarEdificiosEspeciales` (mismo cluster de cola vía política que Barracón/Galería/Palacio) y a `EDIFICIOS_CON_NIVELES` (para que `avanzarMejoras` lo mejore de nivel interno igual que el resto).
  - `engine/market.ts`: `colocarOrdenMercado` exige `tieneMercadoActivo`.
  - `engine/trade.ts` (el cambio más grande): `construirCaravanaComercial` (nueva, cuesta madera, respeta cupo, exige Mercado); `avanzarCaravanas` ya no destruye la flota comercial al entregar — vuelve a `'disponible'` en el origen (antes, TODOS los tipos de caravana desaparecían al llegar); `despacharTrueques` se reemplaza por `asignarCaravanasATrueque`, que ya no crea caravanas de la nada — asigna las `'disponibles'` del pool propio a los lados pendientes por `scoreAsignacion` descendente cuando hay más envíos que caravanas.
  - `app/gameStore.ts`: `crearCaravana`, `caravanasInfo` (solo lectura) — únicos puntos de entrada nuevos para la UI.
  - `main.ts`/`ui/canvas.ts`: nombre/función/color de Mercado; panel "Flota de Caravanas" (Acciones) con info en vivo (mercado activo, cupo, disponibles, en tránsito) y botón "Construir caravana".
- **Verificado en el navegador de punta a punta** (con `mercado.costo.piedra` bajado a 0 en el panel de balance solo para saltar la escasez de piedra de este mundo de prueba, no un cambio de código): colocar orden o construir caravana sin Mercado se rechaza (`"necesita un Mercado activo"`); tras construir Mercado (política + auto-construcción especial) y una caravana (consume exactamente 50 madera, queda `disponible: 1`), una orden de mercado se acepta; un trueque activo asigna la caravana automáticamente (`"Caravana comercial de ... sale hacia ..."`) y viaja exactamente `distancia/16` ticks (confirma la velocidad ×2: 735.8 unidades → 46 ticks observados); al entregar, vuelve a `disponible: 1, en_transito: 0` en vez de desaparecer. `tsc --noEmit` limpio en cada paso, sin errores de consola.

---

## Fase 0.3 — Movimiento con terreno, caminos comerciales y chokepoints

### 39. Acoplamiento interfaz/motor en el render de control de chokepoints — corregido a petición del usuario
- **Error:** al implementar el anillo de color que marca qué Facción controla un chokepoint (Doc 1.5), la regla de control ("qué zona de influencia lo cubre") se calculó DENTRO de `ui/canvas.ts` — importando `pointInPolygon` de `world/geometria.ts` y recorriendo `state.zonas` a mano — en vez de reutilizar `controladorDeChokepoint` (`engine/chokepoints.ts`), que ya implementaba exactamente esa regla para el peaje. Violaba el mismo principio de "acoplamiento 0 entre interfaz y motor" que el usuario ya había recordado explícitamente durante la corrección #38 (Ampliación de comercio): la interfaz solo debe pintar datos ya resueltos por `gameStore`, nunca reimplementar una regla de juego por su cuenta. El riesgo concreto: si la regla de control cambiara alguna vez (p. ej. añadir empate por reputación), habría que recordar tocarla en DOS sitios, y `ui/canvas.ts` habría importado lógica de `world/` con el propósito de decidir política de juego (distinto de las lecturas de geometría/terreno ya existentes, que sí son datos puros para pintar).
- **Cómo se detectó:** el usuario, revisando el cambio, señaló la importancia de mantener ese acoplamiento en cero — no fue un fallo de compilación ni de test (`tsc`/`vitest` no detectan duplicación de lógica de negocio entre capas).
- **Solución:** nuevo método de solo lectura `GameStore.chokepointsControl(zonas?)` (`app/gameStore.ts`) que llama a `controladorDeChokepoint` (`engine/chokepoints.ts`) y devuelve un `Map<chokepointId, asentamientoControladorId>` ya resuelto. `DrawState` (`ui/canvas.ts`) gana el campo `chokepointsControl: Map<string, string>`; el bucle que dibuja el anillo pasa de recorrer `state.zonas` con `pointInPolygon` a un simple `state.chokepointsControl.get(chokepoint.id)`. Se retira el import de `pointInPolygon` de `ui/canvas.ts` — ya no queda ningún import de `world/*` en la capa de interfaz salvo el tipo `Mapa` (que la propia fachada, ver `world/mapa.ts`, declara explícitamente como el único puente compartido entre motor e interfaz). `main.ts` pasa a calcular `chokepointsControl` vía `gameStore.chokepointsControl(zonas)` al construir `DrawState`, reutilizando las `zonas` ya obtenidas para esa misma llamada.
- **Verificado:** `tsc --noEmit` limpio; suite completa (118 tests) sigue en verde sin tocar ningún test (el cambio es interno a cómo se calcula un dato de render, no a la regla en sí); en el navegador, fundar un asentamiento y avanzar tick no genera errores de consola y el anillo de control se sigue pintando igual que antes del refactor.

---

### 40. Gate de materia prima + líneas de producción (distancia) + política "Líneas de Producción" — a petición del usuario
- **Contexto:** el usuario señaló, sin pedir cambios de código todavía, dos carencias de la auto-construcción de edificios de transformación (Doc 4.2): (1) Curtiduría/Fundición se auto-construían sin comprobar si el asentamiento tenía algún cobre/livestock, quedando produciendo 0 para siempre en la mayoría de los casos; (2) la distancia entre un extractor y su transformador no tenía ningún efecto en el juego, pese a tener sentido que la producción sea más lenta cuanto más lejos esté la fuente del insumo. Se discutieron ambos puntos en detalle antes de tocar código (definición de "poseer" un recurso, si el trueque cuenta igual que la extracción propia, si la penalización de distancia afecta coste o solo producción, contra qué fuente se mide cada insumo) — ver también el bloque nuevo de 4.2/4.2.1/4.4. Confirmado explícitamente: el gate de materia prima SOLO aplica a la auto-construcción, no a la adición manual; la penalización de distancia SOLO reduce producción, nunca aumenta el coste del insumo por unidad.
- **Solución:**
  - `constants.ts`: `LINEAS_PRODUCCION` (`distanciaSinPenalizacion`, `distanciaMaxima`, `factorMinimo`, `distanciaEstandarSinFuente`) — placeholder sin cifra de diseño previa, editable en el panel de balance (`app/balanceConfig.ts`). Nueva entrada en `POLITICA_CATALOGO`: `lineas_produccion` (Maestro de Obras, campo booleano `lineasProduccionPriorizadas`).
  - `engine/construction.ts`: `tieneInsumoDeArranque` (gate — stock > 0 de al menos un insumo directo de la receta de nivel 1, sin distinguir origen); `RECURSO_A_EXTRACTOR` (recurso crudo → tipo de extractor) y `fuentesDeRecurso` (posiciones de los edificios activos que producen un recurso, crudo o intermedio); `factorPorDistancia` (curva lineal con suelo) y `factorLineaProduccion` (mínimo entre insumos — el eslabón más débil manda), aplicado como multiplicador de `cantidad` en `avanzarRecetas` sin tocar `consumePorUnidad`. El bucle de auto-construcción de Curtiduría/Armería/Fundición en `evaluarNecesidades` pasa de "primer tipo sin instancia, sitio o no" a "primer tipo sin instancia QUE PASE EL GATE" (efecto colateral bueno: ya no se detiene en Curtiduría cuando esta no pasa el gate, sigue evaluando Armería/Fundición en la misma pasada). `puntosConcentricos` (refactor: extrae el barrido de anillos que antes vivía solo dentro de `sitioConcentrico`) + `sitioConcentricoLineaProduccion` (misma geometría, pero evalúa TODOS los huecos y devuelve el que minimiza la penalización contra las recetas de nivel 1 del tipo) — se usa en vez de `sitioConcentrico` cuando la política está activa.
  - `engine/politicas.ts`: `algunaPoliticaActiva` (helper genérico de flag booleano, mismo patrón que `valorMaximoPolitica`/`sumaFactorPolitica` ya existentes) + `lineasProduccionPriorizadas` exportado.
  - `src/engine/__tests__/lineas_produccion.test.ts` (nuevo, 15 tests): gate por tipo de edificio (unit, vía `tieneInsumoDeArranque`) + un caso de simulación real (cobre inyectado → Fundición se auto-construye; livestock en 0 → Curtiduría nunca aparece en 40 ticks); `factorPorDistancia`/`factorLineaProduccion` (clamp, monotonía, fuente más cercana, insumo intermedio vs. crudo, eslabón más débil); política (`lineasProduccionPriorizadas` antes/después de `activarPolitica`, `sitioConcentricoLineaProduccion` nunca peor que `sitioConcentrico`, y un caso de simulación real comparando dónde queda la Fundición con la política activa vs. sin ella).
- **Verificado:** `tsc --noEmit` limpio; suite completa (24 archivos, 151 tests) en verde, incluida la existente (sin tocar ningún test previo). Durante la verificación con simulación real se descubrió (no relacionado con este cambio) que el asentamiento de la seed de test (42) no tiene ningún nodo mineral alcanzable en su zona — sin inyectar recursos en el almacén, ninguna Curtiduría/Fundición/Armería llegaba a construirse nunca por falta de PIEDRA para pagar el costo de construcción (20 inicial < 30 del costo), una limitación preexistente del mapa/seed y no un bug de este cambio; los tests de simulación real inyectan cobre/piedra directamente para aislar el comportamiento del gate del de la disponibilidad de piedra.

### 41. Política "Protección de Riesgos" retirada — a petición del usuario
- **Contexto:** el usuario pidió eliminarla razonando que "el canal de construcción vía política ya se eliminó" (refiriéndose al retiro de las 4 políticas "Construir Barracón/Galería de tiro/Palacio/Mercado", corrección de la sección "Ampliación de comercio" más arriba). Verificado antes de tocar nada: ese canal viejo (gate de EXISTENCIA de un edificio detrás de una política, con cluster de cola aparte) ya no tenía ni rastro en el código — pero Protección de Riesgos era un mecanismo DISTINTO (reprioriza auto-construcción hacia Leñera/Granja, no gatea la existencia de ningún edificio) que sí seguía activo, documentado (Docs 4.2/4.4) y expuesto en la UI (`main.ts`). Se confirmó explícitamente con el usuario que quería eliminarla de todos modos, tras señalarle la diferencia. Se revisó el resto del catálogo de políticas buscando otros restos del canal viejo — no se encontró ninguno más (Vía Rápida, Postura Defensiva, Líneas de Producción, Edicto de Cosecha, Racionamiento, Culto a la Fertilidad, políticas de Tesorero/General: ninguna gatea la existencia de un edificio, todas son factores/prioridades/flags sobre auto-construcción ya universal).
- **Solución:** `constants.ts` — se quita la entrada `proteccion_riesgos` de `POLITICA_CATALOGO`. `engine/politicas.ts` — se quitan `valorMaximoPolitica` (helper genérico que ya no tiene ningún campo que leer) y sus dos exports `minimoLenerasPrioritario`/`minimoGranjasPrioritario`. `engine/construction.ts` — se quita por completo el bloque `soloSupervivencia` de `evaluarNecesidades` (el `if (objetivoLenerasPrioritario > 0 || ...)` que podía bloquear el resto de la función); el cuerpo que antes vivía dentro de `if (!soloSupervivencia) { ... }` (Granja/Leñera/extractores/Vivienda/Almacén/transformación/Carpintería) queda sin ese wrapper, ejecutándose siempre — el resto de la lógica de prioridad no cambia, sigue siendo puro scoring por bandas (`SCORE_BANDAS`). `main.ts` — se quita el `if` de `efectoPolitica` que mostraba el texto de "Prioriza Leñeras..."; de paso se añade el texto que le faltaba a "Líneas de Producción" (caía en el fallback genérico "Sin efecto mecánico modelado todavía").
- **Verificado:** `tsc --noEmit` limpio; suite completa (24 archivos, 151 tests) sigue en verde sin tocar ningún test (no había tests dedicados a Protección de Riesgos). Confirmado por grep que no queda ninguna referencia a `proteccion_riesgos`/`minimoLenerasPrioritario`/`minimoGranjasPrioritario`/`soloSupervivencia` en `src/`.

---

## Fase 0.5 (en curso) — Especialización de asentamientos

### 42. Deadlock real de arranque de comercio (Mercado exigía piedra) — detectado instrumentando, corregido a petición del usuario
- **Contexto:** durante el diseño de Fase 0.5 (`Consideraciones/Fase_0_5_Definicion_Especializacion_y_Cupos.md`), antes de tocar cupos/roles se instrumentó el motor para medir si el excedente que el diseño da por hecho existe de verdad. Primer paso: `pausadoPorAlmacenLleno` (`domain/types.ts` `Edificio`, `engine/almacen.ts` `agregarRecursoConSobrante`) — antes, la producción que no cabía en el almacén se perdía en silencio vía `Math.min(capacidad, ...)` en `agregarRecurso`; ahora el edificio queda marcado, sin cambiar el comportamiento numérico (mismo clamp, solo se reporta). Conectado en los 4 puntos de producción de `engine/construction.ts` (Granja, Leñera, extractores minerales, recetas de transformación).
- **El hallazgo real:** al medir sobre el mapa completo (`MAPA_DEFAULT` 2000×2000, 5 seeds) qué fracción de posiciones fundables tiene algún mineral/livestock alcanzable dentro del radio de zona por nivel (`ZONA_INFLUENCIA.radioMaximoPorNivel`: 60/90/120), el resultado fue que en nivel 1 el **53%-98% de las posiciones fundables no tocan ningún mineral** (seed 3 es un caso extremo, 1.7%; los mejores seeds solo llegan a 33-47%). Confirmado con el usuario que esta escasez es INTENCIONAL (fomenta comercio temprano) — pero instrumentar el sistema de comercio que se supone la resuelve (Doc 3.2/3.12) expuso que **ese sistema era él mismo inalcanzable** para esos mismos asentamientos: Mercado costaba 100 madera + 40 piedra; la reserva de fundación (Doc 1.3) es de 20 piedra, y un asentamiento sin mineral alcanzable nunca junta más — fijo en 20 para siempre. Además, `asignarCaravanasATrueque` (`engine/trade.ts`) exige que CADA LADO de un trueque tenga su propia caravana en su propio origen para entregar lo pactado — no basta con que el lado rico tenga Mercado y caravana. Consecuencia: un asentamiento sin piedra no podía construir Mercado (necesita piedra) → no podía construir caravana (necesita Mercado) → no podía entregar su lado de NINGÚN trueque, ni para dar ni para recibir. Deadlock total y estructural, no una particularidad de un seed de test.
- **Solución:** `constants.ts` `EDIFICIO_CATALOGO.mercado.costo` pasa de `{madera: 100, piedra: 40}` a `{madera: 100}` — a petición explícita del usuario, que precisó mantener el coste de piedra en las MEJORAS de Mercado (nivel 2: 150 madera + 100 piedra; nivel 3: 450 madera + 200 piedra, sin cambios): para cuando el asentamiento mejora el Mercado ya tuvo tiempo de conseguir piedra, por extracción propia o por el comercio que el Mercado nivel 1 acaba de destrabar. Actualizado `Docs/4_Sistema_de_Poblacion_Construccion_y_Mantenimiento.md` (catálogo de Mercado, 4.2.1).
- **Verificación de punta a punta:** se construyó `engine/simulacionAutoComercio.ts` — un NPC de trueque **solo para simulación** (banner explícito, apagado por defecto vía `SIMULACION_AUTO_COMERCIO.activo = 0` en `constants.ts`, togglable en caliente desde `app/balanceConfig.ts`, enganchado en `app/gameStore.ts` FUERA de `avanzarSimulacion` — el juego real sigue siendo 100% manual, Doc 3.2) que asegura Gobernador+Tesorero, construye Mercado/caravana propia, y empareja déficit/superávit de piedra/cobre/estaño/oro/livestock entre asentamientos de la misma Facción proponiendo trueques (pagando con el excedente real de madera/trigo del lado deficitario). Tuvo que asignar también Tesorero + `reservaManual.madera` (mecanismo YA existente para un jugador humano, Doc 4.5) porque sin reservarla, la auto-construcción del propio asentamiento le ganaba la madera a la caravana antes de que el NPC tuviera su turno — mismo patrón que un Tesorero humano usaría. Probado sobre el mapa completo con dos asentamientos de la misma Facción (uno con piedra alcanzable, otro sin ella): Mercado construido en ambos, caravanas propias, trueque propuesto y CUMPLIDO con entrega física por el mapa — el asentamiento sin mineral pasó de 20 piedra (reserva inicial estancada) a 290 hacia el tick 225 de una simulación de 800 ticks, con 10+ acuerdos cumplidos.
- **Verificado:** `tsc --noEmit` limpio en cada paso; suite completa (29 archivos, 184 tests) en verde sin tocar ningún test existente. Los scripts de instrumentación/prueba usados para medir y validar (`*.script.test.ts`) se borraron tras extraer los datos — no quedan en el repo, son desechables por diseño.

---

### 43. Núcleo mecánico de Fase 0.5 (techo de población, split nivelAlcanzado/nivelActual, mantenimiento por población, XP de Facción, cupo por nivel) — implementado
- **Contexto:** tras discutir y cerrar el diseño de `Consideraciones/Fase_0_5_Definicion_Especializacion_y_Cupos.md` (consejo LLM + 9 correcciones del usuario), se implementó el núcleo mecánico completo en un solo pase, en orden de dependencia: (1) nivel de Facción por XP — prerrequisito de (5); (2) techo de población por nivel; (3) split `nivel`/`nivelActual` + degradación escalonada; (4) mantenimiento por población; (5) cupo de asentamientos por nivel. Deliberadamente FUERA de esta pasada: invertir los gates de nivel (Armería/Curtiduría/Fundición siguen siendo requisito de nivel 2, no premio), materiales procesados de construcción (§3.3), y roles/vocación con cuota (§4) — quedan como diseño puro, ver puntos abiertos #3/#4 y §4 del documento.
- **Nivel de Facción por XP** (`domain/types.ts` `Faccion.experiencia`, `constants.ts` `NIVEL_FACCION` reescrito por completo — antes sumaba población total de la Facción, lo que el consejo LLM detectó que autodesactivaba el cupo de §5 al instante): `engine/faccion.ts` `calcularNivelFaccion` ahora deriva el nivel puramente de `experiencia` contra `NIVEL_FACCION.xpParaNivel` (curva de umbrales acumulados, ×~1.6 por nivel); `aplicarAjustesExperiencia` (nuevo, mismo patrón que `aplicarAjustesReputacion` ya existente en `engine/reputacion.ts`) aplica ganancias de combate/construcción/conquista/caravanas. XP otorgada en las 4 funciones de combate (`engine/combate.ts`: `iniciarAsedio`, `combateCampoAbierto`, `interceptarCaravana`, `atacarCampamentoBandidos` — las dos últimas ganaron parámetro `facciones`, antes no lo necesitaban) y en `engine/simulation.ts` (construcción de edificios, vía nuevo campo `edificiosCompletados` en el retorno de `avanzarConstruccion`). `avanzarNivelesFaccion` pierde el parámetro `asentamientos` (ya no hace falta, todo sale de la XP ya acumulada). `engine/fusion.ts` inicializa `experiencia: 0` en la Facción fusionada (mismo criterio que `reputacion: 0`, ya existente: "nace una Facción nueva sin historial propio").
- **Techo de población por nivel** (`constants.ts` `NIVEL_ASENTAMIENTO.techoPoblacion`, placeholder 300/1500/6000): `engine/population.ts` `crecerPoblacion` escala a la baja el crecimiento de las 3 clases este tick si juntas se pasarían del techo — nunca purga población ya asentada, solo limita cuánta puede sumarse.
- **Split `nivel` (nivelAlcanzado) / `nivelActual`** (`domain/types.ts`, campos nuevos `nivelActual?`/`rachaMantenimientoSano?`, ambos opcionales con fallback — partidas guardadas antiguas tratan `nivelActual` ausente como igual a `nivel`, vía helper `nivelActualDe` nuevo en `engine/asentamientoQuery.ts`): `nivel` sigue siendo monótono (nunca baja, gates de población+edificios de siempre) y de ahí salen el techo de población y el techo de RADIO de zona de influencia (decisión: el territorio NO se encoge). `nivelActual` puede subir y bajar — de ahí salen los gates de CONSTRUIR/MEJORAR/RECLUTAR/lanzar Caravana de Fundación, actualizados en `engine/construction.ts` (`alcanzoTopeDeAlmacenes`, gate de Carpintería, gate de mejora en `avanzarMejoras`, `anadirEdificioManualmente`) y `engine/expansion.ts` (gate de nivel 2 de la Caravana de Fundación) para leer `nivelActualDe(asentamiento)` en vez de `asentamiento.nivel`. `engine/mantenimiento.ts` `avanzarMantenimiento`: al tocar 0 el medidor, si `nivelActual > 1` degrada un escalón y reinicia el medidor a 100 (NO destruye, NO reduce el consumo/mantenimiento — la misma gente sigue comiendo lo mismo); solo cae en ruinas si ya estaba en `nivelActual` 1. Recupera un escalón tras `MANTENIMIENTO.ticksSanosParaRecuperarNivel` (30) ticks SEGUIDOS de pago íntegro (`rachaMantenimientoSano`, se resetea en cualquier tick con déficit) — evita el yo-yo de nivel que había señalado el consejo LLM. `avanzarNivelAsentamiento` sincroniza `nivelActual` con `nivel` de inmediato al promover legítimamente (sin esperar racha), salvo que ya estuviera degradado de antes.
- **Mantenimiento por POBLACIÓN, no por nivel** (`constants.ts` `MANTENIMIENTO.poblacionReferencia` nuevo, reemplaza `factorCrecimientoPorNivel` retirado): `calcularCostoMantenimiento` calcula `factorPoblacion = 1 + poblacionTotal/poblacionReferencia` en vez de escalar por `(nivel-1)`. `nivelParaPiedra`/`nivelParaOro` (qué recursos se cobran, no cuánto) siguen atados a `nivel` (nivelAlcanzado) sin cambios — coherente con que degradar no reduce mantenimiento. Se descartó explícitamente sumar coste por Nº de edificios (propuesta anterior del boceto): chocaba con el factor de distancia a la capital ya existente, que ya es el mecanismo real de "imperio disperso cuesta más", y lo habría castigado doble.
- **Cupo de asentamientos por nivel** (`constants.ts` `CUPO_NIVEL_ASENTAMIENTO.maxNivel2`/`maxNivel3`, curva por nivel de Facción con el invariante `maxN2+maxN3 = CAP_FUNDACION_POR_NIVEL-1` — siempre queda al menos un asentamiento en nivel 1): `engine/faccion.ts` `calcularCupoNivel` (mismo patrón que `calcularCapFundacion` ya existente). Enforcement en `avanzarNivelAsentamiento` (`engine/mantenimiento.ts`), que ahora acepta un callback opcional `tieneCupoParaNivel(nivelObjetivo): boolean` y sube de a un escalón por vez, llamando al callback en cada escalón — si no hay cupo, se detiene ahí (el asentamiento queda "elegible, esperando cupo", nunca se bloquea ni retrocede). `engine/simulation.ts` calcula el cupo libre por Facción al INICIO del tick (a partir de cuántos asentamientos YA están en cada nivel) y lo consume/libera dentro del mismo tick a través del callback — subir de 2 a 3 libera el cupo de 2 que se abandona, disponible para otro asentamiento propio procesado más tarde en la misma pasada. **Decisión interina sin confirmar por el usuario**: "el primero que llega" (orden del array), no promoción manual del Rey; conquista no se re-evalúa contra el cupo (mismo criterio que `CAP_FUNDACION_POR_NIVEL`, Doc 1.7).
- **Hallazgo sin resolver, confirmado real (no solo teórico) al implementar**: con cupo de nivel 2 en 0 a nivel de Facción 1, una Facción recién creada no puede promover NINGÚN asentamiento a nivel 2 — y la Caravana de Fundación (Doc 1.8) exige nivelActual 2 como mínimo. Deadlock de expansión temprana real, pendiente de que el usuario decida cómo resolverlo (ver punto abierto #6 del documento de diseño).
- **Verificado:** `tsc --noEmit` limpio en cada paso. Suite completa (29 archivos, 184 tests) en verde — dos actualizaciones de snapshot CONSCIENTES (`vitest run -u`, revisadas antes de aceptar): (a) nivel de Facción ya no llega a nivel 2 tan rápido en 100 ticks con XP como con la fórmula de población vieja — es el cambio de diseño esperado, el propio nombre del test lo advierte; (b) deriva numérica menor (madera/población ±1) en el snapshot de mantenimiento, consecuencia esperada del cambio de fórmula. Además, 3 scripts de verificación aislada desechables (`*.script.test.ts`, borrados tras confirmar): degradación 2→1 sin destruir y sí destruye desde nivel 1; recuperación exacta en el tick 30, ni antes ni después; cupo — sin cupo se queda elegible sin bloquearse, con cupo parcial se detiene en el escalón correcto, salto directo 1→3 pide cupo en orden 2 luego 3 (nunca al revés), compatibilidad sin callback.

---

### 44. Hambruna de población (nutrición 0-100 + muerte por hambre sostenida) — a petición del usuario
- **Contexto:** el usuario señaló que un asentamiento sin trigo para mantener a su población no tenía NINGÚN efecto negativo real — `crecerPoblacion` (`engine/population.ts`) solo frenaba el crecimiento con un booleano `trigo>0 ? 1 : 0.2` y `consumirComida` se limitaba a descontar lo disponible sin más consecuencia, mientras que las TROPAS ya tenían resuelto exactamente este mismo problema con raciones (moral que colapsa sin suministro → deserción permanente, `avanzarMantenimientoTropas`, Doc 5.4) — población civil era la única "clase" del juego sin castigo real por hambre. Se plantearon 5 opciones (espejo de tropas / emigración en vez de muerte / activar el placeholder de `felicidad` / degradar el medidor de Mantenimiento / penalización de producción sin muerte) y se confirmó con el usuario ir con una combinación de la 1 (espejo de tropas: moral→deserción) y la 3 (que la nutrición module el crecimiento en vez de un booleano), descartando explícitamente enganchar esto al medidor de Mantenimiento (§4.5 ya documenta por qué el trigo se sacó de ese cálculo, a propósito, para no duplicar la mecánica).
- **Solución:**
  - `domain/types.ts`: `Asentamiento.nutricionPoblacion?: number` (0-100, ausente = 100 vía helper, mismo patrón opcional que `nivelActual`).
  - `constants.ts`: `POBLACION.hambre` (`nutricionInicial: 100`, `regeneracionPorTick: 5`, `degradacionSinComida: 20`, `factorCrecimientoMinimo: 0.2`, `umbralMuertePorHambre: 0`, `fraccionMuertePorTickHambre: 0.05`) — valores calcados a propósito de `MILITAR.regeneracionMoralPorTick`/`degradacionMoralSinRacion`/`desercionFraccionPorTickSinMoral` para que ambos sistemas colapsen al mismo ritmo (5 ticks sin suministro) y cuesten la misma fracción por tick.
  - `engine/asentamientoQuery.ts`: `nutricionPoblacionDe` (fallback a 100).
  - `engine/population.ts`: `consumirComida` se renombra y amplía a `avanzarNutricionPoblacion` — descuenta el trigo disponible (igual que antes) y además sube/baja `nutricionPoblacion` según la fracción de consumo cubierta; con la nutrición en el umbral (0) sostenido, resta cada tick `fraccionMuertePorTickHambre` de pesants+artesanos (nobleza protegida — "los nobles comen primero") y emite un evento. `crecerPoblacion` reemplaza el booleano de comida por un factor lineal entre `factorCrecimientoMinimo` (nutrición 0) y 1 (nutrición 100).
  - `engine/simulation.ts`: `avanzarNutricionPoblacion` corre en el mismo punto donde corría `consumirComida`, antes de `crecerPoblacion` (para que el factor de crecimiento de este tick ya vea la nutrición actualizada) y después de `avanzarNivelAsentamiento`.
  - `main.ts`: barra "Nutrición" en el panel de detalle del asentamiento, junto a la de Mantenimiento (mismas clases CSS `.mantenimiento-bar`/`.mantenimiento-fill`, sin CSS nuevo).
  - `Docs/4_Sistema_de_Poblacion_Construccion_y_Mantenimiento.md`: nuevo párrafo "Hambruna" en 4.1; corregida la nota de 4.5 que decía "un déficit de trigo solo frena el crecimiento" (ya no es cierto — ahora también cuesta población si se sostiene, solo queda desacoplado del medidor de Mantenimiento en sí).
  - `src/engine/__tests__/hambruna.test.ts` (nuevo, 4 tests): nutrición se mantiene en 100 sin déficit; colapsa a 0 en exactamente `nutricionInicial/degradacionSinComida` ticks sin trigo; hambre sostenida cuesta pesants/artesanos y nunca nobleza; la nutrición baja reduce el crecimiento (cota determinista por magnitud, sin necesidad de mockear `Math.random`).
- **Verificado:** `tsc --noEmit` limpio; suite completa (32 archivos, 202 tests) en verde, INCLUIDO el snapshot de regresión general sin necesitar actualizarlo (`vitest run -u`) — la simulación de referencia (seed 99, 100 ticks) nunca entra en déficit de trigo, así que el nuevo factor lineal coincide con el viejo booleano en ese caso (`trigo>0` ⇒ nutrición se mantiene en 100 ⇒ factor 1, igual que antes). Probado también de punta a punta en el navegador (fundar facción/asentamiento, avanzar tick, panel de Asentamientos): la barra "Nutrición 100/100" se muestra correctamente y varios ticks no generan errores de consola ni del servidor de Vite.

---

### 45. Granja no escalaba proactivamente con la población, se quedaba atrás sobre todo en nivel 1 — a petición del usuario
- **Contexto:** investigando `issues/granjas_no_escalan_con_poblacion.md` (mano de obra compartida crea un equilibrio de hambre estable, confirmado por experimento A/B en batch) desde el ángulo de la auto-construcción: el disparador de Granja (`evaluarNecesidades`, `engine/construction.ts`) solo reaccionaba al déficit YA ocurrido (`producción actual < consumo actual`), un paso por detrás de la población siempre por diseño — la Vivienda, en cambio, ya se auto-construye de forma PROACTIVA contra su propio umbral de ocupación (85%), antes de saturarse. Además, mejorar una Granja existente (`avanzarMejoras`) es labor-neutral (`trabajadoresRequeridos` fijo en los 4 niveles internos, solo sube el rinde) mientras que construir una nueva diluye el `ratioManoObra` compartido con el resto de edificios productores — pero la preferencia por mejorar sobre construir era puramente accidental (orden de ejecución en el mismo tick), y el costo geométrico de la mejora (×2 por nivel) la anulaba en la práctica frente al costo fijo de una Granja nueva. El usuario pidió aplicar el arreglo pero DESCARTAR explícitamente la opción de gatear la construcción por mano de obra libre disponible (opción 3 del issue) — queda fuera, no implementada.
- **Solución** (`engine/construction.ts`, bloque de Granja en `evaluarNecesidades`):
  - **Objetivo proactivo además del reactivo**: `enDeficitProyectado`, calculado contra la capacidad de Vivienda ya construida/en camino (`capacidadViviendaPesants`/`capacidadViviendaArtesanos`), no la población que ya llegó — mismo criterio que Vivienda. `Math.max` garantiza que nunca es menos estricto que el déficit reactivo original (`enDeficitTrigo`, sin tocar).
  - **Prioridad real a MEJORAR sobre CONSTRUIR**: `hayMejoraGranjaDisponible` — si alguna Granja activa por debajo de nivel máximo puede pagar YA su siguiente mejora (misma reserva que protege el resto de construcción), se suprime la propuesta de Granja nueva ese tick. Con 0 Granjas activas es trivialmente falso: el arranque nunca se bloquea.
  - **Matiz descubierto en verificación, no en el diseño inicial**: dejar que el objetivo proactivo por sí solo desbloqueara el modo ráfaga existente (`maximoGranjasPendientesEnDeficit = 3`, hasta 3 Granjas en camino a la vez) sobre-construía — la capacidad de Vivienda casi SIEMPRE va por delante de la población real (así está diseñada), así que ese margen normal se trataba como una emergencia constante. Verificado en simulación: salían 4 Granjas en vez de 1-2, diluyendo material que Fundición/Armería necesitaban (rompió `lineas_produccion.test.ts` — Fundición dejaba de auto-construirse en 100 ticks pese a tener sus insumos). El modo ráfaga se dejó reservado al déficit REACTIVO (`enDeficitTrigo`) únicamente; lo proactivo solo adelanta CUÁNDO se pide la siguiente Granja, de una en una.
- **Verificado:** `tsc --noEmit` limpio; suite completa (34 archivos, 208 tests) en verde. Se actualizó conscientemente el snapshot versionado (`snapshot_baseline.test.ts.snap`, `vitest run -u`, diff revisado) — cambio de comportamiento intencional: en el corte de tick 100 de la simulación de referencia (seed 99), trigo sube de 601 a 829, 3 Granjas quedan todas mejoradas en vez de "3 activas + 1 en camino", una Vivienda más. Además se instrumentó un caso concreto fuera de la suite (Fundación forzada a nivel 2, cobre/piedra inyectados, seed 42, 100 ticks) para diagnosticar la sobre-construcción antes de la recalibración: con el objetivo puramente proactivo desbloqueando la ráfaga aparecían 4 Granjas y Fundición nunca llegaba a construirse; con la recalibración (ráfaga solo por déficit reactivo) aparecen 2 Granjas, ambas mejoradas a nivel 3-4, y Fundición queda activa hacia el tick 94 — dentro de la ventana que ya exigía el test existente.
- **Cierre del issue (mismo día)**: el usuario descartó explícitamente las opciones 2 (separar el pool de mano de obra por categoría — era el arreglo de raíz, rompe el acoplamiento entre `ratioManoObra` y todos los edificios productores), 3 (gate por mano de obra libre) y 4 (revisar `factorCrecimientoMinimo`/`umbralMuertePorHambre`). El issue queda cerrado con el alcance de esta pasada — el mecanismo de fondo (mano de obra compartida entre todos los edificios productores) sigue existiendo en el código tal cual lo documenta `issues/granjas_no_escalan_con_poblacion.md`, solo se mitigó el síntoma del lado de la auto-construcción de Granja.

---

## Etapa 5 (árbol único de anclas) — dos bugs reales de proliferación/reparto, encontrados con un laboratorio visual aparte

El árbol único de anclas (Etapa 5, `Consideraciones/Vista_Asentamiento_Trazado_Urbano.md` §5.4-5.7) se había
verificado hasta ahora solo por simulación abstracta y por los 237 tests existentes — ninguno de los dos
expone cómo se ve un asentamiento real creciendo durante cientos de ticks. Para poder inspeccionarlo se
construyó `laboratorio.html` + `src/lab/*` (un asentamiento aislado sobre el motor real, `avanzarSimulacion`,
sin partida ni facciones alrededor — herramienta de desarrollo interna, no parte del juego ni de la suite de
tests) con una tabla por ancla (semilla activa/saturada, ancla llena, huérfana, distancia a su padre, código
de posición en el árbol tipo "R"/"a"/"a1") y una vista de árbol con resaltado cruzado mapa↔árbol al pasar el
cursor. Jugando con ella el usuario encontró dos bugs reales de la Etapa 5 (más un desliz de mi parte al
corregir el primero, revertido antes de shipear nada) y un tercero que queda documentado como abierto.

### 46. Proliferación de anclas residenciales: la Lógica 2 nunca reutilizaba una instancia que no fuera la más cercana al origen
- **Error:** con más de una instancia de ancla de la misma categoría (ej. dos Plazas), `sitiosParaTipo` elegía
  siempre la más cercana al ORIGEN del asentamiento (`anclaMasCercana`), sin memoria de si tenía hueco. Centro
  Urbano tiene `posicion=(0,0)` exacto, así que nunca perdía ese puesto: en cuanto su núcleo de satélites se
  llenaba para siempre, `asegurarAnclaPara` seguía sondeando esa misma instancia llena y fabricaba un ancla
  nueva en cada ronda de demanda en vez de reutilizar cualquiera de las que ya tenían hueco de sobra.
- **Cómo se detectó:** reportado por el usuario probando el laboratorio (seed 1): la tabla mostraba 8 anclas
  residenciales, ninguna marcada como saturada de verdad, dos de ellas huérfanas.
- **Solución:** nuevo campo persistido `Edificio.anclaLlena` (se marca una sola vez, nunca se revisa — mismo
  patrón que `semillaSaturada` del árbol) + `anclaActivaParaCategoria` (`engine/trazado.ts`), que recorre las
  instancias de la categoría de más cerca a más lejos del origen saltando las `anclaLlena`, probando cada una
  con la colocación real (`sitiosPorAtraccionDura`) hasta encontrar la primera con hueco. `asegurarAnclaPara`
  (`engine/construction.ts`) la usa antes de considerar crear un ancla nueva. Detalle de diseño confirmado
  explícitamente por el usuario tras una pregunta directa: **persistido, igual que `semillaSaturada`** — no se
  recalcula en vivo en cada consulta. Ver `Vista_Asentamiento_Trazado_Urbano.md` §5.3.1 para el diseño completo.
- **Desliz corregido en el camino, no shipeado:** una primera versión de la corrección hacía que `anclaLlena`
  disparara también el avance de `semillaActiva` (Lógica 1) — el propio usuario lo señaló como error suyo
  ("ancla llena no dispara nueva semilla, solo ancla saturada dispara cambio de semilla") apenas lo vio en el
  laboratorio, y se revirtió por completo antes de continuar. `anclaLlena` (Lógica 2) y `semillaSaturada`
  (Lógica 1) quedan como criterios independientes, documentado explícitamente en el comentario de ambos campos
  (`domain/types.ts`) para que no se vuelvan a mezclar.
- **Verificado:** `tsc --noEmit` limpio; 237/237 tests en verde; en el laboratorio (seed 1, tick 300 y 450) el
  total de anclas se mantuvo estable en 4 en vez de seguir creciendo sin límite (184 anclas hacia el tick 400
  en la corrida con el bug).

### 47. Ranuras del árbol repetidas: una semilla podía nacer varias anclas hijas en la misma dirección
- **Error:** `direccionesBarajadas` da el mismo orden de las 8 direcciones cardinales cada vez que se consulta
  la MISMA semilla (determinista por diseño). Si la primera dirección de esa lista ya tenía un ancla hija,
  `huecoEnDireccion` expandía el radio de búsqueda EN ESA MISMA DIRECCIÓN en vez de probar una de las otras 7
  todavía libres — varias anclas hijas de una semilla terminaban alineadas en un solo rayo, a distancias
  crecientes, en vez de repartirse alrededor.
- **Cómo se detectó:** reportado por el usuario viendo la vista de árbol del laboratorio: dos anclas con el
  mismo código de ranura (ej. "a" repetido, a distancias muy distintas) colgando de la misma semilla.
- **Solución:** nueva función `ranuraOcupada` (`engine/trazado.ts`) — comprueba geométricamente si alguna
  ancla ya existente cae casi exacto en el ángulo de una dirección dada, dentro del rango de radio de una
  ranura. `crearAnclaNueva` separa las 8 direcciones en libres/ya usadas y prueba primero las libres (a la
  mínima distancia que cada una permita), recurriendo a expandir una ya usada solo si ninguna libre sirve.
- **Verificado:** `tsc --noEmit` limpio; 237/237 tests en verde; en el laboratorio (seed 1, tick 300 y 500) los
  tres hijos de la raíz quedaron en tres ranuras distintas ("a", "h", "f") a distancias mínimas (6.0-7.5
  celdas) sin ningún código repetido en la tabla, contra el caso anterior con dos anclas compartiendo ranura
  "a" a distancias de 5.5 y 10.0.

### Abierto: ancla creada sin garantizar sitio para al menos un satélite
El laboratorio (seed 70) expone un tercer caso, todavía sin corregir: un Patio de Gremios nace consistente y
queda huérfano para siempre — ningún edificio de industria consigue nunca pegársele. Coincide con el
diagnóstico original que motivó toda la Etapa 5: `crearAnclaNueva`/`asegurarAnclaPara` garantizan que el ANCLA
en sí cabe (hueco + separación) antes de colocarla, pero nunca comprueban que, una vez colocada,
`sitiosPorAtraccionDura` vaya a encontrarle sitio a algún satélite de la categoría que la disparó. Documentado
también en `Vista_Asentamiento_Trazado_Urbano.md` §"Abierto".

Además, mientras se construía el laboratorio se añadió una pestaña de "Construcción manual" (dropdown con los
tipos construibles a mano + botón "Encolar", mismo camino que `anadirEdificioManualmente` del motor real) y
9999 de cada material sembrados al fundar, para poder forzar demanda de cualquier edificio sin que la
escasez de recursos sea una variable más a controlar al probar el árbol de anclas.

---

## Rebalanceo de trigo y §E6.16 (2026-09-02)

### 48. Producción base de Granja ×2 — el asentamiento nacía en déficit estructural

**Encontrado** midiendo para responder a la revisión por consejo del movimiento de ejércitos, no buscándolo:
un asentamiento **nivel 1 a tope de población (300) come 30 trigo/tick** y una **Granja nivel 1 producía 15**.
No hacía falta simular para verlo. Lo que sí hizo falta simular fue decidir cuánto subirla.

Experimento A/B/C con la misma seed (15 facciones, 600 ticks), palanca `BATCH_TRIGO_X` nueva en
`scripts/run-batch-sim.ts` — mismo criterio que `BATCH_SIN_RECLUTAMIENTO`/`BATCH_SIN_ATAQUES`, muta el
catálogo que es el punto único de lectura (`produccionTrigoDeGranja`):

| | 1x | **2x** | 3x |
|---|---|---|---|
| Nutrición media (t600) | 28.98, **cayendo** | **100** | 100 |
| Nivel 2 | 7/13 | **11/13** | 11/13 |
| Nivel Facción máx | 3 | **5** | 5 |
| Tropas vivas (mundo entero) | **15** | 777 | 943 |

A 1x **las tropas se morían de hambre** (23→15). **3x es idéntico a 2x** en todo salvo en cuánto ejército
sostiene: a 2x la nutrición ya satura, así que triplicar es trigo sin destino. Adoptado **2x**: 15/22.5/30/45
→ **30/45/60/90**, `BALANCE_VERSION` 3→4, snapshot de regresión actualizado.

Dos límites que conviene no olvidar: **nivel 3 sigue en 0 incluso a 3x** (no es alimentario — issue nuevo
`issues/npc_no_alcanzan_nivel_3.md`, causa sospechada `artesanos = 0`), y subir la base **esquiva** el
problema de que las Granjas no escalan (`granjasActivasMedia = 2`, nivel interno 1 en las tres
configuraciones) en vez de resolverlo.

### 49. §E6.16: edificios construidos sobre celdas de calle — el guardián lo trajo el punto 48

Al aplicar el 2x, `perfilesTrazado.test.ts` empezó a fallar (*"gremial: vivienda pisa la celda de calle
-7,-5"*). Comprobado revirtiendo el cambio: **sin 2x pasa, con 2x falla**. Pero no es un bug nuevo — es
§E6.16, abierto y diagnosticado desde el 31 de agosto, que el usuario había encontrado a mano en el
laboratorio y que **la suite no alcanzaba**. Las ciudades más grandes lo hicieron reproducible.

Ese apartado decía textualmente que antes de arreglarlo hacía falta un caso que lo congelara. Apareció solo.

**Arreglo (salida 3 de las cuatro documentadas)**: `pisaCalleComprometida` en `engine/construction.ts`,
dentro del bucle de commit, **después** de cupo/tope/fondos y **antes** de pagar. Recalcula `redDeCalles` sobre
el orden REAL en que va a quedar la ciudad y descarta el candidato si cae sobre calle.

**Lo que hizo viable esa salida, y no estaba en el análisis original**: `anadirConectadas` (`trazado.ts:433`)
nunca siembra calle sobre celda ya ocupada, y el replay ocupa las celdas de cada edificio antes de sembrar sus
calles. Luego **ningún edificio puede quedar bajo una calle nacida después de él**, y validar cada candidato
contra su prefijo basta: no hace falta iterar a punto fijo ni deshacer pagos.

No se eligieron 1 ni 2 porque cambian qué se paga primero cuando no alcanza para todo (eso es balance), ni 4
porque sería rediseñar el crecimiento emergente.

**Coste medido: cero.** Batch idéntico antes/después en las 11 métricas de ciudad y simulación.
**Guardián**: `src/engine/__tests__/edificiosSobreCalle.test.ts` con la ciudad del laboratorio (seed 1, 200
ticks) que §E6.16 pedía, más seeds 60 y 200. Verificado a mano sobre 15 seeds antes de recortar.
786/786 tests, `tsc --noEmit` limpio en motor, cliente y lab.

### 50. El agua pasa de terreno caro a OBSTÁCULO (2026-09-02)

A petición del usuario: "los ejércitos y las caravanas no deben poder moverse sobre agua, el agua es un
obstáculo". **Revierte una decisión explícita de Fase 0.3**, cuyo comentario decía: *"Agua/cima no se prohíben
duro (romperían el pathfinding en cualquier mundo donde el camino más corto los roce) — se penalizan lo
bastante fuerte para que A* los evite salvo que no haya alternativa real"*. El síntoma que la tumbó salió
trazando un ejército tick a tick en el Paso 4: se le veía arrastrarse sobre el mar a 1/14 de su velocidad.

`Mapa.esTransitable` nuevo; A* salta las celdas de agua en vez de darles coste alto; `calcularRuta` devuelve
`Point[] | undefined`. **Lo importante es lo segundo**: antes caía a `[origen, destino]` cuando no encontraba
camino, y con el agua infranqueable esa recta de reserva sería justo una ruta por el mar.

Tres cosas que salieron al hacerlo, y ninguna era la que se buscaba:

**(a) La caja de búsqueda de A\* tenía margen fijo**, así que un rodeo más largo que el margen no se
encontraba. Daba igual mientras hubiera recta de reserva; ahora significaba rechazar un viaje posible. Se
añadió un reintento sobre el mapa entero cuando la caja ajustada falla — barato (a `ESPACIADO_MALLA`=45 un
mapa de 2000×2000 son ~1.900 celdas) y solo se paga tras fallar.

**(b) Dos llamadores no daban error de tipos y eran los peligrosos.** `Caravana.ruta` es OPCIONAL, así que
pasarle un `undefined` compilaba — y la caravana caería a la fórmula de línea recta de siempre
(`avanzarCaravanas`), es decir, cruzaría el mar en silencio. Rechazan explícitamente: la caravana de comercio
no sale (y la carga se queda en el almacén, comprobado ANTES de descontar), la de fundación tampoco.

**(c) Se podía FUNDAR SOBRE AGUA.** `evaluarViabilidadFundacion` solo excluía `'cima'`. Era inocuo mientras el
agua fuera cara; con ella infranqueable, un asentamiento en el mar queda incomunicado para siempre. Lo destapó
el fixture de pruebas, que llevaba fundando en (500,500) de la seed 42 —donde hay mar— sin que nadie lo
notara, porque nada dependía del terreno. `'agua'` se suma a `'cima'` como inhabitable, y el fixture se movió
a (400,400).

**Coste medido: cero.** Batch de 15 facciones × 600 ticks, misma seed: las 10 métricas idénticas antes y
después. Reglas en Doc 1.0b (mundo), 5.12.5 (ejércitos) y 3.10 (caravanas).

### 51. El laboratorio de batch llevaba un paso entero roto, y el type-check no lo veía

Al medir el punto 50 el batch daba **600 excepciones**. No era el agua: `avanzarEjercitos` recibía
`ejercitos` undefined porque el script construye su propio `EstadoSimulacion` y se le olvidó el campo al
añadir la entidad en el Paso 4. Peor: **atribuí el daño al cambio equivocado** hasta instrumentar el error, y
comparé contra una línea base anterior al paso que lo rompió.

Causa de fondo: `tsconfig.json` tiene `"include": ["src"]`, así que **`scripts/` nunca se type-checkea**. Es
exactamente el mismo agujero que ya mordió en Fase D2, cuando el mismo script quedó roto sin que nadie se
enterara. Dos veces es un patrón.

Arreglado con `scripts/tsconfig.json` + `npm run typecheck:scripts`, mismo patrón que `typecheck:lab`. No es
un script de usar y tirar: de sus corridas salen calibradas las constantes del juego.

`catalogo-edificios.ts` queda FUERA a propósito — arrastra 15 errores propios, incluida una referencia a
`'muralla'` (retirada en el Paso 5 de murallas, o sea que lleva roto desde entonces en ejecución, no solo en
tipos). Merece su propia pasada.

Y el contador de excepciones del batch ahora puede imprimir las 3 primeras con `BATCH_MOSTRAR_ERRORES=1`:
decía QUE algo fallaba, nunca QUÉ.

## Nota general

Todas las correcciones anteriores son de **diseño/balance**, no de sintaxis: el proyecto compiló sin errores de TypeScript en todo momento salvo en los pasos intermedios normales de refactor (añadir un campo a un tipo y luego actualizar todos los lugares que lo instancian), que se resolvieron sobre la marcha y no se listan aquí por ser rutinarios.
