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

## Consecuencias pendientes de medir

- **Gate de murallas por nivel (2026-10-09):** los bots comprometen el recinto de nivel 1 en cuanto pueden (`bots/cerebro/gobierno.ts`), así que ahora lo harán desde el nivel 2, antes que antes. El coste (≈2.100 de madera por
  una empalizada, según el diseño) y el upkeep pesan antes en la economía de una ciudad pequeña. Hay que medirlo con batch (`Docs/Mecanicas a balancear.md`).
- **Economía (2026-10-11):** quitar la penalización por distancia de las recetas (+45 % de rendimiento en toda la cadena metalúrgica, que daba ×0,688), contar los puestos y anclas a la mitad en el mantenimiento y pausar (en vez de desperdiciar) con el almacén lleno cambian el ritmo
  de la economía en el mismo sentido: más producción y menos coste. Hay que medirlo con batch (`Docs/Mecanicas a balancear.md`) antes de calibrar nada.
