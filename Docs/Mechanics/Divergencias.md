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


### 14. Una caravana lanzada a mano puede ir a cualquier sitio, incluido su propio origen, y acuña oro · **Alta**
- `prepararCaravanaManual` no mira el destino: ni que sea distinto del origen, ni que sea propio o aliado. El canon (3.13.3) dice «mover mercancía a plaza propia o aliada».
- Enviarla **al propio origen** (probado) vuelve en 2 ticks y la entrega paga la comisión de misma Facción (`COMISION.tasaMismaFaccion`) **acuñada** (3.3): 400 de madera dejan ≈ 10 de oro por viaje, sin coste. Entre
  dos plazas propias pasa lo mismo con el bonus por distancia. Y a una plaza enemiga se le puede regalar mercancía por la ruta que cruce.
- **Falta decidir:** qué destinos vale (¿solo plazas propias y aliadas, y nunca el propio origen?) y si la comisión de una entrega manual sin trueque debe acuñarse.

### 15. La comisión del trueque se acuña y la reputación baja no la «paga» nadie · **Media**
- `comisionDeEntrega` suma el oro al destino **sin quitarlo de nadie** (es una fuente de oro, 3.1), y `factorComisionPorReputacion` (×1,5 con reputación ≤ −40) lo multiplica: el destino cobra más y a la Facción
  poco fiable no le cuesta nada. El canon 2.7 dice que tratar con ella «sale más caro para ELLA». Además el factor se aplica también a las entregas entre plazas de la **misma** Facción, y el mostrador (3.5) no lo
  lleva aunque el canon dice que la comisión se modula «por reputación de Facción».
- **Falta decidir:** si la comisión debe salir del origen (o de la carga), o si el castigo es otro; y si el mostrador también lo lleva.

### 16. El reparto automático se atasca con un casco vacío primero o con un trueque inalcanzable · **Media**
- `asignarCaravanasATrueque` usa `disponibles[idx]`: un casco sin carros con animal (capacidad 0) que esté **primero** en el array bloquea a todas las demás caravanas de ese origen, porque la carga sale vacía y no
  se avanza `idx` (probado). Solo se evita reservándolo (`reservarCaravana`).
- `idx++` ocurre **antes** de trazar la ruta: un lado sin camino por tierra gasta una caravana por tick y, con una sola, impide que salgan los demás lados de ese origen mientras puntúe más.
- El canon (3.2) solo dice que, sin caravana disponible, el envío espera.
- **Falta decidir:** saltar las caravanas sin capacidad al repartir, y no gastar la caravana cuando no hay ruta.

### 17. «Carga Ampliada» no alcanza a las caravanas adjuntas ni aparcadas · **Baja**
- `factorCapacidadCaravana` (×1,5) solo se aplica al cargar para viajar (reparto y `prepararCaravana`). `capacidadCargaDe` (ejércitos), `cargarCaravanaAdjunta` y `moverCargaCarroAparcada` usan la capacidad sin
  política. El canon (3.12) dice «×1.5 la capacidad de carga de las caravanas propias».

### 18. En un trueque entre plazas de la misma Facción la reputación se cuenta dos veces · **Baja**
- `aplicarEntregaATrueque` y el vencimiento recorren `[Facción de A, Facción de B]` sin deduplicar: cumplir uno consigo mismo da `+bonusTruequeCumplido` dos veces y vencer lo penaliza dos veces. El canon
  (3.2/2.7) dice «a ambas Facciones» pensando en dos.

### 19. Un trueque vencido puede resucitar como cumplido · **Baja**
- `aplicarEntregaATrueque` no mira el `estado`: una caravana que ya salía cuando el acuerdo venció sigue abonando líneas al llegar y, si salda los dos lados, lo pasa de `expirado` a `cumplido` (con su
  recompensa de reputación y experiencia), aunque ya se hubiera penalizado el vencimiento.

### 20. `mercado.compra` es código muerto · **Baja**
- `comerciarEnPlaza` (motor) construye un evento `mercado.compra` que el comando descarta (emite solo `mercado.comercio_en_plaza`). Sin consumidores.

## Consecuencias pendientes de medir

- **Gate de murallas por nivel (2026-10-09):** los bots comprometen el recinto de nivel 1 en cuanto pueden (`bots/cerebro/gobierno.ts`), así que ahora lo harán desde el nivel 2, antes que antes. El coste (≈2.100 de madera por
  una empalizada, según el diseño) y el upkeep pesan antes en la economía de una ciudad pequeña. Hay que medirlo con batch (`Docs/Mecanicas a balancear.md`).
- **Economía (2026-10-11):** quitar la penalización por distancia de las recetas (+45 % de rendimiento en toda la cadena metalúrgica, que daba ×0,688), contar los puestos y anclas a la mitad en el mantenimiento y pausar (en vez de desperdiciar) con el almacén lleno cambian el ritmo
  de la economía en el mismo sentido: más producción y menos coste. Hay que medirlo con batch (`Docs/Mecanicas a balancear.md`) antes de calibrar nada.
