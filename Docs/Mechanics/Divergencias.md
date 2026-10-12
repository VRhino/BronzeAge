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

### 15. Vasallaje: faltan «conquista por un tercero» y las obligaciones de defensa · **Media**
- Hecho el 2026-10-12: la ruptura 3 (todas las plazas conquistadas por un tercero → vasalla del conquistador, §5.5) y la consecuencia de no defender (reputación, §5.9).
- **Queda:** el canon dice que el señor «debe defender» a sus vasallos; no es una obligación del motor, solo reputación.

### 16. Reputación: faltan eventos del canon · **Baja**
- Hecho el 2026-10-12: defender / no defender a un vasallo (+15 / −3, §5.9) y guerra tras romper una alianza (−15, ventana de 3 días, §5.4). Quitado del canon el «incumplir orden de mercado aceptada».
- **Queda:** el asedio con números (sin héroes de la señora) no aplica la regla de defensa, y no hay vía para cancelar un trueque antes de tiempo.

### 17. El canon describe los chokepoints, que ya no existen · **Media**
- `Docs/Game/1_Sistema_de_Mundo_y_Territorio.md` §1.5 sigue describiendo puertos de montaña con control por zona y peaje en oro. El generador los retiró en la v15 (2026-08-26): ni `generarChokepoints`, ni el tipo, ni el peaje existen en `src/`.
  Falta decidir si se borra la sección del canon (y las menciones de `Docs/Arquitectura/` y `Notas_revision.md`) o si la mecánica vuelve.

### 18. La regeneración del ganado no es «el doble» de la del mineral · **Baja**
- El canon (1.4) dice que el ganado regenera «al DOBLE de rápido» que los yacimientos minerales; el código usa 3 min frente a 10 (×3,3). Falta decidir cuál manda: la frase o las cifras.

### 19. Ningún bosque admite una sola Leñera · **Media**
- `LENERA_POR_BOSQUE` (`umbral2` 45, `umbral3` 65) se calibró en «tercios del rango de radio 30–80»; los bosques salen hoy con radio 45–120. Medido en las seeds 1/42/7: 0 bosques de capacidad 1, 48–50 de capacidad 2 y 120–122 de capacidad 3 (de 170).
  O las Leñeras por bosque deben ser mín 1 / máx 3 de verdad, y hay que recalibrar los umbrales al rango actual, o se acepta el 2–3 y se actualiza el comentario. Es balance de explotación: cambiarlo no altera el mapa.

### 20. Comentarios de calibración desactualizados en el generador · **Baja**
- `ELEVACION` documenta los percentiles «agua ~7 %, costa ~11 %, llano ~50 %, colina ~18 %, montaña ~11 %, cima ~3 %»; con el borde y el suavizado (v8–v14) el mundo libre da llano 42–47 %, colina 15–17 %, montaña 15–19 %, cima 0,6–1,8 %, costa 11–14 % y agua 4–10 %.
  Lo mismo con «los bosques tienen radio 30–80» (`mapa.ts`, `constants.ts`; hoy 45–120) y «60 piedras por seed» (hoy 240). Solo comentarios; los umbrales no se tocan sin subir `WORLDGEN_VERSION`.

### 21. El canon 1.1 describe un spawn de jugadores que no existe · **Baja**
- «Spawn de jugadores nuevos: posición aleatoria uniforme» no se corresponde con nada: no hay colocación inicial de jugadores; se funda con una Caravana de Fundación (canon 1.8) y quien no tiene plaza, con campamentos y alijos (1.9b). Falta reescribir esa línea.

## Consecuencias pendientes de medir

- **Comercio (2026-10-11):** la comisión deja de acuñarse al llevarse la carga al propio origen, la mala reputación paga un 50 % extra de su almacén, el reparto automático ya no se atasca con cascos vacíos ni destinos inalcanzables, y los trueques entre plazas de la misma Facción dejan de mover la reputación. Pueden cambiar el oro en circulación y el ritmo del comercio en el batch; medirlo (`Docs/Mecanicas a balancear.md`).

- **Gate de murallas por nivel (2026-10-09):** los bots comprometen el recinto de nivel 1 en cuanto pueden (`bots/cerebro/gobierno.ts`), así que ahora lo harán desde el nivel 2, antes que antes. El coste (≈2.100 de madera por
  una empalizada, según el diseño) y el upkeep pesan antes en la economía de una ciudad pequeña. Hay que medirlo con batch (`Docs/Mecanicas a balancear.md`).
- **Economía (2026-10-11):** quitar la penalización por distancia de las recetas (+45 % de rendimiento en toda la cadena metalúrgica, que daba ×0,688), contar los puestos y anclas a la mitad en el mantenimiento y pausar (en vez de desperdiciar) con el almacén lleno cambian el ritmo
  de la economía en el mismo sentido: más producción y menos coste. Hay que medirlo con batch (`Docs/Mecanicas a balancear.md`) antes de calibrar nada.
