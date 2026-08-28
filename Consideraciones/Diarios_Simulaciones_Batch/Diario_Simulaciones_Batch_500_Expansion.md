# Diario de descubrimientos — batch de 500 simulaciones con Caravana de Fundación (900 ticks)

**Fecha:** 2026-08-08
**Qué es esto:** tercera vuelta de la misma metodología de
[Diario_Simulaciones_Batch_100.md](Diario_Simulaciones_Batch_100.md) y
[Diario_Simulaciones_Batch_500_Comercio.md](Diario_Simulaciones_Batch_500_Comercio.md) — script de prueba
externo al repo, importa `src/engine/*` tal cual está (incluida la Caravana de Fundación, Doc 1.8,
implementada esta misma sesión), sin tocar ni una línea de código. 500 runs, comercio activo igual que el
batch anterior, y ahora además un **agente de expansión**: cada asentamiento que llega a nivel 2 intenta
lanzar una Caravana de Fundación hacia un punto aleatorio a 150-400 unidades de distancia, cada 40 ticks.

## Resumen ejecutivo

| # | Hallazgo | Severidad |
|---|---|---|
| 1 | El mecanismo funciona de punta a punta a escala, sin ningún error del motor — 45 asentamientos nuevos fundados por expansión en 500 runs | ✅ Confirma que el código funciona |
| 2 | La adopción real es baja: solo 140/500 runs (28%) llegan siquiera a intentar expandirse, y de esos, solo 59/432 intentos (13.7%) tienen recursos suficientes para lanzar la caravana | 🟠 Alto (síntoma del cuello de botella ya conocido) |
| 3 | Los asentamientos fundados por expansión colapsan MENOS que los originales (57.8% vs 66.8%) | 🟡 Curioso, muestra pequeña |
| 4 | El cupo del Cap de Fundación se reserva y respeta correctamente — nunca se detectó doble-reserva ni fundación por encima del cap | ✅ Positivo |

## Hallazgo #1 — El mecanismo funciona end-to-end, cero errores

En 500 runs × 900 ticks (con comercio activo de por medio, igual que el batch anterior):

| Métrica | Total |
|---|---|
| Intentos de expansión (asentamiento en nivel ≥2, evaluado cada 40 ticks) | 432 |
| Caravanas lanzadas con éxito | 59 |
| Asentamientos nuevos fundados al llegar | 45 |
| Tasa de éxito lanzamiento → fundación | 76.3% |
| Runs con al menos 1 fundación por expansión | 38/500 (7.6%) |
| Errores/excepciones del motor | 0 |

El 23.7% de diferencia entre "lanzadas" (59) y "fundadas" (45) no se investigó línea por línea — puede
incluir tanto caravanas que seguían en tránsito al terminar el run (900 ticks) como el caso de borde ya
aceptado en el diseño (nivel de Facción bajando en tránsito). No se detectó ninguna caravana fundando por
encima del cap ni gastando recursos sin descontarlos — el flujo completo (gate de nivel, reserva de cupo,
coste, viaje, fundación con los ciudadanos correctos) se comporta como se diseñó.

## Hallazgo #2 — La adopción real es baja: el cuello de botella de siempre

Solo **140 de 500 runs (28%)** tuvieron algún asentamiento en nivel ≥2 en algún momento — coherente con los
dos diarios anteriores, que ya documentaron que nivel 2 es difícil de alcanzar (cuello de botella de
Artesanos/Vivienda/cola de construcción). De los 432 intentos de expansión evaluados, el rechazo dominante
fue:

| Causa de rechazo | Ocurrencias | % |
|---|---|---|
| Recursos insuficientes | 256 | 59.3% |
| Destino fuera del mapa (heurística del agente, no del motor) | 114 | 26.4% |
| Destino ocupado (zona de otro asentamiento) | 3 | 0.7% |

"Recursos insuficientes" domina: un asentamiento que acaba de arañar el umbral de nivel 2 rara vez tiene,
además, el superávit necesario para pagar el coste completo de la Caravana de Fundación (materiales
iniciales + 4 edificios de arranque + 50 madera extra) sin descuidar su propia economía. Esto no es un bug
— es la fricción que el propio diseño de la Caravana de Fundación busca (Doc 1.8: "evitando fundación en
cadena sin fricción") — pero combinado con el cuello de botella de nivel 2 ya conocido, el resultado es que
la expansión real solo ocurre en **7.6% de las partidas** de este batch. Distribución de asentamientos
simultáneos por Facción alcanzados en cualquier momento del run: 462 runs se quedan siempre en 1, 33 llegan
a 2, y 5 llegan a 3.

## Hallazgo #3 — Los asentamientos fundados por expansión sobreviven algo mejor (muestra pequeña)

De los 45 asentamientos fundados por expansión, **26 colapsaron (57.8%)**, frente al **66.8%** de los 1500
asentamientos originales. La diferencia podría deberse a que solo las Facciones ya relativamente exitosas
(con suficiente margen de recursos) llegan a expandirse, así que no es necesariamente una ventaja mecánica
del método de fundación en sí — con solo 45 casos la muestra es demasiado pequeña para afirmar una causa,
se deja anotado como observación, no como conclusión.

## Hallazgo #4 — El cupo del Cap de Fundación se respeta correctamente

Ninguna de las 500 simulaciones mostró una Facción con más asentamientos simultáneos de los que su
`calcularCapFundacion(nivel)` permitiría — la reserva de cupo al LANZAR (no al llegar, la regla que definiste
explícitamente) funcionó de forma consistente en los 59 lanzamientos reales del batch.

## Nota metodológica

Igual que los diarios anteriores: el agente de expansión es una heurística simple (destino aleatorio a
150-400 unidades, hasta 3 ciudadanos por caravana) para generar actividad y medir el mecanismo bajo carga —
no un jugador optimizando dónde expandirse. Ningún archivo de `src/` se tocó para esta prueba.
