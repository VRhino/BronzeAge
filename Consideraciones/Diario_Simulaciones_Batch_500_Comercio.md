# Diario de descubrimientos — batch de 500 simulaciones con comercio activo (900 ticks)

**Fecha:** 2026-08-08
**Qué es esto:** continuación de [Diario_Simulaciones_Batch_100.md](Diario_Simulaciones_Batch_100.md), mismo
método (script de prueba externo al repo, importa `src/engine/*` tal cual está, sin tocar ni una línea de
código), pero con dos diferencias: **500 runs** en vez de 100, y esta vez cada run ejercita también
**Trueque y Órdenes de Mercado** — no solo el bucle autónomo pasivo — para generar movimiento real de
recursos entre asentamientos.

## Metodología (lo que cambia respecto al batch de 100)

Cada 40 ticks, un agente de comercio simple actúa sobre los 3 asentamientos del run:
- **Mercado abierto**: cada asentamiento vende el excedente de cualquier recurso por encima de 150
  unidades (deja 100), y si un recurso baja de 15 unidades e tiene más de 20 de oro, intenta comprar 30.
- **Trueque bilateral**: para cada par de asentamientos del run, si cada uno tiene un recurso distinto con
  más de 80 unidades en stock, se les propone intercambiar 50×50 de esos dos recursos.

Es una heurística deliberadamente simple (no un agente económico "inteligente") — el objetivo era generar
actividad real para observar cómo se comporta el motor de comercio ya implementado bajo carga, no optimizar
las decisiones de compra/venta. Esto importa para interpretar el hallazgo #2 más abajo.

Fundación/cargos/políticas: igual que el batch anterior (1 asentamiento por Facción, 5 jugadores con los 5
cargos, políticas aleatorias por cargo). Nuevo: también se toma una foto de cada Facción (nivel, reputación)
cada 30 ticks, y el precio de referencia final de cada recurso comerciable.

## Resumen ejecutivo

| # | Hallazgo | Tipo |
|---|---|---|
| 1 | El comercio SÍ genera movimiento real y a gran escala: 112.700 unidades por caravana de trueque + 45.126 por mercado, 24.051 de oro en comisiones | ✅ Confirma que el sistema funciona |
| 2 | Trueque se completa 14× más a menudo que Mercado (57.1% vs 4.0%) — Mercado no tiene expiración y acumula 39.574 órdenes sin límite | 🟠 Alto |
| 3 | Estaño y Livestock quedan con el precio en el TECHO de escasez (×3) en el 100% de los 500 runs, sin una sola excepción | 🟠 Alto |
| 4 | El nivel de Facción está totalmente desacoplado del nivel de sus asentamientos — 37 Facciones llegan a nivel ≥5 con TODOS sus asentamientos (vivos o colapsados) atascados para siempre en nivel 1 | 🟡 Medio |
| 5 | La reputación de Facción decae tan rápido (−0.2/tick) que una foto en un tick fijo la subestima sistemáticamente | 🟡 Medio (nota metodológica) |
| 6 | Colapso general 66.7% (1001/1500), consistente con el batch de 100 tras el fix de vivienda — el comercio, tal como se probó, no cambia la dinámica de colapso | ⚪ Confirma lo ya sabido |
| 7 | Cero excepciones del motor en 500 runs × 900 ticks (con miles de trueques/órdenes de por medio) | ✅ Positivo |

## Hallazgo #1 — El comercio funciona end-to-end y mueve volumen real

Con el agente de comercio activo, en los 500 runs se registraron:

| Métrica | Total | Por run (media) |
|---|---|---|
| Trueques propuestos | 1.973 | ~3.9 |
| Trueques cumplidos | 1.127 (57.1%) | ~2.3 |
| Trueques expirados | 814 | ~1.6 |
| Caravanas comerciales llegadas | 2.256 | ~4.5 |
| Volumen movido por caravana | 112.700 unidades | ~225 |
| Volumen movido por Mercado | 45.126 unidades | ~90 |
| Oro generado por comisiones | 24.051 | ~48 |

Confirma que `proponerTrueque`, `avanzarComercio` (caravanas + entrega + comisión + reputación) y
`avanzarMercado` (clearing de órdenes) funcionan correctamente de punta a punta a esta escala, sin ningún
error del motor (ver hallazgo #7).

## Hallazgo #2 — Mercado casi no completa órdenes; sin expiración, se acumulan sin límite

De **39.574 órdenes de Mercado colocadas**, solo **1.580 llegaron a `cumplida` (4.0%)**. Trueque, en
cambio, completa el **57.1%**. Dos causas distintas, ambas reales:

1. **Parcial, del agente de prueba**: el umbral de compra (>20 de oro) no tiene relación con el coste real
   de lo que intenta comprar — comprar 30 unidades al precio de referencia final medio cuesta ~107 oro
   (piedra), ~269 (cobre) o ~540 (estaño); con solo 20 de oro exigidos como mínimo, la mayoría de las
   compras se quedan cortas de oro y la orden nunca llega a `cantidadCumplida >= cantidad`. Esto es un
   defecto del agente de prueba, no del motor.
2. **Estructural, del motor mismo — esta parte sí es un hallazgo real**: `OrdenMercado` (a diferencia de
   `AcuerdoTrueque`, que tiene `expiraEnTick`) **no tiene ningún mecanismo de expiración o cancelación**.
   Una orden que no se completa nunca se limpia — se queda en el array `ordenes` para siempre, activa,
   compitiendo por clearing contra las nuevas. En una partida sostenida durante mucho tiempo, con jugadores
   colocando órdenes que no siempre se llenan del todo (falta de liquidez del comprador, sin vendedor al
   precio pedido, etc.), este array solo puede CRECER — nunca se depura solo. En este batch, con apenas 900
   ticks por run, ya se acumularon ~79 órdenes sin resolver por run en promedio.

## Hallazgo #3 — Estaño y Livestock: precio pegado al techo de escasez en el 100% de los runs

`calcularPrecioReferencia` multiplica el precio base por un factor de escasez acotado entre 0.4× y 3×. En
los 500 runs, sin una sola excepción:
- **Estaño**: precio final exactamente **18.000** (6 base × 3.0, el techo) en los 500/500 runs.
- **Livestock**: precio final exactamente **6.000** (2 base × 3.0, el techo) en los 500/500 runs.

El resto de recursos sí varía entre runs (madera 0.4-3.0, piedra 0.82-3.6, trigo 0.6-4.5, cobre 3.97-9.0).
Que el precio nunca baje del techo máximo significa que el stock GLOBAL acumulado de estaño y livestock
(sumado entre los 3 asentamientos del run) se queda siempre muy por debajo de `stockObjetivoGlobal/3` (≈167
unidades) — en absolutamente ningún run, de 500, la oferta agregada de estos dos recursos llegó ni de lejos
a un nivel "saludable". Para Estaño es coherente con el diseño (cuello de botella del bronce, nodos raros,
comentado explícitamente en `constants.ts`); para **Livestock es más llamativo** — no está señalado en
ningún lado como recurso deliberadamente escaso, y su extractor (Corral) comparte el mismo problema de
"necesita un nodo dentro de una zona que a menudo no crece lo suficiente" que ya se documentó para
Cantera/minas en el diario anterior. Candidato a revisar en una futura sesión de calibración.

## Hallazgo #4 — Nivel de Facción totalmente desacoplado del nivel de sus asentamientos

`calcularNivelFaccion` (Doc 1.7) se basa en nº de asentamientos + población NPC total, sin relación con el
modelo de gates que rige el nivel de CADA asentamiento (Doc 4.5, población + edificios específicos). En
este batch, **37 Facciones alcanzan nivel de Facción ≥5 (de un máximo de 10) mientras TODOS sus
asentamientos —vivos o ya colapsados— se quedaron siempre en nivel de asentamiento 1**, es decir, sin
Armería/Curtidería/Fundición activas simultáneamente en ningún momento de la partida. Narrativamente es
raro: una Facción de "nivel 5+" cuyo único asentamiento nunca pasó del tier fundacional. No es un bug — los
dos sistemas de nivel están documentados como independientes (Doc 1.7 vs Doc 4.5) — pero vale la pena
tenerlo en cuenta si en algún momento se usa el nivel de Facción como proxy narrativo de "qué tan
desarrollada" está una Facción.

## Hallazgo #5 — Reputación de Facción: la foto en tick 900 subestima la actividad real

Con 1.127 trueques cumplidos (cada uno suma +5 de reputación a ambas Facciones, `bonusTruequeCumplido`) y
814 expirados (−8 al lado incumplidor, `penalizacionTruequeIncumplido`), cabría esperar reputaciones finales
notablemente distintas de 0. En cambio, a tick 900: **1.468 de 1.500 Facciones (97.9%) tienen reputación
EXACTAMENTE 0**, y solo 31 terminan positivas (hasta +48.8 en el mejor caso). La razón es
`REPUTACION.decaimientoPorTick: 0.2` — un evento de +5 se diluye de vuelta a ~0 en apenas 25 ticks sin un
evento nuevo que lo refuerce. Como los trueques de este batch se espacian cada ~40 ticks (el intervalo del
agente) y no todos se completan, es habitual que el último evento de reputación de una Facción haya ocurrido
mucho antes del tick 900 y ya se haya disuelto por completo para cuando se tomó la foto final. Esto no es un
problema del sistema de reputación — es un recordatorio de que medirlo en un único tick fijo, en vez de en
una ventana continua alrededor de los eventos, subestima sistemáticamente cuánta actividad hubo.

## Hallazgo #6 — El colapso general no cambia con comercio activo

66.7% de colapso (1001/1500), muy en línea con el 63.3% del batch de 100 tras el fix de vivienda (ver
`Diario_Simulaciones_Batch_100.md`). Consistente con lo ya documentado: el colapso sigue estando dominado
por la distancia al bosque más cercano al fundar y, en menor medida, por el coste de Mantenimiento adicional
al subir de nivel — ninguno de los dos factores lo alivia el comercio tal como se probó aquí (24.2% de los
runs, además, nunca tuvieron ni un solo par de asentamientos con superávits complementarios para proponer
trueque). Nivel al colapsar: 820 en nivel 1, 173 en nivel 2, 8 en nivel 3 — confirma otra vez, a mayor
escala, que nivel 2/3 sí son alcanzables tras el fix de vivienda (ver diario anterior), solo que
insuficientes para evitar el colapso posterior.

## Hallazgo #7 — Motor robusto también bajo esta carga

Cero excepciones no capturadas en 500 runs × 900 ticks (450.000 avances de tick), con miles de trueques y
decenas de miles de órdenes de mercado de por medio. Las únicas anomalías detectadas (1.770 en total) son
ruido de punto flotante (`~-4.4e-16`) en `oro` y `cuero` al descontarlos hasta casi exactamente 0 — mismo
tipo cosmético ya señalado en el diario anterior, ahora también visible en `oro` porque el comercio genera
muchas más operaciones de descuento sobre ese recurso.

## Nota metodológica

Igual que en el batch anterior: esto solo prueba el motor tal cual está — ningún archivo de `src/` se tocó
para esta prueba. El agente de comercio es una heurística simple para generar actividad, no un modelo de
qué haría un jugador real; el hallazgo #2 (baja tasa de clearing de Mercado) mezcla una limitación del
agente de prueba con un hallazgo estructural real (Mercado no expira) — ambos se explican por separado en
esa sección.
