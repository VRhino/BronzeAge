# Divergencias entre el código, el canon y el diseño

Cosas que se encontraron **al documentar** `Docs/Mechanics/` y que no cuadran. Cada una dice qué hace el código, qué dice el canon o el diseño, y qué falta decidir. Cuando se resuelve una, se borra de aquí
(y se corrige donde corresponda).

Alta = impide algo del juego · Media = el juego va contra la intención escrita · Baja = comentario o documento desactualizado.

## Abiertas

- **El candado de las escuadras tras un plazo vencido** ([`Batalla_Unity_Ciclo.md`](Batalla_Unity_Ciclo.md) §15). El contrato pide comprobar los vencimientos justo antes de validar una acción que use una escuadra con candado; el código libera el candado en el siguiente tick, así que durante hasta un minuto de mundo reponer o guarnecer se rechazan por una batalla que ya no está activa.
- **`estado.batallas` crece sin límite** (misma sección). Las batallas cerradas no se purgan; falta decidir cuánto se conserva.

Las últimas (2026-10-12) se resolvieron así: el ticket de una columna sin Facción llevaba `faccionId: ''` (inválido en el contrato) y ahora `null`; cerrar el cliente en plena batalla ya no se rechaza con `batalla.bloqueado`; los eventos de una batalla llegan por el cursor a quien combate aunque no tenga plaza; se borra `faccionesEnBatalla` (sin usos); el rol `observador` («solo lectura», sin ninguna ruta que lo admitiera) se retiró; comerciar por la plaza (proponer, aceptar y rechazar trueques, órdenes de Mercado) es del Tesorero o del Gobernador, residente y dentro; la escala local y la del mapa son espacios separados (se borró `ESCALA`); se borró el diseño viejo del trazado y el asedio de asentamiento a asentamiento (`iniciarAsedio`, sin usos); «el señor debe defender» es solo reputación; defender a un vasallo también cuenta en el combate con números; un trueque aceptado no se cancela (se cumple o vence); y una escuadra reservada para una batalla no se repone ni entra o sale de la guarnición.

## Consecuencias pendientes de medir

- **Comercio (2026-10-11):** la comisión deja de acuñarse al llevarse la carga al propio origen, la mala reputación paga un 50 % extra de su almacén, el reparto automático ya no se atasca con cascos vacíos ni destinos inalcanzables, y los trueques entre plazas de la misma Facción dejan de mover la reputación. Pueden cambiar el oro en circulación y el ritmo del comercio en el batch; medirlo (`Docs/Mecanicas a balancear.md`).

- **Gate de murallas por nivel (2026-10-09):** los bots comprometen el recinto de nivel 1 en cuanto pueden (`bots/cerebro/gobierno.ts`), así que ahora lo harán desde el nivel 2, antes que antes. El coste (≈2.100 de madera por
  una empalizada, según el diseño) y el upkeep pesan antes en la economía de una ciudad pequeña. Hay que medirlo con batch (`Docs/Mecanicas a balancear.md`).
- **Economía (2026-10-11):** quitar la penalización por distancia de las recetas (+45 % de rendimiento en toda la cadena metalúrgica, que daba ×0,688), contar los puestos y anclas a la mitad en el mantenimiento y pausar (en vez de desperdiciar) con el almacén lleno cambian el ritmo
  de la economía en el mismo sentido: más producción y menos coste. Hay que medirlo con batch (`Docs/Mecanicas a balancear.md`) antes de calibrar nada.
