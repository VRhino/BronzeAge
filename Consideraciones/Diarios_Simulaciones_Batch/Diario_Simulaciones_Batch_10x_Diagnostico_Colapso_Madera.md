# Diario de descubrimientos — 10 mundos × 100 Facciones: diagnóstico de por qué mueren los asentamientos de nivel 1

**Fecha:** 2026-08-18
**Qué es esto:** el usuario pidió específicamente diagnosticar por qué colapsan los asentamientos de nivel 1
"si tienen madera" — aprovechando el registro de razón de colapso agregado esta misma sesión a
`avanzarMantenimiento` (`src/engine/mantenimiento.ts`). En vez de una corrida, se corrieron **10 mundos
independientes** (mismas condiciones que
[Diario_Simulaciones_Batch_100_Facciones_Fase06_GateNivel2_SinArtesanos.md](Diario_Simulaciones_Batch_100_Facciones_Fase06_GateNivel2_SinArtesanos.md),
solo con semilla distinta cada uno) para tener una muestra grande de eventos de colapso y buscar el patrón
común. **El resultado es un hallazgo nítido con una causa raíz identificable en el código, sin ambigüedad.**

## Metodología

- 10 mundos secuenciales (no procesos paralelos reales — 10 "tandas" del mismo experimento con semillas
  20260818 a 20260827), cada uno: 100 Facciones, 1 asentamiento cada una, 5 jugadores fundadores,
  `simulaciones-batch/npcGobernanza.ts` reutilizado sin cambios, 3000 ticks.
- Por cada evento `"cae en ruinas"` capturado en `resultado.eventos` de `avanzarSimulacion`, se guardó el
  estado del asentamiento **del tick inmediatamente anterior** (edificios activos, almacén, población) junto
  con la razón detallada que ahora incluye el mensaje (recurso(s) que faltaron y cuánto había vs. cuánto hacía
  falta, edad del asentamiento al morir).
- Motor y `npcGobernanza.ts` sin tocar. **0 excepciones** en los 10 mundos × 3000 ticks. Escenario temporal
  descartado tras extraer este diario.

## Resumen ejecutivo

| Métrica | Valor |
|---|---|
| Total de colapsos registrados (10 mundos) | 1164 |
| De esos, en asentamientos de **nivel 1** | 1152 (99.0%) |
| Con **Leñera activa** en el momento de morir | 995 (86.4%) |
| Con **0 madera** en el almacén en el momento de morir | **1152 / 1152 (100%)** |
| Con madera > 0 en el momento de morir | **0** |
| Edad mediana al morir (nivel 1) | 77 ticks |

**El patrón es absoluto: todo asentamiento de nivel 1 que cae en ruinas lo hace con el almacén de madera
EXACTAMENTE en 0** — no "casi sin madera", cero. Y en el 86% de los casos, esto ocurre **con una Leñera ya
activa** (así que no es simplemente "nunca tuvo forma de producir madera"). La edad mediana de muerte (77
ticks) coincide con `MANTENIMIENTO.graciaTicks` (60) más el tiempo que tarda el medidor en degradarse a 0
bajo déficit total (`degradacionPorDeficitTotal: 10`, `medidorInicial: 100` → 10 ticks en déficit del 100%).

## La causa raíz: el reclutamiento no respeta la reserva de Mantenimiento, y agota el stock inicial de un solo golpe

El motor SÍ tiene un mecanismo para proteger la madera que Mantenimiento va a necesitar
(`reservaDinamicaConstruccion`, `puedeIniciarConstruccion` en `engine/construction.ts`) — pero **ese
mecanismo solo se aplica a la auto-construcción**, no al reclutamiento de tropas:

- `reclutarTropa` (`engine/tropas.ts:29-66`) solo comprueba `tieneRecursos(asentamiento.almacen, costoTotal)`
  — afordabilidad bruta, sin ningún concepto de reserva mínima.
- `milicia_lanceros` (`TROPAS_RECLUTABLES`, `constants.ts:900`): `costoEquipo: { madera: 2 }`,
  `unidadesPorDefecto: 25`. El PRIMER reclutamiento de un residente (escuadrón nuevo, sin veteranía previa)
  paga por las 25 unidades de una vez: **25 × 2 = 50 madera**.
- `FUNDACION.materialesIniciales.madera = 50` — el stock inicial con el que nace TODO asentamiento.

**50 de costo, 50 de stock inicial: el primer reclutamiento de un solo residente agota EXACTAMENTE el 100%
de la madera con la que nace el asentamiento**, sin dejar nada — ni para la propia auto-construcción
(Vivienda/Granja/Almacén, que si SÍ respetan la reserva pero ya no tienen margen para respetar nada porque el
almacén ya está en 0), ni para Mantenimiento cuando termine la gracia.

`POBLACION.pesants.inicial = 20` con `tasaCrecimientoBase: 0.12` — Pesants cruza el umbral de 25 (mínimo
para poder reclutar `milicia_lanceros`) en aproximadamente 2 ticks (20 × 1.12² ≈ 25.1). Y
`npcGobernanza.ts` (`reclutarParaTodos`) intenta reclutar para **los 5 residentes cada tick**, en cuanto
puedan pagarlo — así que este drain ocurre casi siempre dentro de los primeros ~10 ticks de vida del
asentamiento, muy por delante de que Mantenimiento empiece a cobrar (tick 60). El asentamiento pasa el resto
de la gracia con el almacén de madera en 0 (Leñera, si llega a construirse, apenas repone lo justo para que
la auto-construcción siga intentando otros proyectos, sin margen de sobra) y, en cuanto Mantenimiento
reclama sus ~3 madera/tick en tick 60, no hay nada que pagar — el medidor degrada a 0 en ~10-20 ticks más,
coincidiendo exactamente con la edad mediana de muerte observada (77).

## ¿Es esto un bug del motor?

No de forma inequívoca — es una interacción entre dos piezas, ninguna "rota" por sí sola:

1. `reclutarTropa` no aplicar la reserva de Mantenimiento podría ser una decisión de diseño deliberada
   (reclutar es una acción de JUGADOR, no auto-construcción automática — el motor no le impide a un jugador
   real vaciar su propio almacén si así lo decide). No se puede afirmar que sea un descuido sin que el usuario
   lo confirme.
2. Quien SÍ actúa sin ningún criterio de prudencia es `simulaciones-batch/npcGobernanza.ts` — nuestro propio
   script de prueba (fuera del motor), que recluta para TODOS los residentes en CUANTO pueden pagarlo, sin
   dejar margen para Mantenimiento. Un jugador humano probablemente no vaciaría su almacén de madera por
   completo en el tick 2 de vida de su asentamiento.

Lo que si es un hecho verificable en el código, no una interpretación: **la reserva de Mantenimiento protege
a la auto-construcción pero no al reclutamiento**, y esa asimetría es la que permite que el stock inicial se
agote de un solo golpe.

## Nota metodológica

- No se tocó el motor ni `npcGobernanza.ts` para este batch — solo se instrumentó el escenario temporal
  (ya descartado) para capturar el estado de cada asentamiento un tick antes de morir.
- No se aisló cuantitativamente el peso exacto del reclutamiento frente a otros factores (auto-construcción
  compitiendo por el mismo recurso, por ejemplo) — la evidencia (100% de las muertes con madera en 0, edad de
  muerte pegada al fin de la gracia, magnitud exacta 50=50) es fuerte pero circunstancial, no una prueba
  A/B controlada.
- 12 de los 1164 colapsos registrados fueron de asentamientos que ya habían llegado a nivel 2 (fuera del
  alcance de esta pregunta, que era específicamente sobre nivel 1) — no se investigaron por separado.

## Sugerencia de siguiente paso

Dos palancas posibles para confirmar y, si el usuario lo decide, corregir — ninguna implementada en este
batch:

1. **Batch de control** (sin tocar el motor): repetir este mismo escenario con `reclutarParaTodos`
   desactivado en la config de `npcGobernanza.ts` (o con un límite de "no reclutar en los primeros N ticks"),
   todo lo demás igual, y comparar la tasa de colapso y la edad de muerte contra este batch — confirmaría
   cuantitativamente cuánto pesa este factor.
2. **Si el usuario decide que es un gap de diseño real**: dos formas de cerrarlo, cualquiera de las dos SÍ
   tocaría el motor — (a) que `reclutarTropa` respete algún tipo de reserva mínima de Mantenimiento (similar
   a `puedeIniciarConstruccion`), o (b) bajar el costo/tamaño del primer lote de `milicia_lanceros` para que
   no pueda, por sí solo, agotar el stock inicial completo.
