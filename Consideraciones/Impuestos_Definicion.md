# Impuestos — generación de oro por población: diseño

> **DISEÑO EN CURSO (2026-09-08).** Ronda 1 de decisiones cerrada con el usuario (4 decisiones). El plan
> técnico paso a paso queda para la siguiente ronda, a petición del usuario ("full diseño antes del plan de
> desarrollo"). Ficha de origen: `Docs/Mecanicas a desarrollar.md` §1.

## 0. El punto de partida

El oro entra hoy por **dos** vías y sale por otras dos:

| Entra | Detalle |
|---|---|
| `mina` | `produccionBaseOro: 4`/min. Pero el oro es un mineral **raro** (`RECURSO_RAREZA.raro`, como el cobre/estaño) y pocas posiciones de fundación tienen un nodo alcanzable en su zona — como referencia, la nota de "Arma de Madera" (Doc 4.2.1) mide ≈6% para cobre/livestock. Para la mayoría de asentamientos esta vía **no existe**. |
| Comisiones de comercio | Trueque: `agregarRecurso(almacen,'oro',comision)` al entregar (`engine/trade.ts`). Mostrador (Doc 3.3): se la queda la plaza donde ocurre la transacción (`engine/market.ts`). Exige comercio activo. |

| Sale | Detalle |
|---|---|
| Mantenimiento | `MANTENIMIENTO.oroBase: 2`/min × `factorPoblacion` × `factorDistancia`. **Solo a partir de nivel 3** (`nivelParaOro`). |
| Compras | Mercado (mostrador), animales de caravana (caballo 60, camello 40 — el buey se paga en madera), Maravilla (500 oro). Todo iniciado por un jugador o el heurístico NPC. |

Doc 3.1 lista como usos *previstos* del oro la tecnología/Aedas, los mercenarios y los sueldos — **ninguno
existe en Fase 0**. Doc 3.5 marca como PENDIENTE explícito "en qué se usa la riqueza acumulada".

**Consecuencia:** un asentamiento sin mina de oro alcanzable y sin comercio activo no ve una sola pepita, pase
lo que pase con su población. Y como el techo de nivel real de la Fase 0 para el NPC es 2 (ver §1 empírico),
el único sink de oro que no depende de un jugador —el mantenimiento de nivel 3— **nunca se dispara** en un
mundo NPC.

## 1. Lo que dice el batch (medición 2026-09-08, 40 facciones)

`BATCH_FACCIONES=40`, seeds por defecto, dos corridas (1200 y 1500 ticks):

| Señal | Valor | Lectura |
|---|---|---|
| `oroMedio` por asentamiento | 158 → 165 → 190 → 216 → 228 → 232 → 236 (ticks 250→1500) | Crece **desacelerando** hacia una meseta en los ~230-240. Fuentes ≈ sinks hoy: el oro se acumula despacio y se está frenando solo. |
| `artesanosTotal` (mundo entero) | **0** | Ningún NPC construye un edificio de transformación en toda la corrida. |
| `palaciosActivos` | **0** | Ningún NPC construye Palacio → **cero Nobleza en todo el mundo**. |
| `nivelesAsentamiento` | `{1: 10, 2: 39, 3: 0}` | Sin Artesanos no hay gate a nivel 3. El NPC vive entero en nivel 1-2. |
| `pesantsMedia` / `pesantsMaximo` | 181 / 225 | Asentamientos NPC pequeños, **solo Pesants**. |
| `conMercado` | 46/49 | El comercio sí está activo casi en todas partes — de ahí las comisiones que sostienen los ~236. |
| `nutricionMedia` | 100 | Bien alimentados; el freno no es la comida. |

**Lo que esto implica para el diseño:**

1. **La diferenciación por clase es, hoy, una palanca de jugador.** En el mundo NPC no hay Artesanos ni
   Nobleza, así que `recaudacionOro` NPC = `pesants × tasaPesants` y nada más. El batch **solo puede calibrar
   la tasa de Pesants**; las tasas de Artesanos y Nobleza se calibran por intención de diseño y por
   razonamiento sobre asentamientos de jugador (que sí construyen Palacio y transformación), no por simulación.
2. **No hay sink de oro que valga para el NPC.** El único que existe (mantenimiento nivel 3) no se alcanza. El
   mercado recicla algo de oro (compras), y eso es lo que explica la meseta de `oroMedio`.
3. **Añadir recaudación pasiva rompe esa meseta.** Con ~181 Pesants de media y sink casi nulo, hasta una tasa
   "modesta" de 0.01/hab son +1.8 oro/min/asentamiento sin nada que lo drene: ~+2.200 oro por asentamiento en
   1.200 ticks, es decir `oroMedio` pasaría de ~236 a ~2.400. Un factor 10. Sobre un servidor de 12 meses,
   ilimitado.

**El impuesto y el hueco de sinks son dos mitades del mismo problema.** No se puede diseñar el primero
ignorando el segundo (§6).

## 2. Decisiones cerradas — Ronda 1 (2026-09-08)

1. **Activación: pasiva y automática, espejo del consumo de comida, MÁS política de "Presión Fiscal" desde el
   primer pase.** Sin edificio nuevo. La recaudación base corre cada tick para todo asentamiento; la política
   es el knob que la sube a cambio de un coste.
2. **Orden de clases: Nobleza > Artesanos > Pesants** (rinde más per cápita la Nobleza). Base imponible por
   riqueza; crecer Nobleza = asentamiento rico; refuerza el valor del Palacio.
3. **Magnitud: intermedia.** Excedente moderado perceptible, ni colchón mínimo ni motor de oro desbocado. El
   sink queda como **bloqueante conocido** de la calibración final, no resuelto aquí (§6).
4. **Control del jugador en el pase 1: panel + política de Presión Fiscal.** El panel muestra "recaudación:
   +X oro/min" desglosada por clase. La política la fija un cargo (ver §3, punto abierto: Tesorero vs
   Gobernador).

## 3. El modelo

### 3.1 Recaudación base — `recaudacionOro(asentamiento)`

Espejo exacto de `consumoComidaPoblacion` (`engine/population.ts`), signo opuesto:

```
recaudacionOro(a) =
    ( a.poblacion.pesants   × IMPUESTOS.tasaPesants
    + a.poblacion.artesanos × IMPUESTOS.tasaArtesanos
    + a.poblacion.nobleza   × IMPUESTOS.tasaNobleza )
  × factorPresionFiscal(a)          // política, §3.3 — 1 si no hay ninguna activa
```

- **Se suma a `almacen.oro`** cada tick, con `agregarRecurso` (respeta capacidad de almacén, igual que la
  producción de mina — el oro que no cabe se pierde, ver `agregarRecursoConSobrante`).
- **Punto en el tick:** dentro de la pasada por asentamiento de `avanzarSimulacion`, junto a población /
  mantenimiento. Orden exacto pendiente del plan técnico, pero **después** de `crecerPoblacion` (recauda sobre
  la población de este tick) y **antes** de `avanzarMantenimiento` (que el oro recién recaudado pueda pagar el
  mantenimiento del mismo tick — si no, el asentamiento entra en déficit un tick de más por un problema de
  orden, no de economía).
- **NO escala por distancia a la capital.** El `factorDistancia` del mantenimiento ya es el "impuesto de
  cohesión" del lado del COSTE (Fase_0_5 §5.1). Meterlo también en el ingreso sería doble contabilidad. Dejar
  el ingreso plano hace que el impuesto **premie crecer alto y cohesionado** — alineado con la intención de
  la ficha ("sentido económico a crecer un asentamiento más allá de su producción de materiales").
- **NO escala por `nivelActual` degradado.** Misma lógica que el mantenimiento, que tampoco baja al degradarse
  ("la misma gente sigue gastando lo mismo", Doc 4.5 §6.2): la misma gente sigue tributando lo mismo. Anotado
  como acople futuro posible (§7) — una administración en ruinas recaudando menos crearía riesgo de espiral,
  interesante pero no para el primer pase.
- **`estabilidad` / `felicidad` como placeholder = 1**, igual que ya hacen las 3 fórmulas de `crecerPoblacion`.
  Cuando exista un medidor de bienestar (Sprint 4+, Sacerdote), entra aquí como multiplicador por el mismo
  sitio, sin rediseñar nada.
- **Bruto, no neto.** El oro que se recauda y el trigo que se consume son recursos distintos; no hay un cálculo
  "neto" — se añade el ingreso y ya.

### 3.2 Tasas por clase — `IMPUESTOS` en `constants.ts`, PLACEHOLDER

Punto de partida propuesto, oro/min per cápita (pendiente de calibración por batch para Pesants; por
razonamiento de diseño para Artesanos/Nobleza):

```ts
export const IMPUESTOS = {
  tasaPesants:   0.008,   // subsistencia — apenas tributan
  tasaArtesanos: 0.03,    // ~4× Pesants: tienen excedente, comercian
  tasaNobleza:   0.12,    // ~15× Pesants: base imponible por riqueza (decisión 2)
} as const;
```

Anclaje del arranque (a discutir en Ronda 2):

- Asentamiento NPC típico (181 Pesants, 0 Art, 0 Nob): **+1.4 oro/min** — un trickle, ~⅓ de una mina. Sobre la
  meseta actual de `oroMedio` ~236 sigue siendo inflación lenta sin sink, pero contenida (§6 propone que el
  trickle base sea deliberadamente bajo y que el knob real sea la política).
- Asentamiento de jugador nivel 2 orientado a producción (800 Pesants + 200 Artesanos): 6.4 + 6 = **+12.4
  oro/min** — comparable a 3 minas, sin nodo de oro. Empieza a "valer la pena crecer".
- Ciudad de jugador nivel 3 (4.000 Pesants + 400 Art + 60 Nob): 32 + 12 + 7.2 = **+51 oro/min**, contra un
  mantenimiento de oro de ~26/min a esa población. Excedente neto grande — **este es el caso que el sink tiene
  que poder drenar** (§6).

### 3.3 Política "Presión Fiscal" — cargo, efecto

Encaja sin fricción en el catálogo existente (`POLITICA_CATALOGO`, mismo molde que "Edicto de Cosecha" /
"Racionamiento"): entradas nuevas con campos `factor*`, un accessor en `engine/politicas.ts`, y el llamador
aplica el factor.

```ts
{ id: 'presion_fiscal',  cargo: '???', nombre: 'Presión Fiscal',
  factorRecaudacion: 1.6, factorCrecimientoPoblacion: 0.8 },
{ id: 'alivio_fiscal',   cargo: '???', nombre: 'Alivio Fiscal',
  factorRecaudacion: 0.5, factorCrecimientoPoblacion: 1.15 },
```

- **Sin política activa = baseline** (`factorRecaudacion` = 1). Las dos son toggles excluyentes por vivir en
  el mismo pool/slot, igual que las ordenanzas de trazado.
- **El coste de "Presión Fiscal" es un golpe directo al crecimiento de población** — `factorCrecimientoPoblacion`
  multiplica el `comidaFactor × ... ` común a las 3 clases en `crecerPoblacion`. No hace falta un sistema de
  felicidad nuevo para tener un downside real: más oro ahora ↔ menos gente mañana. (Cuando exista felicidad,
  se puede migrar el downside ahí.)
- Números todos PLACEHOLDER.

**PUNTO ABIERTO — ¿qué cargo?** El usuario dijo "Gobernador". Pero existe el cargo **Tesorero**, cuyo pool es
justo la economía de la plaza: `comercio_abierto` / `aranceles` (comisiones), y toda la economía de flota de
caravanas. La presión fiscal es temáticamente de Tesorero, no de Gobernador. Como el Gobernador tiene **pool
completa** (puede activar cualquier política, `activarPolitica`), ponerla en el pool de Tesorero no se la
quita al Gobernador — solo decide de qué slot sale y quién más puede usarla. **Recomendación: Tesorero.**
Confirmar en Ronda 2.

## 4. Lo que ya existe y no hay que inventar

- **`consumoComidaPoblacion`** es el molde exacto de `recaudacionOro`: `poblacionTotal × tasa × factorPolítica`.
  Copiar la forma, invertir el signo, desglosar por clase.
- **`agregarRecurso` / `agregarRecursoConSobrante`** (`engine/almacen.ts`) ya gestionan sumar un recurso al
  almacén respetando capacidad — la producción de mina ya pasa por ahí.
- **El catálogo de políticas y `productoFactor`** (`engine/politicas.ts`) ya resuelven "una política aporta un
  `factor*` que el motor multiplica". "Presión Fiscal" es una entrada más, como "Edicto de Cosecha".
- **`factorConsumoComida`** ya demuestra el patrón de un factor de política que modula un cálculo per cápita de
  población — `factorRecaudacion` es su gemelo.
- **El panel de Mantenimiento** (`gameStore.mantenimientoInfo`) ya reúne y muestra costes/consumos por tick —
  la línea de "recaudación: +X oro/min" desglosada por clase se añade ahí, no en un panel nuevo.
- **`avanzarTributos`** (`engine/diplomacia.ts`) ya mueve oro entre facciones por vasallaje — no es este
  sistema, pero confirma que "oro que se transfiere por tick automáticamente" ya tiene precedente en el motor.

## 5. Representación en el motor (esbozo — se cierra con el plan técnico)

- **`constants.ts`:** `IMPUESTOS` (§3.2) + 2 entradas en `POLITICA_CATALOGO` (§3.3).
- **`engine/politicas.ts`:** accessors `factorRecaudacion(a)` y `factorCrecimientoPoblacion(a)` vía
  `productoFactor`; extender `CampoFactor`.
- **`engine/population.ts`:** función `recaudacionOro(asentamiento)` nueva; `crecerPoblacion` multiplica su
  factor común por `factorCrecimientoPoblacion(a)`.
- **`engine/simulation.ts`:** una llamada en la pasada por asentamiento, entre `crecerPoblacion` y
  `avanzarMantenimiento` (§3.1).
- **Cliente:** una línea en el panel de Mantenimiento/economía.
- **Sin cambios de tipo en `Asentamiento`** — la recaudación es derivada, no estado. La política ya cabe en
  `politicasActivas`.
- **Sin migración de snapshot.**
- **Tests:** un `test_*` de motor sobre `recaudacionOro` (desglose por clase, factor de política, tope de
  almacén) y el efecto de `factorCrecimientoPoblacion` en `crecerPoblacion`.

## 6. La tensión de verdad: el sink de oro

**Estado: bloqueante conocido, NO resuelto en esta ficha (decisión 3 del usuario).**

El batch (§1) es inequívoco: hoy no hay sink de oro que funcione para un asentamiento que no comercie a lo
grande ni llegue a nivel 3. Añadir ingreso pasivo sin sink = `oroMedio` crece sin techo. Para que el impuesto
cumpla su intención declarada —que el oro sea algo que **quieras** y que **no te sobre**— tiene que haber algo
caro y recurrente en lo que gastarlo.

**Mitigación para poder shippear el pase 1 sin resolver esto del todo:**

- El **trickle base** (tasa de Pesants) se deja deliberadamente **bajo** (0.008 propuesto) — el NPC, que no
  activa políticas para esto todavía, solo ve ese trickle: ~+1.4 oro/min, inflación lenta y acotada, no
  explosiva.
- El **knob real es la política** de Presión Fiscal, que solo activa un jugador que *sabe para qué quiere el
  oro*. Un jugador que sube impuestos y no tiene en qué gastar el oro está pagando el coste de crecimiento a
  cambio de nada — se corrige solo.
- La **calibración final de las tasas espera al sink.**

**Candidatos a sink (para rondas siguientes, no se decide aquí):**

| Candidato | Nota |
|---|---|
| Adelantar el mantenimiento de oro a **nivel 2** (`nivelParaOro: 2`) | El más barato: una constante. Hace que ~39/49 asentamientos NPC tengan un coste de oro real, y el batch podría por fin calibrar contra un sink de verdad. |
| **Taberna: intel y mapas como asset** (ya anotado al final de `Docs/Mecanicas a desarrollar.md`) | Sink recurrente y voluntario, y complementa niebla de guerra. Candidato natural a diseñarse en tándem con esto. |
| **Sueldos de tropa en oro** | Doc 3.1 ya lo lista como uso previsto. Espejo de la ración de trigo (Doc 5.4). |
| **Mercenarios / tecnología (Aedas)** | Doc 3.1 los lista; ambos sistemas están diferidos enteros (Doc 6) — sink potente pero lejano. |
| Recargo de oro en mejoras de edificio de nivel alto | Da un destino al oro sin sistema nuevo. |

## 7. Puntos abiertos

- **Ronda 2 — cargo de la política:** Tesorero (recomendado) vs Gobernador (dicho por el usuario). §3.3.
- **Ronda 2 — números de arranque:** confirmar el trío `IMPUESTOS` y los factores de la política, o ajustarlos.
- **Ronda 2 — dirección del sink:** ¿se aborda un sink en este mismo bloque de trabajo (p. ej. `nivelParaOro: 2`
  + Taberna) o se shippea el impuesto con trickle bajo y el sink va aparte? §6.
- **Diferido — NPC y la política:** `npcGobernanza` no activa Presión Fiscal. Mientras no lo haga, el NPC solo
  ve el trickle base. Darle criterio (¿sube impuestos si el oro está bajo y la población alta?) es una pasada
  posterior.
- **Diferido — acople con `nivelActual`:** recaudación reducida en un asentamiento degradado (riesgo de
  espiral, temático). Fuera del primer pase. §3.1.
- **Diferido — `estabilidad`/`felicidad`:** cuando exista el medidor de bienestar, migrar el downside de
  Presión Fiscal ahí y añadir el multiplicador de bienestar a `recaudacionOro`.

## 8. Invariantes

- `recaudacionOro` es **derivada**, nunca estado guardado — se recalcula cada tick desde población + políticas.
- El oro recaudado **respeta la capacidad de almacén** (se pierde el sobrante), igual que la producción de mina.
- Sin política de Presión Fiscal activa, `factorRecaudacion` = 1 y `factorCrecimientoPoblacion` = 1 — el
  comportamiento por defecto no cambia respecto a "tasas base planas".
- La recaudación **no escala por distancia a la capital ni por `nivelActual`** (§3.1) — el eje de distancia es
  del lado del coste (mantenimiento), no del ingreso.
- El batch NPC **cambia con esta mecánica** — es un sistema nuevo, no un refactor. `oroMedio` subirá. Se
  documenta el antes/después en la corrida de verificación.
