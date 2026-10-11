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

### 9. Las Eras IV y V no están en el código · **Media**
- El canon (Doc 6.2, 6.5, 4.5) define **cinco** Eras, con tarifa de adopción para las cinco y tecnologías de las IV y V. El código solo tiene `reinos_palaciales`, `crisis_adaptacion` y `polis_imperios` en `ERAS`, y `TARIFA_ADOPCION` solo cubre esas tres.
  Consecuencia directa: la Era V que exige la Maravilla (`ordenEraMinimo: 5`) no existe nunca, y `ASCENSO_ASENTAMIENTO.nivelTechoProvisional` frena en el nivel 4. Está en curso (`Consideraciones/BA-006_Revision_Tecnologia_Eras.md`).

### 10. Las vías de comercio y de conquista para conseguir tecnología no están implementadas · **Media**
- Canon 6.1: «Comercio: pagar a una Facción que ya la tiene adoptada» y «Conquista: al conquistar un asentamiento que ya reclutaba con una tecnología, esa tecnología le aparece al conquistador».
- Código: la única forma de que una tecnología aparezca sin el hito es `hacerAparecer`, que solo llaman la compra a un Aeda y el final de una épica. Ni el comercio ni `aplicarConquista` tocan las tecnologías.
- **Falta decidir:** si se implementan ahora o se retiran del canon hasta que haya diseño de cómo se paga/transfiere.

### 11. Una épica o una compra no registran descubridora · **Media**
- `primeros[id]` (la Facción descubridora) solo se escribe cuando una Facción cumple el **hito**. Si una tecnología aparece por la épica de un Aeda residente **antes** de que nadie cumpla el hito, `primeros` queda vacío: los Aedas itinerantes no la «conocen» (nunca se revela ni se vende), no se canta el descubrimiento
  y los campamentos de mercenarios no la desbloquean (`tecnologiasDelCampamento` exige `primeros`). (La compra a un Aeda exige que ya exista `primeros`, así que ahí no pasa.)
- El canon 6.3 dice que la primera Facción en conseguirla queda como descubridora, sin distinguir la vía.
- **Falta decidir:** si completar una épica cuenta como descubrir (registrar `primeros`), o solo el hito.

### 12. «Facciones humanas vivas» cuenta todas las Facciones con asentamientos · **Baja**
- Canon 6.5b y el comentario de `tecnologiasDelCampamento`: el 50 % se mide sobre las Facciones humanas vivas, **las que no gobierna la IA**. Desde que los bots juegan desde fuera, con comandos, el motor no distingue Facciones de bots de las humanas: `humanasVivas` son **todas** las que conservan algún asentamiento.
- **Falta decidir:** si se reformula el canon (todas las Facciones vivas) o se necesita una marca de «humana» en la Facción.

### 13. Nombre de constante desfasado en el canon · **Baja**
- Doc 6.4 llama `AEDAS.retrasoConocimientoHoras` a lo que el código llama `AEDAS.retrasoConocimientoMinutos` (24·60).

## Consecuencias pendientes de medir

- **Gate de murallas por nivel (2026-10-09):** los bots comprometen el recinto de nivel 1 en cuanto pueden (`bots/cerebro/gobierno.ts`), así que ahora lo harán desde el nivel 2, antes que antes. El coste (≈2.100 de madera por
  una empalizada, según el diseño) y el upkeep pesan antes en la economía de una ciudad pequeña. Hay que medirlo con batch (`Docs/Mecanicas a balancear.md`).
- **Economía (2026-10-11):** quitar la penalización por distancia de las recetas (+45 % de rendimiento en toda la cadena metalúrgica, que daba ×0,688), contar los puestos y anclas a la mitad en el mantenimiento y pausar (en vez de desperdiciar) con el almacén lleno cambian el ritmo
  de la economía en el mismo sentido: más producción y menos coste. Hay que medirlo con batch (`Docs/Mecanicas a balancear.md`) antes de calibrar nada.
