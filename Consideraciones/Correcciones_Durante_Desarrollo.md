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

## Nota general

Todas las correcciones anteriores son de **diseño/balance**, no de sintaxis: el proyecto compiló sin errores de TypeScript en todo momento salvo en los pasos intermedios normales de refactor (añadir un campo a un tipo y luego actualizar todos los lugares que lo instancian), que se resolvieron sobre la marcha y no se listan aquí por ser rutinarios.
