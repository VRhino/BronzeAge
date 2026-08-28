# Diario de descubrimientos — batch de 500 simulaciones enfocado en productos de transformación (900 ticks)

**Fecha:** 2026-08-08
**Qué es esto:** cuarta vuelta de la misma metodología de
[Diario_Simulaciones_Batch_100.md](Diario_Simulaciones_Batch_100.md),
[Diario_Simulaciones_Batch_500_Comercio.md](Diario_Simulaciones_Batch_500_Comercio.md) y
[Diario_Simulaciones_Batch_500_Expansion.md](Diario_Simulaciones_Batch_500_Expansion.md) — script de prueba
externo al repo, importa `src/engine/*` tal cual está, sin tocar ni una línea de código. 500 runs, 900 ticks,
mismo agente de comercio simple de los batches anteriores (Mercado + Trueque cada 40 ticks), pero esta vez la
instrumentación se centra en una sola pregunta: **¿se generan de verdad los productos de las cadenas de
crafting de Fundición/Curtiduría/Armería (Doc 4.2.1), y hasta dónde llegan?**

Diferencia deliberada de metodología respecto al batch de Comercio: el agente de trueque/mercado esta vez
**solo opera sobre los 6 recursos básicos** (madera, piedra, trigo, cobre, estaño, livestock) — los 12
productos de transformación (lingotes/cuero/armas/armaduras) se dejan fuera del comercio a propósito, para
que cualquier stock observado sea 100% producción local vía receta, no una importación por caravana.

## Metodología

- 500 runs independientes (mismo patrón que los batches anteriores: 3 asentamientos por run —
  Micenas/Troya/Ugarit —, 5 jugadores fundadores con los 5 cargos, políticas aleatorias por cargo). 1500
  asentamientos fundados en total.
- Cada 30 ticks se toma una foto de cada asentamiento vivo: nivel, población, `ratioManoObraArtesanos`,
  estado/nivel interno de Fundición/Curtiduría/Armería, y el stock de los 12 productos de transformación
  (`lingoteCobre`, `lingoteEstano`, `lingoteBronce`, `cuero`, `cueroCurtido`, `cueroCalidad`, `armaCobre`,
  `armaBronce`, `armaBronceCalidad`, `armaduraBasica`, `armaduraIntermedia`, `armaduraBronce`).
- El log de eventos del motor (`"X completado."` / `"X mejora a nivel interno N."`) se parsea en cada tick
  para saber el tick exacto en que cada edificio se activa y cuándo sube de nivel interno — más preciso que
  el muestreo cada 30 ticks para ese dato concreto.
- Motor sin tocar: el script solo importa `engine/*` — no se modificó ningún archivo fuente para esta prueba.

## Resumen ejecutivo

| # | Hallazgo | Severidad |
|---|---|---|
| 1 | Los 3 edificios SÍ producen — la cadena de crafting funciona end-to-end, sin errores del motor | ✅ Confirma que el código funciona |
| 2 | Pero la adopción real es baja: solo 13.5%-19.5% de los 1500 asentamientos llegan a tener el edificio activo en 900 ticks | 🟠 Alto (mismo cuello de botella ya documentado en batches anteriores) |
| 3 | De los que SÍ tienen el edificio activo, la receta de **nivel 1** solo llega a generar stock observable en el 12%-28% de los casos — el resto se queda en 0 durante todo el run | 🔴 Crítico |
| 4 | De los que llegan a **nivel interno 2** (37.8%-46.8% de los activos), las recetas nuevas que desbloquea casi nunca producen nada: 1.1%-13.5% según receta, y 0% para las que necesitan DOS insumos intermedios a la vez (Bronce) | 🔴 Crítico |
| 5 | Nivel interno 3 es, a efectos prácticos, inalcanzable en 900 ticks: 1 sola Curtidería de 1500 lo consigue, ninguna Armería | 🟡 Medio (consistente con el cuello de botella de nivel de asentamiento ya conocido) |
| 6 | La mano de obra de Artesanos NO es el cuello de botella una vez el edificio ya está activo — el ratio llega a 1.0 con margen. El bloqueo real está en la ESCASEZ de insumos, sobre todo estaño | 🟢 Descarta una hipótesis obvia |
| 7 | Cero errores/excepciones del motor en 500 runs × 900 ticks | ✅ Positivo |

## Hallazgo #1 y #2 — La cadena funciona, pero pocos asentamientos llegan a construirla

De los 1500 asentamientos fundados:

| Edificio | Activo alguna vez | Nivel interno 1 (al final) | Nivel interno 2 | Nivel interno 3 |
|---|---|---|---|---|
| Fundición | 203 (13.5%) | 108 | 95 (46.8% de los activos) | 0 (tope del catálogo es nivel 2) |
| Curtiduría | 292 (19.5%) | 166 | 125 (42.8%) | 1 (0.3%) |
| Armería | 238 (15.9%) | 148 | 90 (37.8%) | 0 (0.0%) |

Ticks medianos hasta la primera activación: Curtiduría 62.5, Armería 67, Fundición 73 — bastante antes de la
mitad del run, así que el problema de adopción no es de timing sino de cuántos asentamientos llegan a tener
los recursos/sitio para construirlas en absoluto (mismo patrón "cuello de botella temprano" que
`Diario_Simulaciones_Batch_100.md` ya documentó para el nivel de asentamiento en general — Fundición/
Curtiduría/Armería son justo los 3 edificios que gatean el ascenso a nivel 2).

## Hallazgo #3 — La receta de nivel 1 arranca, pero se queda en 0 la mayoría de las veces

De los asentamientos que SÍ tienen el edificio activo, cuántos llegaron a acumular alguna vez stock > 0 del
producto de nivel 1:

| Producto | Edificio | Con edificio activo | Con stock > 0 alguna vez | % |
|---|---|---|---|---|
| `lingoteCobre` | Fundición | 203 | 47 | 23.2% |
| `cuero` | Curtiduría | 292 | 35 | 12.0% |
| `armaCobre` | Armería | 238 | 65 | 27.3% |
| `armaduraBasica` | Armería | 238 | 67 | 28.2% |

En el 72%-88% de los casos restantes el edificio está `activo` (ya pasó por la cola, ya se pagó, ya tiene
`nivelInterno: 1`) pero el producto nunca llega a acumular stock — la explicación más probable, coherente con
el hallazgo #6 más abajo, es que la propia materia prima (cobre/livestock) es escasa o está lejos del sitio
donde cayó el edificio (colocación automática por `sitioConcentrico`, sin garantía de cercanía a un nodo), así
que la receta corre a una fracción mínima de `produccionBase` — nunca en 0 exacto tick a tick, pero por debajo
del umbral de redondeo/consumo que le permitiría notarse en una foto cada 30 ticks antes de que downstream
(Armería consumiendo `lingoteCobre`/`cuero` en el mismo asentamiento) se lo coma otra vez.

## Hallazgo #4 — Nivel interno 2 se alcanza, pero sus recetas nuevas casi nunca producen nada

Este es el hallazgo más claro del batch. Tomando como base SOLO los edificios que ya llegaron a nivel interno
2 (no todos los "activos" — muchos siguen en nivel 1 y estas recetas ni existen para ellos):

| Producto (nivel 2) | Insumos | Edificios en nivel ≥2 | Con stock > 0 alguna vez | % |
|---|---|---|---|---|
| `lingoteEstano` | estaño | 95 | 9 | 9.5% |
| `lingoteBronce` | lingoteCobre + lingoteEstano | 95 | 2 | 2.1% |
| `cueroCurtido` | cuero | 126 | 17 | 13.5% |
| `armaBronce` | lingoteBronce + madera | 90 | 2 | 2.2% |
| `armaduraIntermedia` | lingoteCobre + cueroCurtido | 90 | 1 | 1.1% |

El patrón es consistente: una receta de nivel 2 que depende de **un solo insumo intermedio** (`lingoteEstano`
sobre estaño en bruto, `cueroCurtido` sobre `cuero`) ya arranca produce en el 9.5%-13.5% de los casos — bajo,
pero no cero. En cambio, cualquier receta que necesita **dos insumos intermedios a la vez** (`lingoteBronce` =
`lingoteCobre` + `lingoteEstano`; `armaBronce` = `lingoteBronce` + madera; `armaduraIntermedia` = `lingoteCobre`
+ `cueroCurtido`) cae a 1.1%-2.2% — el producto de un cuello de botella se convierte en el insumo de otro, y la
probabilidad de que ambos coincidan con stock > 0 en el mismo asentamiento en el mismo momento es baja.

`lingoteEstano` es el caso más claro de causa raíz: su receta pide `produccionBase: 3` a cambio de
`consumePorUnidad: { estano: 5 }` — o sea, **15 estaño/tick** para correr a régimen completo, mientras que una
sola Mina de Estaño produce `produccionBaseEstano: 1.5`/tick (`EDIFICIO_CATALOGO.minaEstano`,
`src/constants.ts:167`) y el estaño es, de los 4 minerales, el más raro del mapa (`RECURSO_RAREZA.raro`,
`cantidadBase: 6` nodos en todo el mundo de 1000×1000, `src/constants.ts:17`). Con como mucho 1-2 Minas de
Estaño reales por asentamiento (el resto del batch de Comercio ya documentó que estaño queda pegado al techo
de precio en el 100% de los runs por esta misma escasez — ver `Diario_Simulaciones_Batch_500_Comercio.md`
hallazgo #3), la receta de `lingoteEstano` corre muy por debajo de su `produccionBase` casi siempre, y
`lingoteBronce` — que necesita ese trickle de estaño MÁS un trickle de cobre a la vez — casi nunca ve ambos
insumos disponibles simultáneamente.

## Hallazgo #5 — Nivel interno 3 es inalcanzable en la práctica

Curtidería y Armería tienen recetas de nivel 3 en el catálogo (`cueroCalidad`, `armaBronceCalidad`,
`armaduraBronce`), pero de 1500 asentamientos solo **1 Curtidería** llegó alguna vez a nivel interno 3, y
**ninguna Armería**. Ninguno de los dos productos de nivel 3 se observó nunca con stock > 0. Es coherente con
el gate: nivel interno 3 exige nivel de asentamiento 3 (500 pesants + 200 artesanos + Carpintería/Barracón/
Galería de tiro activos, `NIVEL_ASENTAMIENTO.requisitos`, `src/constants.ts:597`), un umbral que los batches
anteriores ya mostraron que casi nadie alcanza en 900 ticks — no es un hallazgo nuevo sobre crafting en sí,
es el mismo cuello de botella de progresión de siempre propagándose a las recetas de tier superior.

## Hallazgo #6 — La mano de obra de Artesanos NO es el cuello de botella (descarta la hipótesis obvia)

Antes de correr el batch, la hipótesis más intuitiva era "los Artesanos crecen más lento que los Pesants
(`POBLACION.artesanos.tasaCrecimientoBase: 0.05` vs `0.12`, `src/constants.ts:104-109`) y cada edificio de
transformación pide 4-20 `trabajadoresRequeridos` — seguro que `ratioManoObraArtesanos` es el freno real".

Los datos NO lo confirman: en la foto final de todos los asentamientos con algún edificio de transformación
activo, `ratioManoObraArtesanos` está en 1.0 (cobertura completa) con amplio margen — el crecimiento
estocástico de Artesanos (`crecimientoEstocastico`, `src/engine/population.ts:13`) compone rápido una vez
aparece el primer Artesano (evento "Los primeros Artesanos se establecen..."), y para cuando un edificio llega
a nivel interno 2 el asentamiento ya cumplió el gate de 50 artesanos mínimo para subir a nivel de asentamiento
2 — sobra mano de obra para los 8-22 `trabajadoresRequeridos` que pide la transformación en ese punto. La
demanda de mano de obra escala con el nivel interno del edificio, pero la población de Artesanos escala más
rápido en términos absolutos una vez arranca.

Esto es un hallazgo negativo útil: descarta una hipótesis de rebalanceo (subir la tasa de crecimiento de
Artesanos) que no habría resuelto el problema real, que está aguas arriba, en la disponibilidad de materia
prima mineral (hallazgo #4) y en la tasa de adopción de los propios edificios (hallazgo #2).

## Hallazgo #7 — Cero errores del motor

500 runs × 900 ticks, con Fundición/Curtiduría/Armería subiendo de nivel interno 317 veces en total (95 + 132
+ 90) y produciendo activamente en las combinaciones descritas arriba, sin ninguna excepción del motor.
`avanzarRecetas`, `avanzarMejoras` y el resto de la cadena de `avanzarConstruccion` se comportan como se
diseñaron a esta escala.

## Nota de alcance

Igual que en los batches anteriores, esto es un diario de OBSERVACIONES sobre el estado ACTUAL del motor —
no se aplicó ningún cambio de código. Los porcentajes de "producto generado" son un límite inferior real
(basado en fotos cada 30 ticks, así que un pico de stock muy breve entre dos fotos podría no registrarse),
pero la tendencia — recetas de un solo insumo intermedio generan poco, recetas de dos insumos intermedios
casi nunca — es consistente en las 12 líneas de producto y no depende de ese margen de muestreo. Colapso
general del batch: 1069/1500 (71.3%), en línea con los batches anteriores tras el fix de vivienda.
