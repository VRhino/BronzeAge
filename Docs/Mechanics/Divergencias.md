# Divergencias entre el código, el canon y el diseño

Cosas que se encontraron **al documentar** `Docs/Mechanics/` y que no cuadran. Cada una dice qué hace el código, qué dice el canon o el diseño, y qué falta decidir. Cuando se resuelve una, se borra de aquí
(y se corrige donde corresponda).

Alta = impide algo del juego · Media = el juego va contra la intención escrita · Baja = comentario o documento desactualizado.

## Abiertas

### 1. El candado de batalla de una escuadra solo se mira al abrir otra batalla · **Media**
- `Escuadron.reservaBatalla` impide que la misma escuadra entre en otra batalla, pero ni `reclutarTropa` (reponer), ni `asignarGuarnicion`/`retirarGuarnicion`, ni `guardarLoadout`/`borrarLoadout` la comprueban: durante una batalla de Unity se puede reponer, guarnecer o sacar del loadout una escuadra reservada.
- Falta decidir si esas acciones deben rechazarse mientras haya candado (el modelo de datos compartido dice que el candado presente significa «en una batalla»).

## Consecuencias pendientes de medir

- **Comercio (2026-10-11):** la comisión deja de acuñarse al llevarse la carga al propio origen, la mala reputación paga un 50 % extra de su almacén, el reparto automático ya no se atasca con cascos vacíos ni destinos inalcanzables, y los trueques entre plazas de la misma Facción dejan de mover la reputación. Pueden cambiar el oro en circulación y el ritmo del comercio en el batch; medirlo (`Docs/Mecanicas a balancear.md`).

- **Gate de murallas por nivel (2026-10-09):** los bots comprometen el recinto de nivel 1 en cuanto pueden (`bots/cerebro/gobierno.ts`), así que ahora lo harán desde el nivel 2, antes que antes. El coste (≈2.100 de madera por
  una empalizada, según el diseño) y el upkeep pesan antes en la economía de una ciudad pequeña. Hay que medirlo con batch (`Docs/Mecanicas a balancear.md`).
- **Economía (2026-10-11):** quitar la penalización por distancia de las recetas (+45 % de rendimiento en toda la cadena metalúrgica, que daba ×0,688), contar los puestos y anclas a la mitad en el mantenimiento y pausar (en vez de desperdiciar) con el almacén lleno cambian el ritmo
  de la economía en el mismo sentido: más producción y menos coste. Hay que medirlo con batch (`Docs/Mecanicas a balancear.md`) antes de calibrar nada.
