# Vista de Asentamiento — Trazado urbano dinámico

Documento vivo. Recoge el estado actual del diseño; lo que sigue sin implementar va al final, marcado como
abierto.

## Principio rector

Nada del trazado urbano se define al fundar el asentamiento. Ni calles, ni manzanas, ni reservas de espacio.
Todo emerge pieza por pieza, a medida que se construyen edificios.

## 1. Geometría de las calles

Las calles corren sobre las **aristas** de la rejilla, no sobre celdas: pasan entre celdas, por los vértices,
y no consumen superficie construible.

```
┌───┬───┬───┐
│ A ┃   ┃ B │      ┃ = calle sobre una arista
├───╄━━━╃───┤
│   ┃   ┃   │
└───┴───┴───┘
```

Consecuencias, que se cumplen por construcción y no por validación:

- Un edificio nunca puede quedar encima de una calle: la calle no ocupa celdas.
- Una calle nunca puede quedar encima de otra: la red se guarda como un **conjunto de aristas unitarias**, así
  que agregar una arista existente es un no-op.
- Un tramo nunca puede atravesar un edificio: toda arista corre por el borde entre dos celdas.

Solo ángulos de 90°. Ningún tramo diagonal.

## 2. Representación de la red

- La red es un **conjunto de aristas unitarias** de la rejilla, separado en `calles` y `caminos` (§10).
- Es **derivada, no persistida**: se reconstruye recorriendo los edificios en su orden de construcción y
  aplicando la regla de crecimiento paso a paso (`redDeCalles`, `engine/trazado.ts`). Mismo input, mismo
  output: no hace falta guardar nada ni migrar partidas guardadas.
- El motor la necesita para decidir dónde colocar cada edificio nuevo, así que se calcula ahí. `ui/canvas.ts`
  solo dibuja lo que recibe (acoplamiento 0: no importa nada de `engine/*` ni de `domain/types`).

## 3. Invariante de conexión

Todo edificio toca una calle o un camino: al menos una arista de su perímetro pertenece a la red. Vale igual
para un 1x1 que para el 6x6 de Granja nivel 4, sin ninguna regla de "fachada" aparte por tamaño.

## 4. Crecimiento de la red

- **Semilla.** Al fundar, la red inicial es el perímetro del Centro Urbano: sus cuatro lados como aristas de
  calle. Nace como un anillo cerrado y ofrece frente de calle en las cuatro direcciones desde el primer tick.
- Una calle se crea solo cuando hace falta, al colocar un edificio que la necesita.
- Un edificio que queda lejos de la red se conecta extendiendo la calle más cercana hasta él (nunca trazando
  un camino nuevo desde el Centro Urbano).
- Entre las aristas candidatas a fachada se elige la que **prolonga una calle existente en línea recta**, para
  que las fachadas se alineen con la hilera en vez de leerse como cruces sueltos.

## 5. Colocación de edificios

- Se prioriza la zona/barrio que le corresponde al tipo de edificio: una cuña de 45° en una dirección cardinal
  asignada de forma aleatoria-por-asentamiento (`direccionesDelAsentamiento`).
- Dentro de esa zona, se prefiere un hueco que **continúe una fila** ya construida sobre la misma calle
  (pared con pared con un vecino), después uno que ya tenga frente de calle, y solo si no hay ninguno de los
  dos se coloca suelto y se extiende la red hasta ahí.
- El barrio manda para elegir **dónde empieza una manzana nueva**; la fila manda **dentro de una manzana ya
  empezada** — una fila en crecimiento se completa aunque se desvíe del ángulo exacto de su cuña.

## 6. Tamaños de edificio

Cada tipo ocupa un rectángulo ancho×alto de celdas. El tamaño se deriva de `tipo` (+ `nivelInterno` en
Granja), nunca se persiste en el `Edificio` (`EDIFICIO_TAMANO`, `constants.ts`; `tamanoEdificio`,
`engine/trazado.ts`).

| Tipo | Tamaño | Ubicación |
|---|---|---|
| Centro Urbano | 3x3 | origen |
| Carpintería | 5x4 | barrio |
| Fundición | 2x2 | barrio |
| Curtiduría | 2x2 | barrio |
| Armería | 2x3 | barrio |
| Barracón | 2x2 | barrio |
| Galería de tiro | 2x4 | barrio |
| Leñera / Vivienda | 1x1 | barrio |
| Almacén | 2x1 | barrio |
| Mercado (pieza principal) | 3x2 | barrio |
| Puesto de mercado | 2x2, 3x2 o 1x1 según la pieza | barrio del Mercado |
| Palacio | 4x4 | sin barrio, lo más cerca posible del centro |
| Corral | 4x3 | afueras |
| Granja | 2x2 → 2x3 → 4x3 → 6x6 (niveles 1-4) | afueras |

**Tope de Almacenes.** La capacidad de almacenamiento no puede crecer sin límite: cada nivel de asentamiento
admite un número fijo de Almacenes — **4 / 8 / 16** (`NECESIDADES.maximoAlmacenesPorNivel`). Cuenta los de
cualquier estado (activo, en obra y en cola), y se aplica igual a la auto-construcción y a la adición manual:
es una regla del juego, no un heurístico interno.

## 7. Granja: crecimiento por niveles

Cuatro niveles internos (`EDIFICIO_CATALOGO.granja.niveles`). El **costo duplica en cada salto** sobre la base
de su costo de construcción: madera 30 → 60 → 120 → 240.

El **rinde de trigo sube mucho más despacio**, con multiplicadores sobre el nivel 1 y no acumulativos:
**×1 / ×1.5 / ×2 / ×3** (15 → 22.5 → 30 → 45). Una granja de nivel 4 cuesta 8 veces la de nivel 1 y rinde 3,
así que mejorar da rendimientos decrecientes por material invertido. Los trabajadores requeridos no cambian.

Al mejorar, el tamaño crece y la Granja se **muda**:

- No se exige que quepa en el sitio actual; busca el hueco de afueras más cercano posible a donde estaba.
- La mejora tiene prioridad sobre la cercanía: si el único hueco libre está en el extremo opuesto del mapa, se
  muda igual. Nunca se bloquea ni queda en espera por falta de espacio adyacente.
- Sigue las mismas reglas de afueras que cualquier Granja/Corral nuevo (§10).

## 8. Mercado: una zona, no un edificio

El Mercado es una **zona** compuesta por una pieza principal —el `mercado` de siempre, 3x2— más piezas
satélite de tipo `puestoMercado` que aparecen al alcanzar cada nivel interno:

| Nivel | Puestos que se añaden | Piezas totales |
|---|---|---|
| 1 | 2 de 2x2 | 3 |
| 2 | 4 de 1x1 + 3 de 2x2 | 10 |
| 3 | 2 de 3x2 | 12 |

- Los puestos son **un tipo distinto** de `mercado` a propósito: así el comercio (`cupoCaravanas`,
  `tieneMercadoActivo`, `EDIFICIOS_UNICOS`) sigue viendo exactamente una instancia de Mercado. El cupo de
  flota lo fija siempre la pieza principal.
- Nacen **gratis y ya activos**, sin pasar por la cola: son parte del Mercado que ya se pagó. No se pueden
  añadir a mano.
- Un puesto puede medir 2x2, 3x2 o 1x1. El discriminador es su `nivelInterno`, que en un puesto **no es
  progresión**: identifica su forma (`PUESTO_MERCADO_FORMA`). Es el mismo mecanismo que usa Granja, y evita
  persistir el tamaño en el `Edificio`.
- Comparten el barrio del Mercado, así que la acreción normal los agrupa alrededor de la pieza principal.
- **Si no hay hueco, el puesto se salta en silencio** y no bloquea la subida de nivel: la zona es superficie,
  no función. Ocurre de verdad — la zona de nivel 3 son 36 celdas de puestos y no entran en la cuña del barrio
  con un radio de influencia pequeño.
- Las partidas guardadas se rellenan al cargar (`migrarEdificiosAEspacioLocal`), de forma idempotente.

## 9. Manzanas emergentes

Las manzanas no se dibujan ni se guardan: son los ciclos de la red, el espacio negativo que queda encerrado
cuando las calles cierran un anillo. Salen de tres reglas:

1. **Fila con medianera.** Al buscar hueco, el edificio prefiere quedar pegado, pared con pared, al costado de
   otro que ya da a la misma calle, antes que empezar sitio nuevo. Sin separación entre vecinos.
2. **Dos hileras de fondo por manzana.** Sale del invariante de §3: si todo edificio debe tocar calle, detrás
   de la segunda hilera ya no se puede construir sin traer otra calle. Las calles de hilera caen cada 2 filas
   (`FONDO_MANZANA`, `engine/trazado.ts`).
3. **Transversales cada N columnas.** N se sortea por asentamiento en el rango 4–8
   (`TRAZADO.largoFilaMin`/`largoFilaMax`) con la misma semilla determinista que
   `direccionesDelAsentamiento`, más un desfase propio para que dos ciudades no tengan los bloques alineados a
   las mismas columnas. La manzana queda de N × 2 celdas. El criterio es **posicional**: cuando un edificio cae
   sobre una de esas líneas, aporta su trozo de calle. La línea completa se va formando conforme la ciudad
   crece hacia ahí, y si nunca crece, esa calle no llega a existir — no hay ningún plano previo, solo un
   criterio de dónde caería la transversal el día que alguien la necesite.

## 10. Afueras

Granja y Corral se colocan fuera de la trama urbana:

- Un **radio vedado** de 10 celdas (`TRAZADO.radioAfuerasMin`) alrededor del Centro Urbano donde no puede
  aparecer ninguno de los dos tipos.
- Dentro de la búsqueda, se prefiere siempre el hueco **más lejano** disponible, así que acompañan al borde de
  la ciudad a medida que crece.
- Las afueras llegan al menos hasta `radioAfuerasMin + anchoBandaAfueras` (`TRAZADO.anchoBandaAfueras`),
  aunque la zona de influencia del asentamiento sea todavía más chica — el campo de una ciudad está fuera de
  su zona de influencia, no dentro de ella. Esto es lo que le da hueco a la Granja inicial al fundar, cuando
  el radio de influencia (30) todavía es menor que el radio vedado (60).
- Igual les llega un **camino** que las conecta con la red — no quedan sueltas.

Camino ≠ calle: son dos clases distintas. La calle es urbana (forma filas, admite medianeras, cierra
manzanas). El camino solo conecta — se dibuja más fino y no genera manzanas ni atrae edificios a sus lados.

## 11. Dibujo

- `ui/canvas.ts` recibe todo resuelto (segmentos de calle/camino, huella por edificio) vía
  `DrawAsentamientoState`, armado en `main.ts` desde `gameStore.getTrazadoAsentamiento`. No importa nada de
  `engine/*` ni de `domain/types` para lógica.
- Los edificios se dibujan con un pequeño margen hacia adentro de su celda: como van pared con pared, sin ese
  margen la calle —que corre justo sobre el borde compartido— quedaría tapada.
- Sin etiqueta de texto bajo cada edificio: el tipo se identifica solo por color
  (`EDIFICIO_COLOR`, `ui/canvas.ts`).

---

## Abierto

- **Gates de mejora de Granja.** El costo y el rinde por nivel están definidos (§7), pero no hay ningún
  requisito adicional (nivel de asentamiento, edificio previo, etc.) que condicione la mejora más allá de tener
  los materiales — es el mismo criterio que ya usan Fundición/Curtiduría/Armería/etc., sin gate extra todavía.
