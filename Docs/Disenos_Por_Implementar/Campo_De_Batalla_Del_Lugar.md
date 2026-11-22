# Campo de batalla del lugar

> **Diseño acordado el 2026-11-22 con el autor. Sin implementar** (vive en `Docs/Disenos_Por_Implementar/` hasta que se implemente; ver su README). Entrada 78 de `Docs/Mecanicas a desarrollar.md`. Las cifras son placeholders por calibrar; no hay bloque `parametros` hasta implementarlo. Contrastado contra el código del motor (`BronzeAgeFase0`, commit `e5be5790`) y de Conquest (`Conquest_prototype`, rama `main` del 2026-11-21), no contra los documentos.

## 1. Qué es

El mapa interior de una batalla deja de ser siempre el mismo y sale del sitio del mundo donde se abrió la batalla. Si la batalla cae junto a una playa, la playa está en el campo. Si cae junto a un río, el río corre por el campo, y si cae en un vado, el vado es un cuello de botella. El bosque, las colinas y los caminos de alrededor también entran. El relieve nunca es liso: se toma el del mundo y se exagera.

La regla de fondo ya está escrita en el contrato del motor (`proto/bronzeage/v1/battle.proto:158`): «El terreno de la batalla sale del artefacto de mundo: Unity no lo inventa». Conquest la cumple solo a medias: lee el artefacto para dibujar el mapa de mundo, pero en la batalla no.

## 2. Lo que hay hoy

### 2.1 Lo que entrega el motor

| Pieza | Dónde | Qué aporta | Qué le falta |
|---|---|---|---|
| `BattleTicket.mapa_referencia` | `battle.proto:158-162, 207-210`; se rellena en `crates/nucleo/src/proyeccion/batalla.rs:179-181` | `mapa_id` (hash del artefacto) y `centro` (el `Batalla.punto`) en toda batalla que no es asedio | Caminos, de dónde viene cada bando, hash del relieve |
| `Batalla.punto` | `crates/combate/src/ciclo.rs:367-388` | En campo abierto es la posición del **defensor** en `t` (`batallas.rs:404`), no el punto medio | Nada |
| `SettlementBattleSnapshot` | `battle.proto:181-187` | Edificios y celdas de muralla del asedio | El punto de la plaza: el asedio no lleva `centro` |
| Artefacto de mundo | `world.proto:105-131`, `crates/mundo/src/lib.rs` | Rejillas de 4 u (48 m): `altura`, `terreno` (agua, costa, llano, colina, montaña, cima), `densidad_bosque`, `humedad`, `transitable`, `coste`; ríos con sus vados como punto (`RADIO_VADO` 0,75 u, `lib.rs:93`) | Nada para esto |
| Paquete de relieve | `world.proto:157-177` | Altura cada 0,5 u (6 m) y bit de agua (mar y lagos) por muestra | Nada |
| Red de caminos | `entities.proto:1255-1270` | Tramos con su escalón (sendero, camino, calzada) | Es dinámica y va por la vista con niebla: el servidor de batalla no la tiene |
| Plan de cada columna | `Ejercito.plan`, `crates/nucleo/src/estado.rs:540` | De dónde venía cada bando | No se manda |
| Azar determinista | `base::Rng` (`crates/base/src/lib.rs:242`), `trazado::azar::hash` | El patrón de semilla por entidad ya existe (`urbanismo.rs:203`) | No hay semilla de batalla; `battle_id` es un UUID determinista |

Escala: 1 u = 12 m (`factor_unity`). Un río dibujado mide unos 18 m de ancho y el paso de un vado unos 18 m. A escala 1:1 eso ya es un cuello de botella para una escuadra de 20 soldados.

### 2.2 Lo que hace Conquest

| Pieza | Dónde | Estado |
|---|---|---|
| Escena de batalla | `BattleScene.unity` | Una sola. El suelo es un Plane de 1500×1500 m, plano, con MeshCollider y NavMesh horneado en el editor (`NavMesh-Ground.asset`). Encima, piezas Synty puestas a mano (murallas, calles) |
| Lectura del ticket | `Net/BatallaRealDeTicket.cs:16, 36-81` | Lee bandos, reglas y contexto. **Ignora `mapa`**: su comentario dice «el mapa es el de siempre» |
| Spawns | `DOTSWorld.unity` | Cuatro a mano, a unos 200 m entre bandos (tres atacantes y un defensor; los defensores 5 y 6 caen al de reserva) |
| Captura y suministro | `Map/CapturePointSetup.cs`, `Map/SupplyPointSetup.cs` | 3 y 3, puestos a mano; leen su posición del transform en `Start()` |
| Modificadores de terreno | `Map/BattleTerrain.cs`, `UnitStatsUtility.cs:116-136`, `BattleBootstrap.System.cs:81-88` | Ya existe `TerrainModifier` por tipo de escuadra (bosque, montaña, nieve), pero es **uno por batalla** y sale del `MapDataSO`, que siempre es `DefaultMap` (llano). Nunca se aplica nada |
| Altura del suelo | `FormationPositionCalculator.calculateTerraindHeight` | Ya muestrea `Terrain.activeTerrain`. Sin Terrain no hace nada |
| Lectura del mundo | `Mundo/Relieve.cs` (`AlturaEn`, `AguaEn`, `RasterDeMundo`), `Mapa/RiosYNiebla.cs` (`CaucesDelMapa`: `ACauce`, `AUnVado`, `SomeroEn`) | C# puro, sin referencias a Unity (`noEngineReferences`). Lo puede usar también el servidor de batalla |
| Vegetación | `Mundo/ReglasDeVegetacion.cs`, `MosaicoDeVegetacion.cs` | Determinista desde bosque, humedad, fertilidad y pendiente. Sirve para decidir dónde, no la densidad de batalla |
| Red | `Net/EspejoClienteSystem.cs:63-74`, `Net/MandoSystem.cs:105-128` | Servidor autoritativo. Cada cliente carga BattleScene por su NavMesh y predice su héroe con él: **los seis jugadores necesitan la misma malla que el servidor** |
| NavMesh en marcha | ninguna llamada a `BuildNavMesh`/`UpdateNavMesh` | El paquete `com.unity.ai.navigation` 2.0.14 está instalado; se puede hornear en marcha |

Los clientes ya descargan el artefacto y el relieve enteros y los guardan en disco (`Mundo/MundoEnDisco.cs:45, 57`). El proceso servidor de batalla no: solo recibe el ticket del lanzador y no sabe la dirección del motor.

## 3. Quién aporta qué

**El motor aporta solo lo que Conquest no puede deducir.** Todo lo que sale del artefacto (relieve, agua, ríos, vados, bosque, clase de terreno) lo lee Conquest por su cuenta, con el `mapa_id` y el `centro` que ya llegan.

Cambios en el motor (pequeños):

1. `BattleMapReference` gana:
   - `relieve_id`: el hash del paquete de relieve (hoy solo va en `InfoMundo`).
   - `caminos`: los tramos de la red de caminos que cruzan el círculo de radio `radio_lectura` alrededor del centro, con su escalón. Son pocos puntos.
   - `procedencia_atacante` y `procedencia_defensor`: un `Punto` por bando, a `distancia_procedencia` hacia atrás sobre su plan (o la plaza, el campamento o el pueblo de donde sale). Si un bando estaba quieto, su posición.
2. El lanzador pasa al proceso de batalla la URL base del motor (`LANZADOR_ARGUMENTOS`, `crates/lanzador/src/main.rs`), y el proceso descarga y guarda el artefacto y el relieve por hash, como ya hace el cliente.
3. Más adelante, para asedios: `SettlementBattleSnapshot` gana `centro`, para montar el terreno alrededor de la plaza (§9).

Conquest aporta todo lo demás: la lectura del lugar, la generación del campo, su montaje en Unity y sus efectos en el combate.

## 4. Las tres piezas en Conquest

Siguen el modelo híbrido: la generación es datos puros; Unity solo monta lo que sale; la lógica de combate lee una rejilla ECS.

1. **Lectura del lugar** (C# puro, junto a `Relieve.cs`). Con el artefacto, el relieve y el ticket, muestrea un cuadrado de `lado_lectura` centrado en `centro`: altura, bit de agua, clase de terreno, bosque, humedad, ríos y vados recortados, caminos del ticket y procedencias. No hay rotación: el norte del campo es el norte del mundo, y el minimapa de preparación casa con el mapa de mundo.
2. **Generación del campo** (C# puro, aritmética entera). Saca:
   - la rejilla de alturas (u16, paso `paso_alturas`);
   - la rejilla de zonas (u8 por celda: llano, colina, bosque claro, bosque cerrado, arena, agua somera, agua honda, camino, acantilado);
   - la lista de objetos (árboles, rocas, matas) con posición cuantizada;
   - los spawns de cada bando y los puntos de captura y suministro;
   - una huella (FNV) de todo lo anterior.
3. **Montaje** (Unity). Crea un Unity Terrain con esas alturas (así `calculateTerraindHeight` empieza a funcionar), los planos de agua, los árboles como instancias del Terrain y colliders solo en los troncos del área jugable, coloca los prefabs de spawn, captura y suministro antes de su `Start()`, y hornea el NavMesh con un área nueva, agua somera, con su coste. La rejilla de zonas pasa al mundo ECS como un blob singleton.

BattleScene se queda con lo común (luz, gestores, subescena). El suelo plano y las piezas del castillo salen de ella; las piezas de muralla siguen sirviendo para los asedios.

## 5. Tipos de campo

El campo se compone por capas (relieve, agua, bosque, caminos), así que un vado con bosque en una orilla sale solo. El **tipo** decide únicamente dónde van los spawns y los objetivos. Se elige el primero que se cumpla:

| Orden | Tipo | Se da si en el cuadrado de lectura | Forma del campo |
|---|---|---|---|
| 1 | Vado | Hay un vado dentro del cuadrado de lectura | El río cruza el campo de lado a lado y el vado es el único paso (§6.4). Los bandos, siempre en orillas opuestas |
| 2 | Paso | La mitad o más de las celdas son montaña o cima, o la pendiente media pasa de `pendiente_paso` | Un valle entre laderas, con acantilados infranqueables en los bordes: cuello de botella sin agua |
| 3 | Costa | Hay mar a menos de `radio_costa_captura` | Playa de arena, franja somera, el mar como flanco cerrado |
| 4 | Orilla de lago | Lo mismo, con agua de lago | Como costa, sin oleaje y con la orilla más estrecha |
| 5 | Ribera | Un río cruza el cuadrado sin vado | El río como flanco: los dos bandos en la misma orilla |
| 6 | Colinas | La diferencia de altura en el cuadrado pasa de `relieve_colinas` | Una o dos crestas y su ladera; terreno alto que disputar |
| 7 | Bosque | El bosque medio pasa de `bosque_campo` | Masas de bosque cerrado con claros y sendas; si pasa de `bosque_cerrado`, un claro central rodeado |
| 8 | Llano | Si no se da nada de lo anterior | Ondulado, con algún soto y alguna loma. Nunca liso |

## 6. Reglas de generación

### 6.1 Tamaño y escala interna contra externa

- Área jugable de `lado_jugable` (600 m de lado, cerrado con el autor el 2026-11-22), más un anillo de `anillo_fondo` (200 m) de terreno no jugable para el horizonte.
- **La escala interna no es la del mundo.** A 1:1 (1 u = 12 m) los 600 m serían 50 u, unas 12 celdas: más que toda la zona de una plaza de nivel 1 completa (techo de 10 u de radio, 20 u de lado a lado; `ZONA_INFLUENCIA.radioMaximoPorNivel`, `crates/nucleo/src/estado.rs:449`), e incluso más que la de nivel 5 (60 u). El autor lo descarta: una batalla abarca menos que una provincia.
- El campo lee del mundo un cuadrado de `lado_mundo` (propuesta: 8 u, dos celdas, menos de la mitad de una provincia de nivel 1) y lo estira a los 600 m jugables. **Escala interna:** `lado_jugable / lado_mundo` = 75 m por u, seis veces la del mapa de mundo.
- **Qué escala y qué no:**
  - Escala la **disposición**: dónde está la costa, por dónde va el río, dónde está el vado, dónde sube la loma, dónde está el bosque, de dónde llega cada bando.
  - No escala el **tamaño de los rasgos**: el ancho del río, el del vado, el de la playa y el de los caminos se fijan en metros de batalla (§12). Estirado ×6, el río de 18 m del mapa mediría más de 100 m.
  - La altura, solo si se exagera (§6.2).
- Con dos celdas de lado, las rejillas de 4 u del artefacto dan muy poca información dentro del campo (2×2 valores de bosque o de clase de terreno). Lo que manda es el relieve (16×16 muestras de 0,5 u), la línea del río, los vados y el bit de agua; el bosque se interpola con ruido.

### 6.2 Relieve

1. Altura del relieve del mundo, con interpolación bilineal (`Relieve.AlturaEn`), menos la media del área jugable.
2. Multiplicada por `exageracion_vertical`. Al estirar la planta ×6 y dejar la altura igual, las pendientes del mundo se aplanan ×6. Con `exageracion_vertical` igual a la escala interna (×6), las pendientes del campo son las del mundo. Con ×1, el campo sale más llano que el mundo. El autor lo calibrará en pruebas, y puede que no haga falta.
3. Si la diferencia de altura del área jugable queda por debajo de `relieve_minimo`, se suma una ondulación de semilla del lugar hasta alcanzarlo. Así el llano no queda liso.
4. Más un detalle de `detalle_amplitud` y `detalle_longitud` de onda.
5. La pendiente se recorta a `pendiente_max_transitable` salvo donde el tipo pide acantilado (paso, orillas altas de colina). Ahí se marca la zona acantilado.
6. Se aplanan las placas de spawn y de objetivo (radio `radio_placa`), para que las escuadras puedan formar.

### 6.3 Agua

- Mar y lago: donde el bit de agua del relieve está puesto. La orilla se ensancha a `ancho_playa` de arena y `ancho_somero` de agua somera, transitable y lenta; más allá, agua honda, infranqueable.
- Río: la polilínea del artefacto, ensanchada a `ancho_rio` como mínimo y hundida por debajo de las orillas (`altura_orilla`). El cauce es agua honda. No se reutiliza `CaucesDelMapa.Consultar` para la profundidad, que está ajustada al mapa de mundo: solo su geometría.

### 6.4 Vado

- El punto del vado es una franja de agua somera de `ancho_vado` a lo ancho del cauce, con las orillas rebajadas en rampa.
- El vado es agua rala: se cruza, pero ralentiza a quien la pisa (zona agua somera).
- **En un vado, los bandos van siempre en orillas opuestas** (autor, 2026-11-22).
- **El paso siempre se asegura:** si hay bandos en orillas opuestas, el vado está dentro del área jugable. Si el vado real cae fuera, se trae dentro.
- **El vado es el único paso.** No hay paso de reserva: el cuello de botella es la gracia.
- El cauce fuera del vado es infranqueable.

### 6.5 Bosque

- La densidad del artefacto (celdas de 48 m) se interpola y se le suma ruido de semilla del lugar, a celdas de `celda_bosque`.
- Por encima de `umbral_bosque_cerrado`: bosque cerrado, con árboles a `separacion_cerrado` y troncos con collider. Por encima de `umbral_bosque_claro`: bosque claro, sin colliders salvo algún tronco. Por debajo: algún árbol suelto.
- Las reglas de dónde no va un árbol (pendiente, agua, roca) son las de `ReglasDeVegetacion`, para que el bosque de batalla se parezca al del mapa de mundo.
- Las sendas se abren en el bosque cerrado: un camino del ticket que lo cruza, o una senda de semilla entre claros.

### 6.6 Caminos

Los tramos del ticket se aplanan a su anchura por escalón (`ancho_sendero`, `ancho_camino`, `ancho_calzada`) y abren paso por el bosque. Zona camino.

### 6.7 Spawns y objetivos

- **Spawns:** cada bando, en el borde del área jugable del lado de su procedencia, a `distancia_spawns` como mínimo entre bandos. En un vado, cada bando en su orilla. Si las dos procedencias caen del mismo lado, el defensor se queda en el centro y el atacante en el borde.
- **Captura y suministro los fija el modo de batalla** (autor, 2026-11-22): cuántos hay y dónde van respecto a los bandos. El terreno no los decide.
- La generación solo encaja cada punto donde el modo lo pide: lo mueve lo justo a una zona seca, llana y alcanzable, y le aplana la placa.
- Los ids automáticos de `CapturePointSetup` y `SupplyPointSetup` no cambian.

### 6.8 Validación

Antes de hornear, una inundación sobre la rejilla de zonas transitables comprueba que:

- cada spawn llega al otro;
- cada spawn llega a cada objetivo.

Si falla, se abre una senda (bosque) o un paso somero (agua) por el camino más corto, y se vuelve a comprobar. Un campo que no pasa la validación tras `intentos_validacion` cae al tipo llano con el relieve del lugar. Así nunca hay una batalla imposible.

## 7. Efectos en la táctica

Los modificadores dejan de ser uno por batalla y pasan a ser **por la zona que pisa cada unidad**. `TerrainModifier` sirve tal cual: solo cambia de dónde sale el terreno.

- Un sistema ECS nuevo lee la rejilla de zonas en la posición de cada unidad y escribe su zona actual.
- `StatResolveSystem`, que es el único escritor de stats, aplica los modificadores de esa zona en vez del singleton global de `BattleBootstrap`.
- El enum `BattleTerrain` gana agua somera. La altura no es una zona: sale de la diferencia de altura con el enemigo. Bosque, montaña y nieve quedan sin uso en batalla.

**Decisión del autor (2026-11-22):** bosque y camino no tienen efecto en batalla; afectan al movimiento de campaña, no al combate. En batalla son paisaje: el bosque se dibuja y sus troncos estorban como cualquier obstáculo, sin modificador. La arena también es paisaje. Solo cuenta el vado; la altura ya estaba implementada.

| Zona | Movimiento | Combate | Notas |
|---|---|---|---|
| Altura | Sin efecto nuevo | **Ya implementada; este sistema no la toca.** `DamageFormula.cs:126`: por encima de 0,5 m de ventaja, +10 % de daño por metro, sin tope | La cresta ya corta la línea de visión: el Terrain lleva collider y `LineOfSight.System` lanza rayos. Hoy no se nota porque el suelo es plano. Con relieve real y sin tope, una loma de 10 m daría +100 %: hay que calibrarlo junto con `exageracion_vertical` |
| Agua somera (vado, orilla) | Muy lento | Menos defensa; sin bracing | El cuello de botella del vado |
| Agua honda, acantilado | Infranqueable | | Flancos cerrados |
| Bosque, camino, arena | Sin efecto | Sin efecto | Paisaje |

La lentitud del vado va por dos lados a la vez: el coste de área del NavMesh (para que la ruta prefiera rodear) y el stat de velocidad de la zona (para que cruzar cueste de verdad).

La IA (escuadras y héroes de la IA) recibe de la generación los sitios clave del campo: el vado, la boca del paso, las crestas. Que el comportamiento Tactician los use queda para después de la v1.

## 8. Determinismo en red

La batalla es de servidor autoritativo, no de pasos sincronizados. Aun así, los seis clientes predicen su héroe sobre su NavMesh, y tienen que ver el mismo campo que el servidor.

- **Cada parte genera el campo por su cuenta** desde las mismas entradas (artefacto, relieve, ticket). Los combatientes ya reciben el ticket, y el artefacto y el relieve ya los tienen en disco. No se manda el campo por la red.
- **La generación es aritmética entera:** alturas en u16, posiciones cuantizadas, azar de `trazado::azar` portado a C# o SplitMix64. Nada de `Mathf.PerlinNoise` ni de `float` en lo que decide la forma. El ruido es entero, sobre una rejilla.
- **La semilla es del lugar, no de la batalla:** hash de `mapa_id` y del centro cuantizado a la celda. El mismo vado da siempre el mismo campo, y los jugadores pueden aprenderlo. Es una duda (§11, 1).
- **Huella:** cada cliente calcula la huella del campo y la manda al conectar; el servidor la compara con la suya. Si no casa, se registra y la batalla sigue. El servidor manda igualmente; lo peor es que ese cliente vea correcciones de predicción.
- **NavMesh:** cada parte la hornea de la misma geometría con la misma versión de Unity. Es lo único fuera de nuestro control. Las diferencias pequeñas solo cuestan correcciones, que ya se miden en `MedicionRedSystems.cs:335`.
- La versión del generador va en la huella, para que un cliente desactualizado lo detecte.

## 9. Asedios, pueblos y avanzadas

- **Asedio:** hoy lleva su propio mapa (`SettlementBattleSnapshot`) y no tiene centro. En la v1 queda fuera. Más adelante, con `centro`, se monta el terreno del lugar alrededor del trazado de la plaza.
- **Saqueo de pueblo, campamento de bandidos, avanzada, fundación:** usan `mapa_referencia`, así que el terreno sale ya. La v1 pone en el centro una huella genérica por contexto (chozas, empalizada del campamento), con semilla del id de la entidad. Usar el trazado real del motor (`urbanismo.rs:612, 657`) exigiría mandarlo en el ticket.
- **Avanzadas por etapas:** el doc del ciclo de batalla habla de un mapa interior por etapa. Con este sistema, cada etapa puede ser otro centro.

## 10. Límites de rendimiento

| Paso | Tamaño | Presupuesto |
|---|---|---|
| Lectura y generación | 1000×1000 m; alturas a 2 m (rejilla de 501², Terrain de 513²); zonas a 4 m (250²) | Menos de 300 ms en el servidor y en el cliente |
| Objetos | Árboles como instancias del Terrain (instanciado por GPU) | Hasta `max_arboles` (6000); colliders solo en troncos del área jugable, hasta `max_colliders` (1500) |
| NavMesh | Solo el área jugable, 600×600 m; vóxel de 0,25 (hoy 0,167) | Horneado asíncrono por teselas durante la preparación; objetivo, menos de 5 s. Es el paso con más riesgo: hay que medirlo antes de cerrar nada |
| Agua | Planos sin física, la orilla somera con material propio | Despreciable |
| Memoria | Artefacto y relieve en disco; el relieve se abre por teselas con LRU (`Relieve.cs`) | Unas pocas teselas de unos 50 KB |

Todo cabe en la fase de preparación, que ya existe: el cliente genera y hornea mientras el jugador ve la pantalla de preparación, y el minimapa de esa pantalla (`PantallasDeMundo.Preparacion.cs:408`, hoy siempre `DefaultMap`) se pinta de la misma rejilla de alturas y zonas.

## 11. Dudas para el autor

Respuestas del autor del 2026-11-22:

1. **Semilla del lugar:** sí. El mismo lugar da siempre el mismo campo.
2. **600×600 m jugables:** sí. La escala interna contra la externa queda por revisar: la batalla debe abarcar menos que una provincia. Propuesta en §6.1.
3. **Exageración:** se calibra en pruebas; puede que no haga falta. Ver §6.2.
4. **Vado:** el único paso es justo la gracia. El vado es agua rala y ralentiza. Los bandos, siempre en orillas opuestas, y el paso siempre asegurado.
5. **Río sin vado:** infranqueable. El paso lento es el vado.
7. **Objetivos:** los fija el modo de batalla.
8. **Contrato con el motor:** sí.
9. **Asedios y pueblos fuera de la v1:** sí.
10. **Arte:** de momento Synty; después se encarga.

6. **Efectos:** por lo que pisa cada unidad, solo el vado. La altura ya está implementada (daño por diferencia de altura) y no cambia. Bosque y camino no afectan en batalla (§7).
- **Escala:** 8 u del mundo estiradas a 600 m (75 m por u).

- **Arena:** paisaje, sin efecto.

No queda ninguna abierta.

## 12. Parámetros (placeholders)

| Parámetro | Valor | Qué es |
|---|---|---|
| `lado_jugable` | 600 m | Lado del área jugable |
| `anillo_fondo` | 200 m | Terreno no jugable alrededor |
| `lado_mundo` | 8 u | Lado del cuadrado del mundo que se estira al área jugable (el fondo lee un poco más) |
| `escala_interna` | 75 m/u | `lado_jugable / lado_mundo`; el mapa de mundo va a 12 m/u |
| `radio_lectura` | 1000 m | Radio de los caminos que manda el motor |
| `distancia_procedencia` | 2 u | Cuánto se retrocede sobre el plan para la procedencia |
| `exageracion_vertical` | ×1 a ×6 | Por calibrar: ×6 conserva las pendientes del mundo |
| `relieve_minimo` | 8 m | Diferencia mínima de altura en el área jugable |
| `detalle_amplitud` / `detalle_longitud` | 2 m / 60 m | Ruido de detalle |
| `pendiente_max_transitable` | 30° | Por encima, acantilado (el agente admite 45°) |
| `radio_placa` | 25 m | Placas llanas de spawn y objetivo |
| `paso_alturas` / celda de zonas | 2 m / 4 m | |
| `radio_vado_captura` | 300 m | Un vado más cerca hace el tipo vado |
| `radio_costa_captura` | 300 m | |
| `pendiente_paso` | 15° de media | |
| `relieve_colinas` | 15 m del mundo | Antes de exagerar |
| `bosque_campo` / `bosque_cerrado` | 0,3 / 0,7 | Densidad media para el tipo bosque |
| `umbral_bosque_claro` / `umbral_bosque_cerrado` | 0,25 / 0,6 | Por celda |
| `celda_bosque` / `separacion_cerrado` | 8 m / 4 m | |
| `ancho_rio` / `altura_orilla` | 24 m / 2 m | Mínimos |
| `ancho_vado` | 20 m | Único paso |
| `ancho_playa` / `ancho_somero` | 60 m / 25 m | |
| `ancho_sendero` / `ancho_camino` / `ancho_calzada` | 3 / 5 / 8 m | |
| `distancia_spawns` | 350 m | Mínima entre bandos |
| `altura_ventaja` | 4 m | Sobre el enemigo, para el efecto de altura |
| `intentos_validacion` | 3 | |
| `max_arboles` / `max_colliders` | 6000 / 1500 | |
