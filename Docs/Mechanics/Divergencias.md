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

### 14. Los cargos no se liberan por inactividad · **Media**
- El canon 2.2 dice «Cargo se LIBERA tras 1 semana de inactividad del jugador que lo ocupa». No hay código: ningún cargo (de Facción ni local) se libera por inactividad. Un Gobernador desaparecido bloquea el resto de cargos de su plaza
  hasta que el Rey lo sustituya.
- **Falta decidir:** cómo se mide la inactividad (¿última orden, última conexión?) y si aplica también al Rey.

### 15. Vasallaje: faltan «conquista por un tercero» y las obligaciones de defensa · **Media**
- Canon 2.4: la ruptura 3 (conquista por un tercero → destrucción o anexión como vasalla del conquistador) no existe: conquistar plazas de una vasalla no toca su vasallaje, que solo termina por las otras vías. Y el señor «debe defender» a sus vasallos:
  hoy solo existe que la guerra a uno arrastra al otro; no hay obligación ni consecuencia por no defender.
- **Falta decidir:** si la conquista de una vasalla por un tercero debe cambiar algo, y qué es «defender».

### 16. Reputación: faltan eventos del canon · **Baja**
- Canon 2.7: penalizaciones por «no defender a un vasallo atacado» e «incumplir orden de mercado aceptada», y bonificación por «defender exitosamente a un vasallo». No existen (las órdenes ya no se aceptan: se toman en el mostrador).

### 17. La fusión no comprueba que el nombre esté libre · **Baja**
- `crearFaccion` rechaza un nombre ya usado; `proponerFusion`/`fusionar` no, así que C puede llamarse como una Facción existente.

### 18. Anexión: la marca de capital de la absorbida queda apuntando a una Facción que ya no existe · **Baja**
- `trasladarFaccion` no limpia `capitalDeFaccionId` de las plazas de B; la fusión sí lo hace. No afecta al cálculo (la capital designada exige la marca de la propia Facción), pero deja un id muerto en el estado.

### 19. El tributo no va a la capital ni se puede cobrar de varias plazas · **Baja**
- Cada tick paga una sola plaza de la vasalla (la de más stock) a una sola de la señora (la primera del estado), con tope de `cantidadPorMinuto`. El canon (2.4) solo dice «tributo pactado en recursos específicos»; no fija quién lo recibe.

## Consecuencias pendientes de medir

- **Comercio (2026-10-11):** la comisión deja de acuñarse al llevarse la carga al propio origen, la mala reputación paga un 50 % extra de su almacén, el reparto automático ya no se atasca con cascos vacíos ni destinos inalcanzables, y los trueques entre plazas de la misma Facción dejan de mover la reputación. Pueden cambiar el oro en circulación y el ritmo del comercio en el batch; medirlo (`Docs/Mecanicas a balancear.md`).

- **Gate de murallas por nivel (2026-10-09):** los bots comprometen el recinto de nivel 1 en cuanto pueden (`bots/cerebro/gobierno.ts`), así que ahora lo harán desde el nivel 2, antes que antes. El coste (≈2.100 de madera por
  una empalizada, según el diseño) y el upkeep pesan antes en la economía de una ciudad pequeña. Hay que medirlo con batch (`Docs/Mecanicas a balancear.md`).
- **Economía (2026-10-11):** quitar la penalización por distancia de las recetas (+45 % de rendimiento en toda la cadena metalúrgica, que daba ×0,688), contar los puestos y anclas a la mitad en el mantenimiento y pausar (en vez de desperdiciar) con el almacén lleno cambian el ritmo
  de la economía en el mismo sentido: más producción y menos coste. Hay que medirlo con batch (`Docs/Mecanicas a balancear.md`) antes de calibrar nada.
