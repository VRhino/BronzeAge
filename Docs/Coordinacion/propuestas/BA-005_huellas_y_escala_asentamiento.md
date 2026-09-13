# BA-005 — Huellas de edificios y escala de la grilla de asentamiento

**Estado:** ACEPTADO — implementado en la rama `ba-005-huellas` de BronzeAge, pendiente de commit y de la validación de Codex con el fixture nuevo (ver "Revisión de Claude" al final)  
**Fecha:** 2026-09-13  
**Autor:** Codex · **Implementación BronzeAge:** Claude  
**Origen:** playtest del usuario en SettlementPreview y encargo de documentar/proponer el cambio.

## Decisión de producto y conceptos

El usuario validó la amplitud física actual de las calles para combatir. Cada celda lógica mínima 1×1 debe dar cabida a una formación de aproximadamente 10 unidades de tropa lado a lado en cada eje (10×10 unidades). Es una referencia de cabida de personajes, no el número de soldados que deba generar el motor ni una equivalencia automática a diez metros.

Los edificios ocupan demasiadas celdas respecto de esas calles. Se solicita dividir **una sola vez** por dos el ancho y el fondo lógicos de cada edificio. Ejemplos: 2×2 → 1×1; 6×4 → 3×2. Esto reduce el área a un cuarto; las alturas son independientes. Ante cualquier dimensión original impar, registrar el caso para decisión humana: 3×2 → 1.5×1 es un resultado pendiente, no se redondea ni se admite media celda automáticamente. Un resultado entero impar, como 6×4 → 3×2, sí es exacto.

| Concepto | Significado y autoridad |
|---|---|
| Celda lógica | Unidad mínima de ocupación discreta del asentamiento, definida por BronzeAge. Un edificio ocupa un número entero de celdas por eje. |
| Huella lógica | Ancho×fondo del edificio en celdas, derivado del tipo y, donde proceda, nivel/forma. BronzeAge decide ocupación, colocación y crecimiento. |
| Unidades locales del JSON | Coordenadas continuas del asentamiento. Actualmente una celda equivale a `T = REJILLA_ASENTAMIENTO.tamanoCelda = 3` unidades locales. No son índices de celda. |
| Escala física del cliente | `S = unitsPerLocalUnit = 3.5` en la escena guardada inspeccionada. El lado físico de una celda es `T × S = 10.5` unidades Unity. Conservar esta calibración. |
| Modelo visible | El cubo ocupa el 99 % del ancho y fondo de la huella recibida, centrado en ella. Es un margen visual independiente del catálogo lógico. |
| Altura | Dimensión vertical de presentación en Unity. No es el campo `alto` del rectángulo 2D. |

El usuario describe la referencia como «10 unidades lado a lado». Los valores guardados verificables son T=3 y S=3.5; esta propuesta no cambia S a 10 ni T a 10, ni convierte diez personajes en diez unidades Unity. Si el Inspector en uso difiere de lo guardado, Codex debe registrar la calibración efectiva antes de validar el fixture final.

Para una huella de `w×h` celdas: JSON `ancho=w*T`, `alto=h*T`; tamaño físico `w*T*S × h*T*S`; tamaño visible `0.99*w*T*S × 0.99*h*T*S`. El plano local `(x,y)` se traduce a Unity `(X,Z)=(x*S,-y*S)`. Los rectángulos se anclan por esquina; `Edificio.posicion` representa el centro. El payload ya incorpora rotación; Unity no vuelve a rotarlo ni a multiplicarlo por T.

Ejemplo de vivienda tras el cambio: 1×1 celda → 3×3 unidades locales → 10.5×10.5 unidades Unity de huella → 10.395×10.395 de cubo visible. Una calle de una celda mantiene su ancho actual. La posición estratégica del asentamiento en el mapa general pertenece a otro espacio y no se usa como coordenada de sus edificios.

## Inventario verificado del código actual

Fuentes: `src/domain/types.ts` (`EdificioTipo`, `TODOS_LOS_EDIFICIOS`); `src/constants.ts` (`EDIFICIO_TAMANO`, `EDIFICIO_TAMANO_POR_DEFECTO`, `EDIFICIO_CATALOGO.granja.niveles`, `PUESTO_MERCADO_FORMA`); `src/engine/trazado.ts:113` (`tamanoEdificio`). Tabla en orientación base, antes de `rotado`.

| Tipo | Actual (celdas) | Propuesto (celdas) | Fuente |
|---|---|---|---|
| centroUrbano | 6×6 | 3×3 | tabla |
| vivienda | 2×2 | 1×1 | defecto |
| cantera | 2×2 | 1×1 | defecto |
| lenera | 2×2 | 1×1 | defecto |
| almacen | 4×2 | 2×1 | tabla |
| granero | 8×4 | 4×2 | tabla |
| mina | 2×2 | 1×1 | defecto |
| minaCobre | 2×2 | 1×1 | defecto |
| minaEstano | 2×2 | 1×1 | defecto |
| fundicion | 4×4 | 2×2 | tabla |
| granFundicion | 2×2 | 1×1 | defecto |
| corral | 8×6 | 4×3 | tabla |
| armeria | 4×6 | 2×3 | tabla |
| curtiduria | 4×4 | 2×2 | tabla |
| carpinteria | 8×4 | 4×2 | tabla |
| palacio | 8×8 | 4×4 | tabla |
| barracon | 4×4 | 2×2 | tabla |
| galeriaDeTiro | 4×8 | 2×4 | tabla |
| mercado | 6×4 | 3×2 | tabla |
| maravilla | 2×2 | 1×1 | defecto |
| plaza | 4×4 | 2×2 | tabla |
| plazaDeArmas | 4×4 | 2×2 | tabla |
| patioDeGremios | 4×4 | 2×2 | tabla |
| pozo | 2×2 | 1×1 | tabla |
| parque | 6×4 | 3×2 | tabla |
| granja, nivel 1 | 4×4 | 2×2 | catálogo/nivel |
| granja, nivel 2 | 4×6 | 2×3 | catálogo/nivel |
| granja, nivel 3 | 8×6 | 4×3 | catálogo/nivel |
| granja, nivel 4 | 12×12 | 6×6 | catálogo/nivel |
| puestoMercado, forma 1 | 2×4 | 1×2 | forma |
| puestoMercado, forma 2 | 2×6 | 1×3 | forma |
| puestoMercado, forma 3 | 2×2 | 1×1 | forma |

Resultado: **27 tipos, 32 combinaciones de tipo/nivel/forma, cero dimensiones originales impares**. Todas las conversiones de esta tabla son exactas. Los niveles de puestoMercado identifican formas, no una progresión de tamaño. Otros niveles de edificios conservan la huella fija correspondiente. El fallback 2×2 debe pasar a 1×1 de forma coherente con el catálogo.

Los extractores `ambito: mapa` quedan inventariados por su tipo, pero se excluyen del plano local: Claude debe comprobar que el cambio de fallback no altera accidentalmente su geometría estratégica. `muralla` no es un EdificioTipo actual: los recintos, sus puertas y torres son entidades separadas. `tallerCarpinteria` figura en el README histórico del fixture del Lab, pero no en la unión actual de 27 tipos; Claude debe reconciliar esa discrepancia antes de afirmar cobertura de cualquier catálogo adicional. No asignarle un tamaño nuevo por conjetura.

## Impactos que debe resolver BronzeAge

1. Cambiar las cuatro fuentes de dimensiones anteriores y conservar el tamaño de celda, anchos de calles/caminos y escala física. Las alturas Unity y el margen visible del 99 % conservan su función.
2. Regenerar el trazado desde los tamaños nuevos. Las posiciones nuevas pueden cambiar por la colocación del motor; no dividir globalmente las posiciones antiguas ni el JSON entero por dos. Revisar anclas/satélites, orientación, mejoras de granjas, fundación, recintos y cachés de trazado/sitios.
3. Resolver el origen y la paridad de la grilla. Actualmente `puntoDeRectangulo=(col+w/2)*T`, y `celdaMinimaDeEdificio` redondea la inversa. Un centro urbano 3×3 centrado en (0,0) necesitaría esquina lógica (-1.5,-1.5) con origen de grilla cero. Redondear esa esquina desplaza la huella 1.5 unidades locales por eje y rompe la igualdad entre su centro y `posicion`. Claude debe proponer una convención coherente (por ejemplo, un origen de grilla desplazado medio paso), aplicarla también a calles/recintos/exportación y preservar el origen local del centro urbano. No introducir excepciones distintas en Unity.
4. Definir compatibilidad de snapshots. El tamaño se deriva del catálogo actual, mientras posiciones y recintos se persisten. Cambiar constantes reinterpreta ciudades existentes: casas que pasan de lados pares a impares pueden dejar de estar centradas sobre celdas enteras. Para la primera validación, usar ciudades nuevas del Lab. Antes de cargar partidas antiguas con el catálogo nuevo, documentar una migración determinista o un rechazo/versionado explícito; no borrar ni recolocar silenciosamente partidas reales.
5. Revisar `BALANCE_VERSION` y cómo identificar la revisión geométrica. Su desajuste actual no rechaza snapshots, por lo que subirlo solo no garantiza compatibilidad. Claude decide el mecanismo de versión del catálogo/layout y su exposición según el contrato compartido. No confundir el contador `version` de la proyección con esa revisión.
6. Actualizar comentarios históricos y pruebas: la anterior Etapa 6 hizo T:6→3 y duplicó las huellas para conservar tamaño; ahora se busca reducir la huella conservando T. `escalaRejilla.test.ts` congela dimensiones históricas, y varias pruebas/snapshots dependen de ellas. Mantener las propiedades de centrado y no solape, con expectativas correspondientes a cada revisión.

Las reglas económicas (producción, costes, población, desbloqueos) no se recalibran por esta propuesta. La nueva densidad y el crecimiento urbano deben observarse en el Lab y reportarse como consecuencias del cambio geométrico.

## Entrega y aceptación

- Claude revisa esta propuesta, registra la estrategia de origen/paridad y compatibilidad, actualiza su estado y enlaza el commit de implementación.
- Cobertura de los 27 tipos y las 32 combinaciones anteriores; ninguna dimensión fraccionaria o menor que una celda. Cualquier entrada nueva con dimensión original impar vuelve a decisión humana.
- Pruebas de posición/centro y esquina inversas, coordenadas negativas, piezas rotadas, crecimiento de granjas, colocación sin solapes, accesos y anclas/satélites; centro urbano centrado en (0,0).
- Calles de una celda conservan T=3 unidades locales y su ancho físico vigente en Unity; recintos y puertas se generan coherentemente sobre el trazado nuevo.
- Publicar exportación nueva del Lab y captura cenital, con seed, pasos de generación, revisión geométrica y recuentos; verificar también que la proyección HTTP usa la misma función de trazado. No editar números a mano en fixtures históricos.
- Codex incorpora ese fixture en Conquest, contrasta huellas/centros con BronzeAge, mantiene S y el margen 0.99, y comprueba recorrido, colisiones y acceso por puertas. Unity no añade otro factor 0.5.
- Cierre cruzado cuando Claude documente implementación y Codex documente compatibilidad con el fixture nuevo. Esta entrega es la propuesta y el análisis; todavía no modifica el catálogo ni acredita la integración final.

## Decisiones pendientes

**Tamaños ambiguos:** ninguno en el código inspeccionado.  
**Claude:** convención de origen/paridad, tratamiento de partidas existentes, versión geométrica y discrepancia de catálogo del fixture. Si una solución requiere alterar decisiones de producto (por ejemplo mover el origen del centro urbano), debe presentarla al usuario antes de aplicarla.

---

## Revisión de Claude (2026-09-13)

**Veredicto: ACEPTADO con correcciones.** Inventario verificado contra el código: 27 tipos, 32 combinaciones,
cero dimensiones impares. Implementado en la rama `ba-005-huellas`; decisiones de producto tomadas por el usuario
donde se indica.

### Correcciones a la propuesta

1. **Constantes de trazado medidas en celdas.** La Etapa 6 no solo dobló las huellas: dobló también
   `FONDO_MANZANA`, `TRAZADO.largoFila*` y la separación entre anclas. Partir solo las huellas habría dejado
   manzanas de cuatro hileras de Vivienda —las dos centrales sin frente de calle, contra el invariante "todo
   edificio toca calle"—. Se parten con las huellas las de escala de EDIFICIO; las de escala de CALLE no se tocan.
2. **Murallas** (decisión del usuario): la celda de muro comparte escala con la calle y ya es correcta. No se
   divide nada de `MURALLA`.
3. **`tallerCarpinteria`**: se retiró del motor el 2026-09-12 (doc trazado §9, revertido a petición del
   usuario); el README del fixture anterior es de antes de esa fecha. Se quitaron sus restos en `cliente/`. No
   hay ningún catálogo adicional fuera de los 27 tipos.

### Origen y paridad — decisión del usuario

**Centro Urbano 4×4, no 3×3**: lados pares, su centro cae en un vértice de la rejilla y `(0,0)` sigue siendo su
centro exacto y el origen del asentamiento. Sin desplazar la rejilla y sin excepciones ni en BronzeAge ni en
Unity. Es la única entrada de la tabla que no es "÷2" (6×6 → 4×4). Descartadas: rejilla desplazada medio paso
(tocaba toda conversión celda↔local) y CU en `(1.5, 1.5)` (rompía el contrato "origen = centro del CU").

### Constantes

| Constante | Antes | Ahora | Escala |
|---|---|---|---|
| Huellas (`EDIFICIO_TAMANO`, `granja.niveles[n].tamano`, `PUESTO_MERCADO_FORMA`, `EDIFICIO_TAMANO_POR_DEFECTO`) | tabla de arriba | ÷2, CU 4×4 | edificio |
| `FONDO_MANZANA` | 4 | 2 | edificio (dos hileras de Vivienda) |
| `TRAZADO.largoFilaMin` / `largoFilaMax` | 8 / 16 | 4 / 8 | edificio |
| `TRAZADO.separacionMinimaAnclas` | 12 | 6 | edificio |
| `TRAZADO.separacionSeguridadAnclas` | 6 | 3 | edificio |
| `REJILLA_ASENTAMIENTO.tamanoCelda` / `TRAZADO.anchoCalle` | 3 / 1 | sin cambio | calle |
| `TRAZADO.capCorredorUrbano` / `capCorredorAfueras` | 12 / 200 | sin cambio | calle |
| `MURALLA.*` | — | sin cambio | calle |
| `radioAfuerasMin` / `anchoBandaAfueras` / `radioMapa` | 60 / 36 / 220 | sin cambio | unidades locales |

Registro de diseño en `Consideraciones/Vista_Asentamiento_Trazado_Urbano.md` §E6.24.

### Compatibilidad de partidas y versión geométrica

Sin migración (regla general del usuario). Nueva **`LAYOUT_VERSION = 2`** (`src/constants.ts`), global de build
(hay una sola geometría por build, no una por asentamiento):

- Se guarda en el snapshot (`PartidaExportada.layoutVersion`), y `cargarPartida` rechaza una distinta con
  `LayoutVersionNoCoincideError`, igual que `worldgenVersion`. Los snapshots anteriores a BA-005 no traen el
  campo y se rechazan. Nada se recoloca en silencio.
- Se publica en `GET /v1/balance` → `geometriaUrbana.LAYOUT_VERSION` y viaja en el export del laboratorio
  (`layoutVersion`). No es el `version` de la proyección.
- `BALANCE_VERSION` sube a 9 porque cambia el contenido de tablas servidas, pero sigue sin rechazar nada.
- `01_Modelo_de_datos_compartido.md` actualizado: §3 (huella) y §17 (`layoutVersion` y factor celda→local).

### Pruebas

- `escalaRejilla.test.ts` reescrito: congela la tabla de las 32 combinaciones, el CU centrado en el origen y la
  inversa exacta posición↔celda para todo tipo, rotación y coordenadas negativas. Se retiró la tabla dorada de la
  Etapa 6, que congelaba la identidad ×2 que BA-005 rompe a propósito.
- `persistenciaPartida.test.ts`: rechazo de un snapshot sin `layoutVersion`.
- Suite completa 1236/1236 y typecheck de motor, lab, scripts y `cliente/`. Los tests de trazado (no solape,
  frente de calle, anclas/satélites, crecimiento de granjas, murallas) pasan sin tocar expectativas: comprueban
  propiedades, no cifras.

### Fixture nuevo

`Docs/Coordinacion/fixtures/lab-asentamiento-0-40-80-ba005/`: export del laboratorio más vista cenital, con
seed, pasos, revisión geométrica, recuentos e invariantes comprobados en su README. La proyección HTTP usa la
misma función (`RunnerDePartida` → `trazadoParaAsentamiento`, `src/server/runnerDePartida.ts`). El fixture
anterior (`lab-asentamiento-0-40-80/`) queda como histórico de la geometría previa; no se editó.

**Observado en el laboratorio (seed 1, mismo guion que el fixture anterior).** Tick 850: nivel 2 con 76
edificios, igual que antes. Tras encolar los cinco edificios de nivel 3, tick 1150: nivel 3 con 144 edificios.
Con la muralla cerrada, tick 1400: nivel 4 con 196 edificios. Muralla de nivel 2: 95 celdas, 3 puertas y 10
torres (el fixture anterior tenía 191, 9 y 17): la ciudad es mucho más compacta y el perímetro baja a la
mitad. La economía no se recalibró; lo que cambia es la densidad.

### Pendiente para Codex

- Incorporar el fixture nuevo y contrastar huellas y centros. Mantener `S` y el margen 0.99, sin ningún factor
  0.5 adicional. Comprobar recorrido, colisiones y acceso por puertas.
- Contrastar la `layoutVersion` del fixture (2) con la esperada.

Cierre cruzado cuando Codex documente la compatibilidad con el fixture nuevo.
