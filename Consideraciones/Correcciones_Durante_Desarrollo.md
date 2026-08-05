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

## Nota general

Todas las correcciones anteriores son de **diseño/balance**, no de sintaxis: el proyecto compiló sin errores de TypeScript en todo momento salvo en los pasos intermedios normales de refactor (añadir un campo a un tipo y luego actualizar todos los lugares que lo instancian), que se resolvieron sobre la marcha y no se listan aquí por ser rutinarios.
