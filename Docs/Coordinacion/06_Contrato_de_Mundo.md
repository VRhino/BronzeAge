# Contrato de mundo: artefacto y paquete de Unity

> **Estado (2026-10-13): diseño cerrado de la fase 2, sin código.** Esquema: [`proto/bronzeage/v1/world.proto`](../../proto/bronzeage/v1/world.proto).
> Resuelve P2 y P6 de [`Reconstruccion_Definicion.md`](../../Consideraciones/Reconstruccion_Definicion.md): hay un solo generador, el regional,
> y servidor y Unity comparten el mismo mundo por hash. **Sustituye a BA-003** (frontera de worldgen) y a lo que BA-005 decía de la escala del mundo.

## 1. Qué es

El mundo se produce **fuera de línea** por el generador regional (TypeScript, de Codex; D3) y es un **artefacto inmutable**:

- se identifica por el **hash** (sha256 del contenido canónico) y por la versión del formato;
- el servidor lo **carga** y no genera nada desde una semilla (adiós a regenerar el mapa al cargar una partida);
- Unity recibe un **paquete derivado** del mismo artefacto, a escala ×12, en sectores, identificado por su propio hash y por el hash del que deriva;
- si el hash que el cliente tiene no es el vigente, **no puede entrar**: el servidor decide, Unity representa.

Está en **unidades estratégicas** (2.667 × 2.000, en milésimas). El factor 12 solo vive en el paquete de Unity.

## 2. Contenido del artefacto (`ArtefactoDeMundo`)

| Capa | Forma | Para qué |
|---|---|---|
| `altura` | raster u16 | relieve; coste de movimiento y altura del terreno en Unity |
| `terreno` | raster u8 (`ClaseTerreno`) | agua, costa, llano, colina, montaña, cima |
| `fertilidad`, `humedad` | raster u8 | cultivos, bioma |
| `densidad_bosque` | raster u8 | campo continuo; coste de movimiento por bosque |
| `anillos` | raster u8 (`Anillo`) | foco (hasta 150), frontera (150-250) y tierra lejana (>250), por distancia al campamento más cercano |
| `transitable` | raster en bits | lo que se puede pisar; **autoridad única** de transitabilidad |
| `rios` | vectorial | cauces, vados |
| `bosques` | vectorial (`ZonaBosque`) | masas explotables con id y área (pocos grandes, muchos medianos y pequeños) |
| `parcelas` | vectorial (`ParcelaBosque`) | **cupos de explotación** de cada masa: puntos fijos separados `2·rp`, con su densidad. Una Leñera (que se construye en la ciudad) ocupa una parcela dentro de la zona de su asentamiento |
| `nodos` | vectorial (`NodoRecurso`) | yacimientos y ganado con cantidad inicial y anillo; estaño y oro por gradiente |
| `vetas` | vectorial (`Veta`) | distritos mineros de estaño (y oro), con su campamento de bandidos y su alijo lejano |
| `campamentos` | vectorial | campamentos de mercenarios: centros de los focos, a 600 unos de otros |
| `bandidos_iniciales`, `alijos` | vectorial | colocados por el generador, no por el motor |
| `navegacion` | grafo | espaciado 8 unidades (~333 nodos por lado), vados cada ~100: **la única autoridad de rutas** |

**Reglas de generación** (las del canon 1.9c; el generador las cumple y el contrato las comprueba al cargar):

1. Primero se colocan los campamentos de mercenarios (aptos: fértiles y conectados; separación 600; margen de borde 60), luego los campos de distancia y los anillos, luego el bosque y los recursos, luego las vetas.
2. **Estaño y oro por gradiente**: 5 % en el foco (muestra en el borde, un tercio de cantidad), 25 % en la frontera (aislados) y 70 % en la tierra lejana (vetas de 3 a 6 nodos de estaño y 0 a 2 de oro en un radio de 20-30), con cuotas por anillo que suman **exactamente** el total (en proporción a la tierra firme).
3. Los bosques son una mezcla: pocos grandes (radio 40-80) en la frontera y muchos pequeños (radio 4-20) dentro de los focos. Cada masa lleva su rejilla de **parcelas** (`rp` = 8, separación ≥ 16).
4. Las posiciones salen de datos versionados y reproducibles: nada se mueve a mano después.
5. **Decidido el 2026-10-13** (cierra O1 y O2): el gradiente de metales y las parcelas de explotación con cuota del 50 % por asentamiento (Doc 1.9c). Las cifras (`rp`, reparto, cuota) viven en el archivo común de datos de juego y se calibran en la fase 6.

## 3. Validaciones al cargar (servidor)

Hash del contenido = `Manifiesto.hash`; cada capa con su sha256; tamaño = 2.667 × 2.000; origen y destino de toda arista del grafo
transitables; todo nodo, campamento y alijo en tierra transitable; cuotas por anillo exactas; parcelas dentro de su masa y a ≥ 2·`rp` unas de otras; ningún
yacimiento dentro de agua. Un artefacto que no pasa **no se carga**.

## 4. El paquete de Unity (`PaqueteUnity`)

- Terreno a ×12 en **sectores** (`SectorUnity{id, rectangulo, archivo, sha256}`), cargables por separado.
- Es **derivado**, no una segunda fuente: no se edita a mano; se regenera con el mismo generador a partir del artefacto.
- Unity **no calcula rutas** (U1): pinta los planes que manda el servidor sobre ese terreno. La navegación local de Unity no decide
  ninguna ruta del mundo.
- El terreno de una batalla sale del artefacto (`BattleMapReference.mapa_id` = hash, `centro`).

## 5. Qué cambia respecto a BA-003 y BA-005

- BA-003 pedía una frontera «semilla → mundo»; ahora la frontera es el artefacto: no hay semilla en el contrato.
- La escala: mapa 2.667 × 2.000 (×12 en Unity), ciudad un 80 % más pequeña, zonas 6-30 (`Calibracion_Distancias_Definicion` y canon 1.0a-1.2).
- El trazado urbano lo calcula el **servidor** y viaja por la sesión (unidades locales; 200 por unidad de mapa); Unity solo lo dibuja
  (decisión del usuario, 2026-10-13).

## 6. Pruebas

El generador y el servidor comparten un fichero dorado: un mundo pequeño con su hash y el resultado esperado de las validaciones.
Unity carga el mismo fichero y comprueba que el paquete derivado coincide en hash con el declarado.

## 7. Formato del artefacto (implementado 2026-10-13)

Escritor: `generador/src/artefacto.ts`. Cargador y validador del servidor: `crates/mundo` (`Mundo::cargar`). Un artefacto real (semilla 42) ocupa
unos 4,7 MB y pasa todas las validaciones; la herramienta `cargo run -p mundo --example validar -- <archivo>` lo comprueba.

- **Unidades:** milésimas de unidad estratégica (`round(u · 1000 / 12)` desde las del generador).
- **Rasters:** celda de 4 unidades estratégicas (667 × 500 en el mundo real), filas de norte a sur, valor del centro de la celda. `altura` u16 LE;
  `terreno`, `fertilidad`, `humedad`, `densidad_bosque`, `anillos` u8; `transitable` en bits (celda `i` → byte `i>>3`, bit `i&7`, LSB primero). Todos con zstd.
- **Hash:** sha256 hexadecimal de la serialización Protobuf del artefacto con `manifiesto.hash = ""`. Cada capa lleva el sha256 de sus bytes comprimidos.
- **Coherencia con el raster:** la transitabilidad es la del raster. Grafo, recursos, campamentos, bandidos y alijos se ajustan al centro de una celda transitable
  antes de escribirse; el campo `anillo` de cada elemento conserva el del punto original.
- **Rendimiento del generador:** unos 12 minutos por mundo (relieve 2,5 min, metales y parcelas 2 min, recursos 2 min, grafo 2 min, rasters 3,5 min); es una herramienta fuera de línea.
- **Ríos y vados (2026-10-13):** los ríos cortan el paso salvo en los vados. El trazo del río se marca como barrera en la rejilla de transitabilidad, engrosado una celda a cada lado (así ninguna arista lo cruza entre dos muestras), y cada vado abre un hueco de 100 unidades del generador de radio. Siempre hay al menos un vado por río y luego uno cada 100 estratégicas (`generador/src/vados.ts`). En la semilla 42: 116 vados; el grafo tiene 52.914 nodos, 19 componentes y el 99,92 % en la mayor, con los 9 campamentos dentro de ella.
- **Pendiente:** hierro en las vetas y vados elegidos por geografía (hoy equiespaciados a lo largo del río).
