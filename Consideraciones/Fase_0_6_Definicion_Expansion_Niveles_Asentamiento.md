# Fase 0.6 — Expansión de niveles de asentamiento (3 → 5) y nueva progresión de gates

> IMPLEMENTADO (esta sesión). Este documento pasó por un consejo LLM (5 asesores + revisión cruzada, mismo
> proceso que [Fase_0_5_Definicion_Especializacion_y_Cupos.md](Fase_0_5_Definicion_Especializacion_y_Cupos.md))
> y sus hallazgos fueron resueltos por el usuario — ver §5. §1 documenta el estado ANTERIOR (3 niveles) como
> línea base de referencia; §2/§3 son lo que ya está en el código. El usuario decidió implementar primero y
> correr simulaciones de calibración DESPUÉS (no antes, como recomendó el consejo) — Fase 0 sigue lejos de
> v1.0.0, en fase activa de añadir/quitar/pulir. **186/186 tests en verde** tras la implementación (tsc y
> vitest limpios; 2 fixtures de `lineas_produccion.test.ts` y el snapshot baseline actualizados a propósito
> por el cambio de balance, ver §6).

## 0. Qué cambia respecto a hoy, en una frase

Hoy `NIVEL_ASENTAMIENTO.nivelMaximo = 3`, la piedra se mezcla con la madera desde nivel 1 en varios edificios,
y el gate de nivel 2 combina población + los 3 edificios de transformación de golpe. Fase 0.6 separa la
piedra a partir de nivel 2, convierte los gates de nivel en **puramente edificios (sin población)**, reparte
la responsabilidad de "transformación" y "militar" en escalones distintos, y añade dos niveles nuevos arriba
(4 = Palacio, 5 = Maravilla) con Murallas como pieza intermedia en nivel 3→4.

## 1. Estado actual del código (línea base, sin tocar)

### 1.1 Catálogo completo de edificios — costo, gate de construcción, niveles internos

Fuente: `EDIFICIO_CATALOGO` (`src/constants.ts`). "Gate construcción" = `requisitoNivelAsentamientoConstruccion`
(nivel mínimo para poder AÑADIR el edificio a la cola por primera vez; vacío = construible desde nivel 1).
"Gate mejora" = `requisitoNivelAsentamiento` de cada nivel interno (2, 3...).

| Edificio | Categoría (`SCORE_BANDAS`) | Costo construcción BASE | Gate construcción | Niveles internos — costo de mejora | Gate de cada mejora |
|---|---|---|---|---|---|
| `centroUrbano` | especial (nace activo) | — | — | — | — |
| `vivienda` | crecimiento | madera 10 | — | — | — |
| `granja` | supervivencia | madera 30 | — | N2: madera 60 · N3: madera 120 · N4: madera 240 (sin piedra hoy) | ninguna mejora tiene gate hoy |
| `cantera` | extractorBase | madera 20 | — | — | — |
| `lenera` | supervivencia | madera 10 | — | — | — |
| `almacen` | crecimiento | **madera 50 + piedra 30** | — | — | — |
| `mina` (oro) | extractorBase | **madera 40 + piedra 10** | — | — | — |
| `minaCobre` | extractorBase | **madera 30 + piedra 5** | — | — | — |
| `minaEstano` | extractorBase | **madera 50 + piedra 20** | — | — | — |
| `corral` | extractorBase | madera 30 | — | — | — |
| `granFundicion` | especial/élite | madera 150 + piedra 100 + oro 50 | gate = nivel de FACCIÓN 3 (no de asentamiento) | — | — |
| `fundicion` | transformacion | madera 80 + piedra 40 | — (desde nivel 1 hoy) | N2: madera 150 + piedra 100 | nivel 2 |
| `curtiduria` | transformacion | madera 80 + piedra 30 | — | N2: madera 150+piedra 100 · N3: madera 450+piedra 200 | nivel 2 · nivel 3 |
| `armeria` | transformacion | madera 80 + piedra 30 | — | N2: madera 150+piedra 100 (+carpintería) · N3: madera 450+piedra 200 (+palacio) | nivel 2 · nivel 3 |
| `carpinteria` | transformacion | madera 60 + piedra 20 | **nivel 2** | N2: madera 120 + piedra 60 | nivel 3 |
| `barracon` | manual (militar) | madera 30 | — | N2: madera 100+piedra 60 (+carpintería) · N3: madera 300+piedra 200 (+palacio) | nivel 2 · nivel 3 |
| `galeriaDeTiro` | manual (militar) | madera 50 | — | N2: madera 140+piedra 20 (+carpintería) · N3: madera 400+piedra 100 (+carpintería N2) | nivel 2 · nivel 3 |
| `palacio` | especial | madera 1500 + piedra 1000 | **nivel 3** | único tier | — |
| `mercado` | comercio | madera 100 (sin piedra) | — (desde nivel 1) | N2: madera 150+piedra 100 · N3: madera 450+piedra 200 | nivel 2 · nivel 3 |
| `puestoMercado` | especial (satélite auto) | — | — | — | — |
| `maravilla` | especial/trofeo | madera 5000+piedra 5000+oro 500+cobre 300+estaño 200+livestock 200 | **nivel 3 (tope actual)** | — | — |

### 1.2 Gates para SUBIR de nivel hoy (`NIVEL_ASENTAMIENTO.requisitos`)

| Subir a | Población mínima | Edificios activos exigidos |
|---|---|---|
| Nivel 2 | 200 pesants + 50 artesanos | `armeria`+`curtiduria`+`fundicion` |
| Nivel 3 | 500 pesants + 200 artesanos | `carpinteria`+`barracon`+`galeriaDeTiro` |

`nivelMaximo = 3` hoy, sube de forma monótona.

### 1.3 Otros sistemas indexados por nivel (1-3 hoy)

| Sistema | Constante | Nivel 1 / 2 / 3 |
|---|---|---|
| Techo de población total | `NIVEL_ASENTAMIENTO.techoPoblacion` | 300 / 1500 / 6000 |
| Radio de zona de influencia | `ZONA_INFLUENCIA.radioMaximoPorNivel` | 60 / 90 / 120 |
| Mantenimiento cobra piedra desde | `MANTENIMIENTO.nivelParaPiedra` | nivel 2 (piedraBase 3) |
| Mantenimiento cobra oro desde | `MANTENIMIENTO.nivelParaOro` | nivel 3 (oroBase 2) |
| Tope de Almacenes | `NECESIDADES.maximoAlmacenesPorNivel` | 4 / 8 / 16 |
| Cupo de asentamientos de ese nivel por Facción (Fase 0.5) | `CUPO_NIVEL_ASENTAMIENTO` | solo cubre niveles 2 y 3 de asentamiento |

## 2. Definición de Fase 0.6 (decisiones confirmadas por el usuario)

### Nivel 1 — arranque, todo en madera

- Toda construcción BASE cuesta solo madera. **Confirmado que esto incluye a `almacen`, `mina`, `minaCobre`,
  `minaEstano`** — hoy piden piedra, la pierden. Ningún gate de nivel para construcción base en este escalón.
- **Mejoras de Granja piden piedra**, cifra confirmada: nivel interno 2 → piedra 20, nivel interno 3 →
  piedra 40, nivel interno 4 → piedra 80 (se suma al costo en madera ya existente, que no cambia: 60/120/240).
  Sin gate de nivel de asentamiento — las 4 mejoras siguen siendo alcanzables estando en nivel 1, igual que
  hoy.
- Mercado se mantiene construible desde nivel 1 (sin cambio).
- **Gate para subir a nivel 2**: ≥3 edificios de extracción construidos, **en paralelo con el umbral de
  población que ya existe hoy** (200 pesants + 50 artesanos) — no lo reemplaza, se suman ambas condiciones
  (mismo criterio en el resto de transiciones, ver más abajo — corregido tras aclaración del usuario: la
  versión anterior de este documento había interpretado que la población se eliminaba, error ya corregido).

### Nivel 2 — entra la piedra, entra la transformación

- **Se desbloquea la construcción BASE de `fundicion`, `curtiduria` y `armeria`** — ganan
  `requisitoNivelAsentamientoConstruccion: 2` (hoy no lo tienen, se pueden construir desde nivel 1). Con esto
  la responsabilidad de "producir transformación" queda exclusivamente en manos de los edificios de nivel 2,
  sin que haga falta prohibirlo en ningún otro sitio.
- **Se desbloquea también la construcción BASE de `barracon` y `galeriaDeTiro`** — ganan igualmente
  `requisitoNivelAsentamientoConstruccion: 2`. Añadido después de Fase 0.6, al diseñar el trazado por anclas
  (ver `Vista_Asentamiento_Trazado_Urbano.md` §5.7.1): son los dos tipos capaces de abrir el grupo militar y
  arrastrar consigo la Plaza de Armas, y al fundar (disco urbano de 5 celdas) no hay ningún hueco que respete
  la separación mínima entre anclas — el núcleo militar nacía pegado al Centro Urbano y ahí se quedaba, porque
  ningún ancla se muda nunca. Sin gate era alcanzable en el tick 1: el Barracón cuesta 30 de madera y la
  caravana de fundación entrega 50. **No adelanta ni retrasa nada para el jugador**: nivel 2 ya era el suelo
  real, porque las tres tropas de `nivelRequerido: 1` piden armaMadera/armaCobre/armaduraBasica y las tres las
  fabrica solo la Armería, que ya exigía nivel 2. Estos dos NO llevan piedra: siguen costando solo madera.
- **Alcance de la piedra confirmado**: la piedra que "se introduce" en este nivel es la que ya llevaban de
  fábrica `fundicion`/`curtiduria`/`armeria` (80+40, 80+30, 80+30) — no se añade piedra retroactivamente a
  ningún tipo de nivel 1 (`granja`, `cantera`, `lenera`, `vivienda`, `corral`, `mina`, `minaCobre`,
  `minaEstano`, `almacen`, `mercado` siguen en madera pura para siempre, aunque se construya una segunda o
  tercera unidad estando ya en nivel 2 o superior).
- **Gate para subir a nivel 3**: 3 edificios de transformación (`fundicion`+`curtiduria`+`armeria`, los 3 —
  con el set actual eso es la totalidad) **+ 2 edificios militares** (`barracon`+`galeriaDeTiro`, ambos) **+
  el umbral de población que ya existe hoy para nivel 3** (500 pesants + 200 artesanos). `barracon`/
  `galeriaDeTiro` pasan además a exigir nivel 2 para su propia construcción BASE (ver el bullet de Nivel 2
  arriba; en la redacción original de Fase 0.6 no llevaban gate y se construían desde nivel 1). Eso no afecta
  a este gate: para llegar a nivel 3 hay que pasar por nivel 2 igualmente, así que el conjunto de
  asentamientos capaces de cumplirlo es el mismo. Lo que sí cambia respecto a antes de Fase 0.6 es que ahora
  cuentan como requisito para avanzar de nivel, no solo como edificios sueltos.

  > ⚠️ **Este gate está roto para cualquier asentamiento sin jugador humano**, y la causa es este cambio:
  > `barracon` y `galeriaDeTiro` **no se auto-construyen** (su política de desbloqueo se retiró y no se
  > sustituyó) y la gobernanza NPC solo añade `mercado`, así que los 5 edificios exigidos no se reúnen nunca.
  > Antes de Fase 0.6 el gate eran los 3 de transformación, que sí se auto-construyen, y el nivel 3 se
  > alcanzaba. Diagnóstico completo, evidencia de batch a ambos lados del cambio y opciones de arreglo en
  > [issues/nivel_3_inalcanzable_sin_jugador_humano.md](../issues/nivel_3_inalcanzable_sin_jugador_humano.md).

### Nivel 3 — carpintería y murallas

- Se desbloquea la construcción de `carpinteria` (su gate sube de nivel 2 a nivel 3).
- **Se desbloquea `muralla`** — edificio nuevo. Implementación mínima confirmada: ocupa 1 celda, cuesta
  **piedra 2000** (sin madera), sin niveles internos ni receta, sin efecto mecánico todavía (no interactúa
  con asedio/combate en esta pasada — existe como edificio construible, nada más). Gate de construcción:
  nivel 3.
- **Gate para subir a nivel 4**: `muralla` construida **+ umbral de población nuevo** (nivel 3→4 no existía
  hasta ahora — cifra sin definir, ver §4).

### Nivel 4 — palacio

- Se desbloquea la construcción de `palacio` (su gate sube de nivel 3 a nivel 4).
- **Gate para subir a nivel 5**: `palacio` construido **+ umbral de población nuevo** (nivel 4→5 tampoco
  existía hasta ahora — cifra sin definir, ver §4).
- Palacio sigue además desbloqueando la aparición de Nobleza (sin cambios, `engine/population.ts`) — ahora
  cumple doble función: puerta de población noble Y requisito de nivel.

### Nivel 5 — la maravilla

- Se desbloquea la construcción de `maravilla` (su gate sube de nivel 3 a nivel 5). `nivelMaximo` pasa de 3
  a 5. Sin gate de subida más allá — nivel 5 es el tope.

### Resumen de la escalera de gates (subir de nivel)

Población y edificios van EN PARALELO en las 4 transiciones — se exigen ambas condiciones a la vez, igual que
hoy. Los umbrales de nivel 2 y 3 son los que ya existen; los de nivel 4 y 5 son nuevos (sin cifra todavía).

| Transición | Población mínima | Requisito de edificios |
|---|---|---|
| 1 → 2 | 200 pesants + 50 artesanos (ya existe) | ≥3 edificios de extracción |
| 2 → 3 | 500 pesants + 200 artesanos (ya existe) | 3 de transformación (fundición+curtidería+armería) + 2 militares (barracón+galería de tiro) |
| 3 → 4 | **sin definir** (nuevo, ver §4) | `muralla` construida |
| 4 → 5 | **sin definir** (nuevo, ver §4) | `palacio` construido |

### Resumen de la escalera de construcción (qué se puede empezar a construir en cada nivel)

| Nivel | Se desbloquea construir | Piedra en construcción BASE desde aquí |
|---|---|---|
| 1 | todo lo demás (extracción, granja, vivienda, almacén, mercado) | no, solo madera |
| 2 | `fundicion`, `curtiduria`, `armeria`, `barracon`, `galeriaDeTiro` | sí, en esos 3 primeros únicamente |
| 3 | `carpinteria`, `muralla` | sí (ya la llevaban / la lleva `muralla`) |
| 4 | `palacio` | sí (ya la llevaba) |
| 5 | `maravilla` | sí (ya la llevaba) |

## 3. Inventario de deltas de código (sin implementar todavía)

1. Quitar `piedra` del costo base de `almacen`, `mina`, `minaCobre`, `minaEstano`.
2. Añadir `piedra: 20/40/80` a `granja.niveles[2/3/4].costoMejora` (junto a la madera que ya tienen).
3. Añadir `requisitoNivelAsentamientoConstruccion: 2` a `fundicion`, `curtiduria`, `armeria`.
4. Mover `carpinteria.requisitoNivelAsentamientoConstruccion` de 2 a 3.
5. Crear el tipo `muralla` en `EdificioTipo` (`domain/types.ts`) y su entrada en `EDIFICIO_CATALOGO`:
   `costo: { piedra: 2000 }`, `requisitoNivelAsentamientoConstruccion: 3`, tamaño 1x1, sin niveles/recetas.
   `tiempoConstruccionTicks` **propuesta, pendiente de ajuste**: 15 (comparable a Palacio, que tarda 20 por
   2500 de material combinado; 2000 de piedra sola sugiere algo por debajo).
6. Mover `palacio.requisitoNivelAsentamientoConstruccion` de 3 a 4.
7. Mover `maravilla.requisitoNivelAsentamientoConstruccion` de 3 a 5; subir `NIVEL_ASENTAMIENTO.nivelMaximo`
   de 3 a 5.
8. **Extender `NIVEL_ASENTAMIENTO.requisitos` a niveles 4 y 5**, manteniendo la forma actual
   `{ pesants, artesanos, edificios }` — población Y edificios siguen yendo juntos, confirmado por el usuario
   (la lectura anterior de este documento, que asumía que la población se eliminaba, era errónea). Solo
   cambia:
   - La LISTA de edificios de cada nivel (extracción en 2, transformación+militar en 3, `muralla` en 4,
     `palacio` en 5).
   - El gate de nivel 3 (`2→3`) pasa de "una lista fija de 3 edificios" a "dos listas a la vez"
     (transformación Y militar) — como cada conjunto tiene exactamente 3 y 2 miembros respectivamente, "al
     menos 3"/"al menos 2" equivale hoy a "todos", así que se puede implementar concatenando ambas listas en
     el mismo array `edificios` sin necesitar un contador genérico "N de M" — a menos que se prevea ampliar
     esos conjuntos más adelante, en cuyo caso sí hace falta el contador.
   - Las cifras de población de nivel 4 y 5 no existen todavía (ver §4).

## 4. Puntos que quedan abiertos — propuestas mías, pendientes de que las ajustes

Estos NO fueron parte de tu respuesta — son proyección para que el resto del sistema no quede descolgado de
los 5 niveles nuevos. Números en **negrita** son placeholder mío, no decisión tuya.

1. **Set exacto de "edificios de extracción"** para 1→2: propongo los 6 tipos ya categorizados como
   `extractorBase`/`supervivencia` en `SCORE_BANDAS` — `cantera`, `lenera`, `mina`, `minaCobre`, `minaEstano`,
   `corral` — contando **tipos distintos**, no instancias repetidas del mismo tipo (3 Canteras no cumplen
   solas; hace falta variedad).
1b. **Umbral de población para 3→4 y 4→5** (no existían hasta ahora, el tope de Fase 0 llegaba solo a nivel
   3): propuesta continuando la progresión actual (200/50 → 500/200, ratio pesants ×2.5, artesanos ×4) —
   **3→4: 1000 pesants + 400 artesanos · 4→5: 2000 pesants + 800 artesanos**. Coherente con el
   `techoPoblacion` propuesto en el punto 2 de abajo (12000/20000): en ningún caso el umbral de SUBIDA supera
   el techo del nivel anterior.
2. **Techo de población** (`techoPoblacion`) para niveles 4 y 5: propuesta **4: 12000 · 5: 20000** — continúa
   la curva actual (300→1500→6000) pero desacelerando el múltiplo (×2 y ×1.7 en vez de ×5 y ×4), asumiendo
   que 4-5 son niveles de "cierre de partida" más que de crecimiento bruto.
3. **Radio de zona de influencia** (`radioMaximoPorNivel`) para niveles 4 y 5: propuesta **4: 150 · 5: 180**
   — continúa el incremento fijo de +30 que ya usan los 3 niveles actuales.
4. **Tope de Almacenes** (`maximoAlmacenesPorNivel`) para niveles 4 y 5: propuesta **4: 24 · 5: 32** —
   desacelera la duplicación actual (4→8→16) a +50%/+33%.
5. **Mantenimiento**: `nivelParaPiedra` (2) y `nivelParaOro` (3) no necesitan cambios — ambos umbrales ya
   caen dentro del rango 1-5 sin modificar nada. Sin propuesta de un tercer material a niveles 4/5 salvo que
   se quiera introducir uno nuevo — fuera de alcance de este boceto.
6. **`CUPO_NIVEL_ASENTAMIENTO`** (Fase 0.5, cupo de asentamientos de nivel N por nivel de Facción): sigue
   sin extender a niveles 4 y 5. Recomiendo resolverlo en una pasada aparte, después de cerrar esta, porque
   toca el invariante `maxN2+maxN3=total-1` de Fase 0.5 y probablemente cambie de forma, no solo de cifras
   (con 4 escalones cupados en vez de 2, ¿sigue teniendo sentido "el primero siempre en nivel 1 obligado"?).
7. **`murallas` sin mecánica**: confirmado como implementación mínima a propósito — cuándo (y si) se le da
   efecto real en asedio/combate queda fuera de esta fase, anotado para revisar junto con Doc 5.10.
8. **No existe hoy ningún tope de Viviendas por nivel** (pregunta del usuario, verificado contra el código):
   a diferencia de Almacén (`NECESIDADES.maximoAlmacenesPorNivel`), Vivienda se autoconstruye solo por
   ocupación (`umbralViviendaOcupada = 0.85`, `engine/construction.ts:530`), sin ningún máximo numérico ligado
   al nivel de asentamiento — el único freno indirecto es el espacio físico de la rejilla urbana y el propio
   `techoPoblacion` (una vez la población topa el techo de su nivel, deja de haber presión de ocupación que
   dispare más Viviendas). Con 5 niveles y techos de población mucho más altos en 4-5 (propuesta del punto 2),
   **puede valer la pena introducir un `maximoViviendasPorNivel`** explícito, mismo patrón que Almacén — no es
   estrictamente necesario si el techo de población ya actúa como freno indirecto, pero sin él la huella
   urbana de una ciudad nivel 5 depende 100% de cuánto espacio quede en la rejilla, no de una regla de diseño.
   Sin propuesta de cifra — queda para que decidas si se introduce. **RESUELTO, ver §7.**

## 5. Consejo LLM y resolución del usuario

Este documento pasó por el mismo proceso de consejo (5 asesores + revisión cruzada) que Fase_0_5. Resumen de
lo que el consejo encontró y cómo lo resolvió el usuario:

1. **¿Regresión de nivel si se destruye Muralla/Palacio después de haber subido?** — **RESUELTO**: NO. Los
   edificios exigidos por un gate son un CHEQUEO PUNTUAL al subir de nivel, no una condición sostenida — el
   asentamiento nunca vuelve a comprobar "¿sigo teniendo Muralla?" una vez ya subió. `nivelActual` solo baja
   por los medios que Fase_0_5 §6.2 ya define (mantenimiento tocando 0), sin relación con perder un edificio
   específico. Ningún cambio de código adicional respecto al mecanismo de mantenimiento ya existente.
2. **¿Es esto un patch de balance o una decisión de identidad de juego?** (el consejo detectó que niveles 4-5
   se sienten menos definidos que 1-3: Muralla sin mecánica, Palacio ya existía, Maravilla es un trofeo) —
   **RESUELTO**: es una decisión de identidad de juego, tomada con el propósito de balancear el ARRANQUE de
   partida (de ahí el peso puesto en 1-3). Que 4 y 5 se sientan menos completos es intencional en esta etapa:
   el proyecto sigue lejos de v1.0.0, en fase activa de iteración — esos dos niveles son la oportunidad de
   definirse mejor a medida que avance el desarrollo, no un vacío a rellenar ya. Retrocompatibilidad con
   partidas/saves antiguos, planteada por el consejo como riesgo, **no aplica en esta etapa del proyecto**.
3. **Validar la piedra (cantera vs. demanda simultánea en nivel 2) antes de implementar** — el consejo lo
   recomendó como bloqueante; **el usuario decide invertir el orden**: se implementa todo lo definido en
   §2/§3 primero, y las simulaciones de calibración (mismo tipo que ya se usaron para destrabar el deadlock
   de Fase_0_5 §1.2) se corren DESPUÉS, sobre el sistema ya implementado, para pulir las cifras placeholder
   de §4 con datos reales en vez de before-the-fact.
4. El resto de hallazgos del consejo (el "N de N+1" implícito en el gate 2→3 concatenado, el orden de
   implementación, `CUPO_NIVEL_ASENTAMIENTO` sin extender a niveles 4-5) se atienden durante la
   implementación — ver checklist de §3 y notas de código.

## 6. Registro de implementación (esta sesión)

Todo el checklist de §3 está en el código. Notas de lo que salió distinto de lo planeado al implementar:

- **El gate de nivel 2 (≥3 de 6 edificios de extracción) SÍ necesitó el contador genérico "N de M"** que el
  consejo (Executor/Contrarián) discutió — a diferencia del gate de nivel 3 (transformación+militar, donde
  "al menos 3 de 3" y "al menos 2 de 2" equivalen a "todos", una simple lista concatenada basta), aquí son 6
  candidatos y solo hacen falta 3, que NO es lo mismo que "todos". `NIVEL_ASENTAMIENTO.requisitos` ganó un
  campo opcional `edificiosMinimo` (`constants.ts`) y `calcularNivelAsentamiento`
  (`engine/mantenimiento.ts`) cuenta tipos distintos construidos en vez de exigir la lista completa cuando
  ese campo está presente — con `edificiosMinimo` ausente se comporta exactamente igual que antes.
- **Landmine real encontrado y corregido, confirma la advertencia del consejo sobre `CUPO_NIVEL_ASENTAMIENTO`**:
  `tieneCupoParaNivel` (`engine/simulation.ts`) leía `cupoRestante.get(...) ?? 0` para CUALQUIER nivel
  objetivo ≥2 — como el Map solo se siembra para niveles 2 y 3, subir a nivel 4 o 5 habría leído `0` siempre
  y **NINGÚN asentamiento habría podido pasar de nivel 3 nunca**, silenciosamente. Corregido: niveles 4 y 5
  quedan sin cupo (ilimitados) hasta que el usuario defina una curva, mismo criterio que nivel 1.
- **Fundición/Curtidería/Armería necesitaron el gate también en la AUTO-construcción**, no solo en la
  adición manual (`anadirEdificioManualmente` ya lo aplicaba genéricamente vía `requisitoNivelBase`, pero el
  disparador automático de estos 3 tipos en `evaluarNecesidades` no pasaba por esa función) — sin este
  segundo punto, un asentamiento en nivel 1 con cobre en almacén seguía auto-construyendo Fundición pese al
  gate nuevo del catálogo.
- **`ProgresoNivelAsentamiento` (UI de progreso hacia el siguiente nivel)** asumía "requeridos = candidatos
  totales de la lista" — con el gate de nivel 2 (3 de 6) esa cuenta habría mostrado un progreso incorrecto
  (barra nunca llega a 100% aunque el gate ya esté cumplido). Se añadió `edificiosConstruidos` a la
  interfaz y se corrigió el cálculo en `asentamientoQuery.ts`/`main.ts`.
- **`snapshot_baseline.test.ts` actualizado conscientemente**: quitar piedra de Almacén/minas cambia el flujo
  de materiales desde el tick 0 (más madera/trigo disponibles, piedra de la reserva inicial ya no se queda
  varada en 20) — mismo tipo de actualización consciente que ya tuvo Fase_0_5.
- Punto abierto #6 de §5 (curva de cupo para niveles 4-5) sigue sin resolver, tal como se decidió — no
  bloqueaba la implementación.
- **Deadlock real detectado por el usuario jugando, corregido**: el gate de nivel 2 pedía 50 artesanos además
  de los 3 edificios de extracción — pero Artesanos no aparece hasta el primer edificio de transformación
  activo (`engine/population.ts`), y esos edificios ahora exigen nivel 2 para construirse (§2). Sin artesanos
  no hay nivel 2; sin nivel 2 no hay transformación; sin transformación no hay artesanos — bloqueo total,
  ningún asentamiento podía pasar de nivel 1. **Corregido**: `NIVEL_ASENTAMIENTO.requisitos[2].artesanos`
  baja de 50 a 0 (pesants se mantiene en 200, no depende de ningún edificio de nivel 2). Verificado con una
  simulación aislada: el asentamiento de prueba ahora sube a nivel 2 en el tick 122 (antes se quedaba
  atascado indefinidamente).

## 7. Ajustes posteriores (sesión de balance, tras jugar con el sistema ya implementado)

Tres correcciones sobre el sistema de §2/§3, detectadas por el usuario jugando (no por simulación batch).
`186/186` tests en verde antes de estos cambios; verificado de nuevo tras cada uno (typecheck limpio, suite
completa en verde salvo los 3 timeouts preexistentes y no relacionados de `exportUnity.test.ts`).

### 7.1 `granja` se suma a la lista de "edificios de extracción" del gate de nivel 2

El gate 1→2 (§4 punto 1) fijó el set en los 6 tipos categorizados `extractorBase`/`supervivencia` en
`SCORE_BANDAS` (`cantera`, `lenera`, `mina`, `minaCobre`, `minaEstano`, `corral`), sin incluir `granja` —
Granja es "supervivencia" pero conceptualmente "producción primaria", no "extracción de mapa". En la
práctica, 4 de esos 6 tipos (`mina`/`minaCobre`/`minaEstano`/`corral`) dependen de tener un recurso raro
(oro/cobre/estaño/livestock) alcanzable en la zona — muchos asentamientos no tienen ninguno de los 4 cerca,
sin que sea un bug, solo generación de mapa. Un asentamiento con Cantera+Leñera+Granja (caso real reportado
por el usuario) se quedaba en 2 de 6 y nunca alcanzaba el `edificiosMinimo: 3`, sin ninguna vía de
recuperarse si no le tocó suerte con un nodo raro.

**Decisión del usuario**: sumar `granja` a la lista (`NIVEL_ASENTAMIENTO.requisitos[2].edificios`,
`constants.ts`) — pasa de "3 de 6" a "3 de 7". El bug de desempate de score entre extractores minerales
(`issues/extractores_minerales_nunca_se_construyen.md`) es un problema DISTINTO y ya se consideraba resuelto
aparte; este cambio no lo sustituye, solo evita que la variedad de recursos del mapa sea la única vía de
cumplir el gate.

### 7.2 Nuevo tope de Viviendas por nivel — `maximoViviendasPorNivel` (cierra el punto 8 de §4)

Implementado el `maximoViviendasPorNivel` que §4 punto 8 dejaba como pregunta abierta: el máximo útil de
Viviendas en `nivel` se deriva de la población que exige alcanzar el SIGUIENTE nivel
(`NIVEL_ASENTAMIENTO.requisitos[nivel+1]`) entre la capacidad de una Vivienda — pasado ese número, construir
más Viviendas no sirve para nada hasta subir de nivel. Aplicado tanto a la auto-construcción
(`evaluarNecesidades`) como a la adición manual (`anadirEdificioManualmente`), mismo patrón que
`alcanzoTopeDeAlmacenes`. Ver `engine/construction.ts` (`maximoViviendasPorNivel`, `alcanzoTopeDeViviendas`) y
los tests nuevos en `engine/__tests__/balance_granja_almacen.test.ts`.

Nota de implementación: a nivel 1 el espacio físico del trazado urbano (`radioPotencial`) ya limita cuántas
Viviendas caben mucho antes de llegar al tope de población (14) — este tope de población pasa a ser el freno
real recién en niveles con más radio/espacio disponible.

### 7.3 Deadlock introducido por 7.2 mismo, detectado por el usuario y corregido: pesants vs. artesanos

La primera versión de `maximoViviendasPorNivel` calculaba el tope SOLO con el requisito de `pesants` del
siguiente nivel, ignorando `artesanos`. Una Vivienda da 15 cupos de pesants pero solo 5 de artesanos
(proporción 3:1) — el gate 2→3 pide 500 pesants / 200 artesanos (proporción 2.5:1, MÁS artesanos por pesant
de lo que esa proporción de Vivienda regala). Con el tope calculado solo por pesants (500÷15=34 Viviendas),
la capacidad de artesanos quedaba en 34×5=**170**, por debajo de los 200 exigidos — un DEADLOCK real: el
propio tope de Vivienda impedía construir las Viviendas de más que hacían falta solo para alojar artesanos,
así que ningún asentamiento en nivel 2 podía subir nunca a nivel 3. El usuario lo detectó jugando: "cabe 510
pesants (ok) y 170 artesanos (200 no, nunca llega)".

**Corregido**: `maximoViviendasPorNivel` calcula el tope necesario para pesants y para artesanos por
separado y toma el MAYOR de los dos (`Math.max`). Con esto, nivel 2 pasa de 34 a **40** Viviendas de tope
(600 pesants / 200 artesanos — ambos cupos cubiertos). Revisados también los niveles 3 (80 Viviendas, cubre
1200 pesants / 400 artesanos) y 4 (160 Viviendas, cubre 2400 pesants / 800 artesanos): ninguno queda en
deadlock con la fórmula corregida. Se revisó también `techoPoblacion` (techo total de habitantes por nivel)
como posible cuello de botella análogo — tiene margen de sobra sobre lo que pide cada gate siguiente en
todos los niveles, no es un problema hoy.

Se añadió un test de regresión (`balance_granja_almacen.test.ts`) que fija este invariante para TODOS los
niveles con requisito siguiente a la vez (el tope de Viviendas debe dar cupo suficiente para pesants Y
artesanos del gate al que apunta), para que una futura recalibración de cifras de `NIVEL_ASENTAMIENTO` no
pueda reintroducir el mismo deadlock sin que un test lo detecte primero.
