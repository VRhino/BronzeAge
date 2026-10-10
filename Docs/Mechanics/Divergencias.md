# Divergencias entre el código, el canon y el diseño

Cosas que se encontraron **al documentar** `Docs/Mechanics/` y que no cuadran. Cada una dice qué hace el código, qué dice el canon o el diseño, y qué falta decidir. Cuando se resuelve una, se borra de aquí
(y se corrige donde corresponda).

Alta = impide algo del juego · Media = el juego va contra la intención escrita · Baja = comentario o documento desactualizado.

## Abiertas

### 5. `ESCALA.unidadesLocalesPorUnidadMapa` no se usa · **Baja**
- Está declarada (40) y documentada como «la equivalencia que faltaba declarar», pero ningún cálculo del motor la aplica: `radioUrbanoDe` devuelve `radioPotencial` (unidades de mapa, 30–180) **tal cual como unidades
  locales**. La ciudad mide, pues, 10–60 celdas de radio, y la «décima parte de la provincia» del comentario no se cumple en el motor.

### 7. `Consideraciones/Vista_Asentamiento_Trazado_Urbano.md` está desactualizado · **Baja**
- Su propio encabezado admite que las secciones numeradas «siguen describiendo el modelo de ARISTAS y están pendientes de reescritura». La descripción vigente es [`Trazado_Urbano.md`](Trazado_Urbano.md). Cuando se
  complete la migración de este contenido, el documento de diseño se borra (decisión del autor).

### 8. `iniciarAsedio` no tiene llamadores · **Baja**
- `engine/combate.ts` conserva `iniciarAsedio` (asedio de asentamiento a asentamiento, exige General) y sus tests, pero ningún comando ni el tick lo usa desde que los ejércitos asedian (`asediarConEjercito`). Es código muerto salvo que se quiera conservar como referencia; borrarlo se lleva sus tests.

### 9. Los edificios gratuitos pagan mantenimiento · **Media**
- El código cuenta como edificio (peso 1) **todo** `Edificio` en estado `activo`, también los que nacen gratis: el Centro Urbano, los **puestos del Mercado** (5 + 7 + 4 = 16 piezas al nivel 3) y las **anclas** (plaza, pozo, parque, plaza de armas, patio de gremios).
  Cada pieza suma `1/edificiosReferencia` (0,02) al factor del coste de madera, piedra y oro: los 16 puestos de un Mercado de nivel 3 suman 0,32 al factor de edificios (que vale 1 + edificios/50) solo por existir.
- El canon (4.5) dice «escala con los edificios activos, cada Vivienda cuenta la mitad» y no menciona estos. Los puestos «son parte del Mercado que ya se pagó» y las anclas son superficie, no función.
- **Falta decidir:** ¿pagan mantenimiento los puestos y las anclas, o se excluyen del recuento (como la Vivienda tiene su peso)? Ver `Economia_Construccion.md` §10.1.

### 10. Los insumos de mina ignoran la distancia real en las líneas de producción · **Baja**
- Canon 4.2: la fuente de un insumo crudo (cobre, estaño…) es «el extractor correspondiente (mina de cobre…)» y la distancia al más cercano penaliza la producción.
- Código: solo cuentan fuentes **del mismo ámbito** que el taller; los talleres son de la ciudad y las minas del mapa, así que cobre, estaño y hierro caen siempre en `distanciaEstandarSinFuente` (90) → factor fijo **0,688** para esas recetas, con la mina pegada o a 100 de distancia.
- **Falta decidir:** si es lo que se quiere (un castigo fijo a toda la cadena metalúrgica) o si la mina debe medirse con una distancia propia (p. ej. a la posición del asentamiento).

### 11. Un almacén lleno desperdicia en vez de pausar · **Baja**
- `pausadoPorAlmacenLleno` es solo una bandera: la granja y los extractores pierden lo que no cabe (el extractor además **drena el nodo**), y una receta **consume todos sus insumos** aunque el producto no quepa.
- El canon 4.3 dice que el sobrante se pierde (coherente para extractores y granjas) pero no dice nada de los insumos de una receta; el nombre de la bandera sugiere que debería parar.
- **Falta decidir:** si una receta con el producto lleno debe parar (y no gastar insumos) y si un extractor con el almacén lleno debe dejar el nodo intacto.

### 12. La Gran Fundición no cuenta como mano de obra de artesanos · **Baja**
- `EDIFICIOS_TRANSFORMACION` es fundición, curtiduría, armería y carpintería: la **Gran Fundición** (12 artesanos en el canon) no suma a `trabajadoresTransformacion`, ni dispara la aparición de los primeros artesanos, ni resta del pool de reclutamiento de artesanos.
  Produce al ritmo global de los demás talleres.
- **Falta decidir:** si debe contar como los otros talleres.

### 13. `requiereEdificio` de una mejora mira solo el primer edificio · **Baja**
- Para mejorar (p. ej. Armería 2 exige Fundición 2) el código mira el **primer** edificio activo de ese tipo en el array; para las recetas (`requiereEdificio`) mira cualquiera (`some`). Con dos Fundiciones de niveles distintos el resultado depende del orden en que se construyeron.
- **Falta decidir:** unificar en «cualquiera» (lo natural) o dejarlo.

### 14. La Maravilla no comprueba la Era · **Baja**
- Canon 4.2.1: exige nivel de asentamiento 5 **y** el servidor en la Era V. El código solo exige `requisitoNivelAsentamientoConstruccion: 5`; hoy es imposible llegar porque `ASCENSO_ASENTAMIENTO.nivelTechoProvisional` frena en el nivel 4, pero al quitar el techo la Maravilla quedaría abierta sin la Era.

### 15. Reembolso al quitar de la cola un edificio dañado · **Baja**
- Quitar un edificio `danado` de la cola devuelve el coste **completo** del catálogo más las mejoras que tenía (decidido el 2026-09-27, canon 4.2), aunque reconstruirlo solo costaría la mitad y lo pagó el dueño anterior. Es un camino de «ingreso» para quien conquista una plaza.
- **Falta decidir:** confirmar que es intencional.

### 16. Cifras de balance que viven en el motor, no en `constants.ts` · **Baja**
- `MAXIMO_TRANSFORMACION_POR_NIVEL`, `NIVEL_GATE_ANCLAS`, `EDIFICIOS_UNICOS`, `EDIFICIOS_CON_NIVELES`, `RECURSO_PROPIO` y el recorte 0–999 de la reserva manual no están en `constants.ts`: el panel de balance no los edita y el test de sincronización de `Docs/Mechanics` no vigila sus copias.
  Habría que moverlas a `constants.ts`.

### 17. Canon 4 con textos desactualizados · **Baja**
- 4.2.1, tabla de extracción: la columna «Tiempo» (Cantera 5 min, Leñera 3, Granja 6…) contradice 4.2 y el código (4 h, 1 h, 2 h).
- 4.2.1 (Armería, «Arma de Madera») dice que la producción de recetas **no** respeta la reserva de mantenimiento; 4.2 y el código dicen que **sí** (el comentario de `RECETA_ARMA_MADERA` en `constants.ts` también está viejo).
- 4.2 llama `horizonteTicksMantenimiento` / `horizonteTicksComida` a lo que el código llama `horizonteMinutosMantenimiento` / `horizonteMinutosComida`.

## Consecuencias pendientes de medir

- **Gate de murallas por nivel (2026-10-09):** los bots comprometen el recinto de nivel 1 en cuanto pueden (`bots/cerebro/gobierno.ts`), así que ahora lo harán desde el nivel 2, antes que antes. El coste (≈2.100 de madera por
  una empalizada, según el diseño) y el upkeep pesan antes en la economía de una ciudad pequeña. Hay que medirlo con batch (`Docs/Mecanicas a balancear.md`).
