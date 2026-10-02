# Rutas de caravana avanzadas — decisiones y plan

> **Estado (2026-10-02): IMPLEMENTADO** (Pasos 1-5 de §4, en una tanda; bitácora en §8). Salió de
> `Docs/Mecanicas a desarrollar.md` §3, ya retirado de allí; las reglas están en `Docs/Game/1` §1.6 (y toques en
> Doc 3.2, 3.6, 3.10). Queda pendiente enganchar el logro a `logistica_campana` cuando se escriba la Era IV
> (`Mecanicas a desarrollar` §20). Todas las cifras son placeholder.

## 0. El punto de partida

- **Pathfinder** (`world/rutas.ts`): A* sobre una malla de 45 u **anclada a la caja de cada búsqueda**. Coste
  por relieve (`Mapa.costeEnPunto`), agua infranqueable. **No mira bosques** (`ZonaBosque`, discos) **ni ríos**
  (`RioZona`, polilíneas que hoy se cruzan gratis).
- **Camino** (`engine/caminos.ts`): **uno por par** A–B, polilínea fija creada al aceptar un trueque. El bonus
  `factorCamino` (×0,5) se aplica a **todo el viaje** si el par tiene camino (`engine/trade.ts`), no por tramo.
  Ni se fusiona ni sabe cuánto se usa. Los ejércitos no lo aprovechan.
- **Comisión**: solo la de entrega en destino (`comisionDeEntrega`). No hay peaje de paso.
- **Inmunidad**: ninguna; bandidos (`engine/bandidos.ts`) e intercepción atacan en cualquier punto.

## 1. Decisiones cerradas con el usuario (2026-09-29)

1. **Bosque**: frena, no bloquea. Coste dentro del disco ×`(1 + 2·densidad)`; el camino lo rodea si el rodeo
   compensa.
2. **Río: infranqueable salvo por vados derivados** de la geometría del río, sin PRNG ni cambio de
   `WORLDGEN_VERSION`:
   - el tramo alto cerca del nacimiento es cruzable (arroyo);
   - un vado cada ~2 km a lo largo del cauce (1 u = 1 m desde el mundo regional, §7);
   - un asentamiento a orillas del río es también paso (el río se cruza por la ciudad).
3. **Red de caminos, no caminos por par.** El A* corre sobre un **grafo de navegación global precalculado**
   (§7); un camino es el conjunto de **aristas** de ese grafo que recorre alguna ruta. Solo se guardan las
   aristas usadas. Dos rutas que pisan la misma arista comparten tramo: la fusión sale sola.
4. **Peso = rutas vigentes.** Una *ruta* es un par origen–destino de caravana **comercial** (trueque automático
   o lanzada a mano; fundación y ejércitos no cuentan). Cada arista guarda qué rutas la usan **con su trazado
   actual**; el peso es su número. Si una ruta se desvía, la arista vieja pierde el punto y queda como sendero
   de peso 0 — **nunca se borra** (el camino es infraestructura permanente, Doc 1.6).
5. **Atracción**: el A* abarata las aristas con peso > 0, más cuanto más peso. Las rutas nuevas se desvían hacia
   los caminos principales. El trazado de un par **se recalcula en cada lanzamiento**, así que migra solo hacia
   los principales.
6. **Paso por una ciudad B**: si el trazado A→C cruza la zona de influencia de un asentamiento B **ajeno**
   (neutral o aliado), se fuerza por B (A→B→C) y la caravana **deja un % de cada recurso que lleva** (placeholder
   2 %) en el almacén de B. No se detiene.
7. **Inmunidad**: dentro de la zona de influencia de un asentamiento **neutral o aliado** (al dueño de la
   caravana) no puede ser atacada — ni bandidos ni intercepción.
8. **Zona enemiga**: se evita en el A* (coste alto), no fuerza paso ni cobra peaje, y no es refugio.
9. **Ejércitos: terreno sí, red no.** Bosque y río les afectan igual (mismo pathfinder). Van más rápido sobre
   un camino, pero **no suman peso** ni pagan peaje.
10. **Bonus de velocidad por tramo**: `factorCamino` se aplica al avanzar sobre una arista de la red, no al viaje
    entero del par.
11. **Logro `logistica_campana` (BA-006 D30)**: existe una arista **fuera de toda zona de influencia** usada por
    ≥ 10 rutas de ≥ 3 Facciones distintas.

## 2. Por defecto (sin decisión explícita — cambiar aquí si no convence)

- Asentamiento B **propio**: no se fuerza el paso ni hay peaje; su zona sí da inmunidad.
- Varios B en la ruta: se pasa por todos, en orden. Peaje sobre la carga que lleve en ese momento; vuelta vacía = 0.
- Peso y grosor en el mapa: tres escalones visuales (sendero 0-1, camino 2-5, calzada 6+). El bonus de
  velocidad es el mismo en los tres (no se escalona por peso en este pase).
- Atracción: coste de arista ×`max(0,5, 1 − 0,1·peso)`. Tiene que ser más débil que el rodeo de un bosque denso.

## 3. Representación en el motor

- **`RedCaminos`** (estado de partida, sustituye a `caminos: CaminoComercial[]`): mapa disperso
  `claveArista → { rutas: RutaId[] }`, con clave canónica de los dos nodos de la rejilla global.
  Peso, Facciones y grosor se **derivan**.
- **`Ruta`**: `{ id, origenId, destinoId, faccionId, aristas }` — su trazado vigente. Al recalcular, se quita de
  las aristas viejas y se añade a las nuevas.
- **`GrafoNavegacion`** (§7): derivado del mundo al cargar, cacheado en `Mapa`, **nunca persistido**. Por nodo:
  transitable. Por arista (8-vecinos): transitable (validada muestreando ≤ 30 m) y coste ya integrado a lo
  largo de la arista — relieve, **bosque** (el disco que atraviese) y **río** (arista que corta un cauce fuera
  de un vado = no transitable).
- **Vados**: derivados puros de `RioZona` (`worldgen/`), entrada del precálculo.
- **`calcularRuta`**: A* sobre el grafo (arrays tipados, índices enteros). Recibe opcionalmente la red
  (atracción) y las zonas (forzar paso por B, evitar enemigas). Sin ellas lo usan los ejércitos.
- ⚠ **"Nunca rejilla horneada"** (`Fase_0_1_Definicion.md`): el grafo es una caché derivada del campo continuo,
  como la malla auxiliar del plan de relieve; lo que se guarda es la red de caminos (aristas usadas), que es
  infraestructura de partida. Excepción consciente.

## 4. Plan

1. **Terreno**: `GrafoNavegacion` precalculado (§7) con bosque y río + vados en las aristas. Afecta a
   caravanas y ejércitos. El batch **no** saldrá bit-idéntico (cambian trazados); re-medir tiempos de viaje.
2. **Red de caminos**: `RedCaminos` + `Ruta`, registro/recálculo por lanzamiento, atracción, bonus por arista
   (caravanas y ejércitos), migración de los `CaminoComercial` guardados a rutas.
3. **Paso por ciudades**: forzado por B, peaje en especie, evitar zonas enemigas, inmunidad en bandidos e
   intercepción.
4. **Logro**: consulta pura `caminoCompartidoAbierto(red, zonas)` para `logistica_campana`; se engancha cuando
   el sistema de tecnología lo consuma.
5. **Proyección y contrato**: la proyección `caminos` cambia de forma (aristas con escalón, bajo niebla). Avisar
   al cliente y a Codex (`src/contratos/v1/`).

## 5. Invariantes

- Ninguna ruta cruza agua ni un río fuera de un vado o de un asentamiento ribereño.
- Una arista nunca desaparece de la red; su peso = rutas vigentes que la recorren.
- Una caravana dentro de la zona de un neutral/aliado no es objetivo de ataque.
- Determinista: mismo mundo + mismo estado → mismo trazado.

## 6. Puntos abiertos

- **Coordinación con el plan de relieve** (`Docs/Arquitectura/11_Plan_Worldgen_Relieve_Estrategico.md`, Fase 5
  drenaje): los vados deben derivarse de los ríos **después** de ese rediseño. Si el Paso 1 llega antes, se
  derivan de los actuales y se re-miden al cambiar.
- Calibración: separación de vados, factor de bosque, atracción, % de peaje.
- Separación del grafo: 150 m vs 250 m. El plan de relieve pide pasos de ≥ 3 intervalos de malla → con 150 m,
  pasos de ≥ 450 m. Decidir junto a la anchura de pasos del worldgen.
- Bosques de 65-195 m de radio caben en 1-2 nodos a 150 m: el coste del bosque se integra **por arista**, no
  por nodo, para no perderlos.

## 7. Escala: mundo regional de 32 × 24 km (medido 2026-09-29)

El worldgen de `codex/worldgen-relieve` pasa de 2000 × 2000 u a **32000 × 24000 u (1 u = 1 m)**, con mar,
lagos y hasta 40 ríos: 192× el área. Medido con seed 42 sobre ese mundo:

| | 2 km | 5 km | 10 km | 25 km |
|---|---|---|---|---|
| A* actual (malla 45 u por búsqueda) | 80-150 ms | 2-4 s | 6,5-13,5 s | no medido (minutos) |
| Grafo precalculado 150 m | — | ≤ 1 ms | ≤ 1 ms | 2-5 ms |
| Grafo precalculado 250 m | — | < 1 ms | < 1 ms | ~1 ms |

- Precálculo del grafo: ~5 s a 150 m (34 k nodos, ~200 KB), ~3 s a 250 m — una vez al cargar el mundo.
  `evaluarTerreno` cuesta ~2,4 µs.
- **Conclusión**: el diseño (§1) escala; la implementación de la malla por búsqueda **no**. Por eso §3 usa el
  grafo precalculado. Con él la red también escala: una ruta de 20 km son ~130 aristas; cientos de rutas son
  < 100 k entradas arista-ruta.
- La proyección al cliente no manda aristas sueltas: manda **tramos fusionados** (cadenas de aristas con el
  mismo escalón) como polilíneas suavizadas.
- **Fuera de esta ficha pero bloqueante al integrar el mundo regional**: las velocidades de caravana y ejército
  están calibradas para 2000 u. A 32 km los viajes duran ~10× más; el ritmo de viaje hay que recalibrarlo con
  el worldgen, no aquí.
- **Coordinación**: `world/rutas.ts` está modificado (sin commit) en `codex/worldgen-relieve` —validación de
  aristas a 5 m, conexión origen/destino a vecinas—. Esa lógica se mueve al precálculo del grafo. Hay que
  decidir quién toca `rutas.ts` primero para no reescribirlo dos veces.

## 8. Implementación (2026-10-02)

Prioridad sobre el trabajo de worldgen de Codex (decisión del usuario): Codex adapta su lógica después. Lo que toca
su rama sin commit (`world/rutas.ts`, `world/mapa.ts`): la validación de aristas y el enganche de origen/destino que
había allí ya están dentro del grafo.

**Dónde vive cada cosa**
- `world/grafoNavegacion.ts`: el grafo (§3), cacheado por mundo (`Mapa.mundo`). Separación
  `max(45, lado mayor / 160)` (`NAVEGACION`, `worldgen/config.ts`): 45 en el mapa de 2000, ~200 en el regional —
  entre los 150 y 250 de §6. Los vados (`hayVado`) viven aquí y no en `worldgen/`: son 3 líneas de aritmética
  sobre la longitud del cauce.
- `world/rutas.ts`: A* sobre el grafo con `OpcionesRuta` (`pesos` = atracción, `pasosRio` = ciudades ribereñas).
  Origen y destino se enganchan a los nodos 4×4 de alrededor a los que se llega en recta.
- `world/mapa.ts`: `costeEnPunto` incluye el bosque (`COSTE_MOVIMIENTO.bosquePorDensidad` = 2), con índice de
  bosques por celda. Lo leen el grafo y el avance por tick: el bosque frena al trazar y al moverse.
- `engine/redCaminos.ts` (sustituye a `engine/caminos.ts`): `RedCaminos = { aristas, rutas }` (las aristas que
  alguna vez se recorrieron, y las rutas vigentes con sus aristas); pesos y conjunto de aristas derivados y
  cacheados por objeto; `registrarRuta`, `podarRutas`, `tramosDeRed` (fusión por escalón para pintar),
  `caminoCompartidoAbierto` (logro), `trazarRutaComercial` (paso forzado + peajes) y `peajeDe`.
- `engine/movimiento.ts`: `factorCamino` en el tramo de la ruta que sea arista de la red (caravanas comerciales y
  ejércitos; las de fundación no).
- `engine/trade.ts`: el trueque automático y el lanzamiento manual trazan con `trazarRutaComercial` y registran la
  ruta; el peaje se cobra al alcanzar el `progreso` de cada ciudad (`Caravana.peajes`), solo a la ida, con evento
  `comercio.peaje_paso`. Aceptar un trueque ya no traza nada.
- Inmunidad: `enRefugio` (`engine/zones.ts`), en `avanzarAtaquesBandidos`, en la persecución de
  `resolverEncuentros` y en `interceptar` (comando).
- Estado: `red?` en `EstadoSimulacion` y `GameSessionState` (ausente = vacía). Proyección: `caminos` pasa a
  `CaminoProyectado { id, escalon, puntos }`, tramos fusionados con la niebla aplicada por arista. Los dos clientes
  solo leían `{ id, puntos }`, así que siguen pintando; el de administración ya pinta el grosor por escalón.

**Interpretaciones (cambiar aquí si no convencen)**
- **No existe "enemigo" en el modelo**: solo alianza/vasallaje, y el motor trata como enemiga a toda Facción ajena
  no aliada. Por eso:
  - inmunidad = dentro de una zona que **no es del atacante** (la propia, la neutral y la aliada son refugio;
    para los bandidos, cualquier zona);
  - "evitar zonas enemigas" (decisión 8) **no está implementado**: no hay estado de guerra del que colgarlo;
  - paso forzado y peaje con **toda** ciudad de otra Facción (aliada o neutral).
- **Rutas vigentes**: una ruta caduca a los 3 días sin lanzar (`RED_CAMINOS.caducidadMinutos`) o si desaparece uno
  de sus extremos. Sin esto, el peso contaría pares muertos para siempre.
- **Partidas guardadas**: los `caminos` (uno por par) se ignoran; la red se rehace con las próximas caravanas.
- Ejércitos y fundación también cruzan ríos por ciudades ribereñas (`pasosRio`).
- Ponytail: en el paso forzado, el tramo A→B no se vuelve a examinar; si roza otra zona ajena antes de llegar a B,
  no para en ella.

**Medido**
- Mapa de 2000 (seeds 1/42/7): grafo 2.025 nodos en 35-45 ms una vez por mundo; ruta de punta a punta 0,3 ms
  (antes 80-150 ms). Entre 78 y 114 aristas de río sin vado por mundo.
- **El bosque cubre ~54 % del mapa actual** (densidad media 0,79): con ×(1 + 2·densidad), los viajes tardan
  **~1,63×** de media antes del bonus de camino. Es el mayor cambio de balance de la tanda; afecta a los tiempos
  de las X de logros de comercio. Palanca: `COSTE_MOVIMIENTO.bosquePorDensidad`.
- Batch de humo (seed 7, 12 Facciones, 2 días): sin excepciones, la red crece (33 aristas).
- Regional (estimado, no medido con el worldgen de Codex): ~19.500 nodos a 200 m, precálculo de unos segundos
  la primera vez que se pide una ruta.

**Pendiente**
- Enganchar `caminoCompartidoAbierto` a `logistica_campana` (Era IV).
- BronzeAgeClient: pintar el grosor por `escalon` (hoy dibuja todos los tramos igual).
- Calibrar con el batch por semanas: bosque, atracción, peaje, caducidad, vados.
