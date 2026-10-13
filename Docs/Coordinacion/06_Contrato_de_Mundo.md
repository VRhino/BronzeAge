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
| `bosques` | vectorial (`ZonaBosque`) | masas explotables con id y **capacidad de Leñeras** (pocos grandes, muchos medianos y pequeños) |
| `nodos` | vectorial (`NodoRecurso`) | yacimientos y ganado con cantidad inicial y anillo |
| `vetas` | vectorial (`Veta`) | distritos mineros de estaño (y oro), con su campamento de bandidos y su alijo lejano |
| `campamentos` | vectorial | campamentos de mercenarios: centros de los focos, a 600 unos de otros |
| `bandidos_iniciales`, `alijos` | vectorial | colocados por el generador, no por el motor |
| `navegacion` | grafo | espaciado 8 unidades (~333 nodos por lado), vados cada ~100: **la única autoridad de rutas** |

**Reglas de generación** (las del canon 1.9c; el generador las cumple y el contrato las comprueba al cargar):

1. Primero se colocan los campamentos de mercenarios (aptos: fértiles y conectados; separación 600; margen de borde 60), luego los campos de distancia y los anillos, luego el bosque y los recursos, luego las vetas.
2. **El estaño solo fuera de 200 de un campamento** (`exclusionEstanoCampamento`), agrupado en vetas de 3 a 6 nodos en un radio de 20-30, con cuotas que suman **exactamente** el total (en proporción a la tierra firme).
3. Los bosques son una mezcla: pocos grandes (radio 40-80) en la frontera y muchos pequeños (radio 4-20) dentro de los focos.
4. Las posiciones salen de datos versionados y reproducibles: nada se mueve a mano después.
5. **Abiertos para la fase 4** (decisión del usuario): dónde va el oro (O1) y cuántas Leñeras admite un bosque grande (O2).

## 3. Validaciones al cargar (servidor)

Hash del contenido = `Manifiesto.hash`; cada capa con su sha256; tamaño = 2.667 × 2.000; origen y destino de toda arista del grafo
transitables; todo nodo, campamento y alijo en tierra transitable; estaño a ≥ 200 de todo campamento; cuotas exactas; ningún
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
