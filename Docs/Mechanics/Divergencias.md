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

### 8. Atacar a una columna: el atacante pierde salvo que aniquile al defensor · **Media**
- **Código:** `atacarColumna` (`engine/ejercitos.ts`) da la victoria al atacante solo si el defensor se queda **sin un soldado en pie**; si no, castiga al atacante (héroes heridos, pierde la mitad del carro y el defensor lo carga). El test `avanzarEjercitos.test.ts` («un ataque que no tumba al defensor es una derrota del atacante») lo fija así.
- **Comentario del código:** «si los dos siguen enteros no hay derrota que castigar». Código y comentario se contradicen.
- **Consecuencia:** `resolverCombate` hace perder al bando débil entre el 30 % y el 70 % de cada escuadra, así que casi nunca queda un bando a cero; atacar una columna casi siempre sale mal para quien ataca, aunque tenga más poder. Con un asedio no pasa: ahí gana quien tiene más poder.
- **A decidir:** si es intencional (hace del ataque una apuesta cara) se corrige el comentario; si no, el vencedor debería ser el que dice `resolverCombate` (o el que tiene más poder restante).

### 9. `iniciarAsedio` no tiene llamadores · **Baja**
- `engine/combate.ts` conserva `iniciarAsedio` (asedio de asentamiento a asentamiento, exige General) y sus tests, pero ningún comando ni el tick lo usa desde que los ejércitos asedian (`asediarConEjercito`). Es código muerto salvo que se quiera conservar como referencia; borrarlo se lleva sus tests.

## Consecuencias pendientes de medir

- **Gate de murallas por nivel (2026-10-09):** los bots comprometen el recinto de nivel 1 en cuanto pueden (`bots/cerebro/gobierno.ts`), así que ahora lo harán desde el nivel 2, antes que antes. El coste (≈2.100 de madera por
  una empalizada, según el diseño) y el upkeep pesan antes en la economía de una ciudad pequeña. Hay que medirlo con batch (`Docs/Mecanicas a balancear.md`).
