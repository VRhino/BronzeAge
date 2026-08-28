# Diario de descubrimientos — 100 Facciones × 1 asentamiento, NPC con reserva/gate/trueque de supervivencia

**Fecha:** 2026-08-18
**Qué es esto:** re-corrida EXACTA del escenario de
[Diario_Simulaciones_Batch_100_Facciones_Fase06_GateNivel2_SinArtesanos.md](Diario_Simulaciones_Batch_100_Facciones_Fase06_GateNivel2_SinArtesanos.md)
(mismo mapa/seed, 100 Facciones, 1 asentamiento cada una, 5 jugadores fundadores, 3000 ticks), esta vez con
el `simulaciones-batch/npcGobernanza.ts` rediseñado: (1) reserva 150 de madera vía `reservaManual` desde que
hay Tesorero, (2) no recluta si el almacén de madera no llega a 150, (3) trueque de SUPERVIVENCIA nuevo —
busca ayuda en CUALQUIER Facción cuando a un asentamiento no le va a alcanzar para pagar Mantenimiento,
pagando con lo que le sobre (oro como último recurso). **El resultado es mixto: mejora radicalmente la
supervivencia temprana, pero no evita el colapso a largo plazo, y tiene un costo lateral inesperado en el
progreso de nivel de asentamiento.**

## Metodología

- Mismo mundo/seed/escala que el batch de referencia — único cambio: el NPC. Motor sin tocar (confirmado por
  `git status` antes/después). 3000 ticks, foto cada 100. **0 excepciones.**
- Escenario temporal descartado tras extraer este diario.

## Resumen ejecutivo

| # | Hallazgo | Severidad/Tipo |
|---|---|---|
| 1 | **Supervivencia temprana muchísimo mejor**: 88 vivos en tick 100 (vs 30 antes, +193%) — el fix de reserva+gate de reclutamiento funciona exactamente como se esperaba | ✅ Éxito claro |
| 2 | **Pero el colapso a largo plazo casi no mejora**: 12 vivos a tick 3000 (vs 10 antes) — la ventaja temprana se erosiona con el tiempo; el colapso se estira, no se evita | 🟡 Mejora marginal en el resultado final |
| 3 | El trueque de supervivencia SÍ funciona de punta a punta (137 acuerdos con entrega real vía caravana) pero es un mecanismo MENOR frente al volumen propuesto: 7723 propuestas, solo 137 cumplidas (1.8%) | 🟡 Funciona, pero su impacto real es pequeño en este batch |
| 4 | **Gap de diseño real encontrado en el NPC**: un asentamiento puede acumular varios acuerdos activos simultáneos para el MISMO recurso con socios distintos — el chequeo de duplicado es por PAR, no por "¿ya tengo ayuda en camino?" — explica el volumen desproporcionado de propuestas | 🟠 Ineficiencia real, no afecta el motor |
| 5 | **Regresión inesperada**: NINGÚN asentamiento llegó a nivel 2 esta vez (vs 2 en el batch anterior) — Mercado (100 madera) + caravana (50 madera) compiten por el mismo excedente de madera que antes iba a edificios de extracción | 🔴 Costo lateral no anticipado |
| 6 | Nivel de Facción máximo y XP más bajos (nivel 8 / 989 XP vs nivel 10 / 7549 XP) — reclutamiento mucho más restringido (545 vs 2442) deja menos escuadrones para cazar bandidos (123 vs 302 campamentos destruidos) | 🟡 Trade-off coherente con el propio gate, pero real |
| 7 | Motor sin tocar, 0 excepciones en 3000 ticks pese a la carga extra (Mercado/caravana/trueque para 100 asentamientos cada tick) | ✅ Positivo |

## Hallazgo #1 y #2 — Mejor al principio, casi igual al final

| Tick | Vivos (batch anterior, sin reserva/gate) | Vivos (este batch, con reserva/gate/trueque) |
|---|---|---|
| 100 | 30 | **88** |
| 200 | 15 | 55 |
| 300 | 14 | 46 |
| 500 | 12 | 25 |
| 700 | 12 | 20 |
| 1000 | 11 | 19 |
| 1300 | 10 | 14 |
| 1900 | 10 | 12 |
| 3000 | 10 | **12** |

La diferencia en tick 100 es enorme — confirma sin ambigüedad que el diagnóstico anterior
(`Diario_Simulaciones_Batch_10x_Diagnostico_Colapso_Madera.md`: un solo reclutamiento agotaba el 100% de la
madera inicial) era la causa dominante del colapso MÁS TEMPRANO. Pero la curva no se estabiliza ahí: sigue
bajando de forma constante hasta tick ~1900, terminando en 12 — solo 2 asentamientos más que el batch
anterior. El fix pospone y suaviza el colapso, no lo resuelve: algo más (probablemente el costo de
Mantenimiento creciendo con la población, `factorPoblacion`, ver `engine/mantenimiento.ts`) sigue ganándole
la carrera a la mayoría de los asentamientos con el tiempo, solo que ahora tarda más en hacerlo.

## Hallazgo #3 y #4 — El trueque de supervivencia funciona, pero casi todo lo que propone nunca se cumple

`truequesSupervivenciaAcumulados` llega a **7723** propuestas totales, pero `acuerdosCumplidos` (con entrega
real completada por caravana) solo llega a **137** — un 1.8% de tasa de cumplimiento. `acuerdosActivos` llega
a un pico de **4495** en el propio tick 100 (¡45 pactos activos por tick de promedio, con solo 100
asentamientos existiendo!).

Revisando la lógica (`truequeDeSupervivencia`, `npcGobernanza.ts`): el chequeo de "no repetir" solo bloquea
un acuerdo IDÉNTICO entre el MISMO PAR de asentamientos para el MISMO recurso mientras siga 'activo' — pero
no comprueba si el asentamiento necesitado YA tiene sobre la mesa otros acuerdos pendientes para ese mismo
recurso con socios DISTINTOS. Como construir Mercado+caravana toma muchos ticks (necesita superar la reserva
de 150 + el costo de 100 del Mercado = 250 de margen), casi ningún acuerdo llega a entregarse a tiempo, así
que el asentamiento sigue "en riesgo" tick tras tick y termina acumulando pactos con varios socios sucesivos
para la misma necesidad — inflando el conteo sin que la mayoría aporte nada real. No es un bug que rompa
nada (0 excepciones, y el 1.8% que sí se cumple demuestra que el mecanismo funciona de punta a punta), pero
es una ineficiencia real del NPC que vale la pena corregir.

## Hallazgo #5 — Regresión inesperada: cero asentamientos llegaron a nivel 2 esta vez

El batch de referencia (sin este rediseño) sí logró que 2 asentamientos alcanzaran nivel 2, con
`extraccionPorTipoActivosTotal` final de `cantera:4, lenera:10, minaCobre:2, corral:2`. Este batch termina
con `nivelesAsentamiento` en 100% nivel 1 durante los 3000 ticks completos, y
`extraccionPorTipoActivosTotal` final de `cantera:0, lenera:12, minaCobre:0, minaEstano:0, corral:0` — la
Leñera (base, casi gratis) es la única que sigue apareciendo; el resto de edificios de extracción
prácticamente desaparecieron.

La explicación más plausible: el nuevo paso de infraestructura comercial (`asegurarInfraestructuraComercial`)
construye Mercado (100 madera) y una caravana comercial (50 madera) para CUALQUIER asentamiento con
Gobernador, en cuanto hay margen sobre la reserva de 150 — y ese mismo margen de madera es exactamente lo que
antes se destinaba a Cantera/Corral/Minas (necesarias para el gate de nivel 2, `≥3 de 6` edificios de
extracción). El nuevo comportamiento del NPC, sin querer, hace que el asentamiento invierta su excedente en
comercio en vez de en extracción — un trade-off real entre "sobrevivir mejor" y "progresar de nivel", no
anticipado al diseñar el trueque de supervivencia.

## Hallazgo #6 — Menos XP de combate: el gate de reclutamiento reduce la caza de bandidos

| Métrica (tick 3000) | Batch anterior | Este batch |
|---|---|---|
| Reclutamientos acumulados | 2442 | 545 |
| Campamentos destruidos acumulados | 302 | 123 |
| Nivel de Facción máximo | 10 | 8 |
| XP de Facción máxima | 7549 | 989 |

Coherente con el propio propósito del gate (menos reclutamiento imprudente = menos escuadrones disponibles
para cazar bandidos = menos XP de combate) — no es una sorpresa, pero vale la pena dejarlo cuantificado: la
prudencia que salva madera en la ventana crítica temprana tiene un costo real en progreso militar/XP de
Facción a lo largo del batch completo.

## Hallazgo #7 — Motor robusto pese a la carga extra

0 excepciones en 3000 ticks, con Mercado/caravana/trueque de supervivencia evaluándose para hasta 100
asentamientos cada tick además de todo lo que ya hacía el NPC (reclutamiento, bandidos, expansión). El batch
tardó 71s (vs ~35s del batch de referencia) — el costo computacional extra es real pero manejable.

## Nota metodológica

- Motor sin tocar — confirmado por `git status` antes y después de correr el batch: solo
  `simulaciones-batch/npcGobernanza.ts` y su `README.md` tienen cambios de esta sesión.
- No se instrumentó por separado cuánto del colapso posterior a tick 100 se debe a `factorPoblacion` de
  Mantenimiento creciendo vs. otras causas — queda como pregunta abierta (Hallazgo #2).
- No se verificó si los acuerdos "acumulados en exceso" (Hallazgo #4) llegan a causar sobregiro real de
  recursos en el lado ofertante cuando varias caravanas intentan entregar contra promesas que sumadas superan
  lo que ese asentamiento en verdad tiene de sobra — con solo 137 cumplidos de 7723 propuestos, el riesgo
  práctico parece bajo en este batch, pero no se confirmó explícitamente.

## Sugerencia de siguiente paso

Ninguna de estas correcciones se implementó en este batch — quedan para que el usuario decida:

1. **Cerrar el gap de acuerdos duplicados** (Hallazgo #4): antes de proponer un nuevo trueque de
   supervivencia, comprobar el volumen YA pactado (pendiente de entrega) para ese recurso, no solo si existe
   un acuerdo con ese socio específico — evitaría la explosión de 7723 propuestas para una necesidad real
   mucho más pequeña.
2. **Revisar el orden de prioridad de la infraestructura comercial** (Hallazgo #5): quizás Mercado/caravana
   debería esperar a un margen mayor (o directamente evitarse mientras el asentamiento aún no cumple el gate
   de extracción de nivel 2), para no competir con la extracción por el mismo excedente de madera.
3. Un batch de control con SOLO el gate de reclutamiento (sin el trueque de supervivencia ni la
   infraestructura comercial) aislaría cuánto de la mejora del Hallazgo #1 viene de cada pieza por separado.
