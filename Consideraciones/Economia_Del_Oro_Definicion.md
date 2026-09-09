# La economía del oro — el oro como presupuesto: diseño

> **DISEÑO CERRADO (2026-09-08). IMPLEMENTACIÓN: Pasos 1-5 hechos, falta el Paso 6 (campaña de calibración).**
> Tres rondas de decisiones con el usuario. **Cierra la entrada que fue §1 (Impuestos) de
> `Docs/Mecanicas a desarrollar.md`** y cubre el lado de la DEMANDA que la hace tener sentido (reclutamiento en
> oro, animales de caravana en oro, intel de taberna). El sink recurrente de intel/taberna sigue siendo su
> propia mecánica pendiente (`Docs/Mecanicas a desarrollar.md` §18). Plan técnico y estado por paso en §10. Canon escrito: Doc 3 §3.1/§3.13.2, Doc 4 §4.1/§4.4/§4.5,
> Doc 5 §5.8. Tests: `recaudacionOro` · `presionFiscal` · `reclutamientoOro` (+ ajustes en `caravanas.test.ts`
> por el buey en oro, y `snapshot_baseline` re-generado por la recaudación).
>
> **Calibración (Paso 6) — iteración 1 (2026-09-08):** medición de Pasos 1-5 → `oroMedio` ×4 y expansión
> desbocada. Ajustes: `IMPUESTOS` a la mitad (0.004/0.015/0.06), y **la caravana #0 gratis se quita** (era
> también un amplificador de la fuente: más comercio → más comisiones). El buey sigue en oro — la fundación ya
> trae 100 oro para la primera caravana.
>
> Este bloque **revisa dos decisiones cerradas del revamp de caravanas** (`Revamp_Caravanas_Definicion.md`
> §17 buey en madera, §18 bootstrap descartado) — ver §7.

## 0. El trabajo del oro en el canon

Todo recurso del juego es de **un solo propósito**: madera construye, trigo alimenta, cobre/estaño → bronce
arma. El oro es el único que **convierte entre ambiciones** — traduce "quiero ejército" a "quiero flota" a
"quiero saber qué hace el vecino". La escasez de oro es *el* mecanismo que obliga a elegir entre expansión
militar, imperio comercial y juego informado. **"Que el oro me limite ir a por todo a la vez, que gastarlo sea
una decisión consciente"** (usuario).

Hoy el oro no hace casi nada de eso:

| Entra | Detalle |
|---|---|
| `mina` | `produccionBaseOro: 4`/min. Oro = mineral **raro** (`RECURSO_RAREZA.raro`); pocas fundaciones tienen nodo alcanzable — referencia: la nota "Arma de Madera" (Doc 4.2.1) mide ≈6% para cobre/livestock. Para la mayoría, esta vía **no existe**. |
| Comisiones de comercio | Trueque: `agregarRecurso(almacen,'oro',comision)` al entregar. Mostrador (Doc 3.3): se la queda la plaza donde ocurre la transacción. Exige comercio activo. |

| Sale | Detalle | Peso real |
|---|---|---|
| Mercado (mostrador) | Comprar género paga oro del carro. | El **único** sink real hoy — y es opcional. |
| Mantenimiento | `oroBase: 2`/min × escala, **solo desde nivel 3**. | El NPC nunca llega a nivel 3 (§1) → **nunca se dispara** en mundo NPC. |
| Animales de caravana | caballo 60, camello 40. **Buey = 30 madera**, no oro. | Marginal: casi todo el mundo usa buey. |
| Maravilla | 500 oro | Trofeo de fin de partida, uno por Facción. |
| Reclutamiento | **0 oro** — todo `costoEquipo` (cadenas de Armería). | Cero. |
| Intel / tabernas | No existe. | — |

Doc 3.1 lista como usos *previstos* la tecnología/Aedas, los mercenarios y los sueldos — **ninguno existe en
Fase 0**. Doc 3.5 marca PENDIENTE explícito "en qué se usa la riqueza acumulada".

**Consecuencia medida:** nadie está limitado por oro en nada que importe.

## 1. Lo que dice el batch (medición 2026-09-08, POST-fix del arnés)

El diario `Diarios_Simulaciones_Batch/Diario_Simulaciones_Batch_IdsNpc_Deadlock_Transformacion.md` (2026-09-08)
encontró y arregló un **bug del arnés de batch** (`scripts/run-batch-sim.ts` no hilaba el contador de ids del
NPC entre ticks → ids colisionados → deadlock de la cola de obra → 0 edificios de transformación → 0
Artesanos). Fix de 3 líneas, no toca el motor. Toda medición de oro/artesanos anterior a ese commit está
contaminada.

**Corrida post-fix** (`BATCH_FACCIONES=40`, seeds por defecto, 1500 ticks):

| Señal | tick 500 → 1500 | Lectura |
|---|---|---|
| `oroMedio` por asentamiento | 89 → 166 → **244** | Sube **casi lineal, sin mesetear** (a diferencia del pre-fix, que aplanaba en ~236). Más Artesanos → más producción → más comercio → más comisiones entrando, con el mismo sink casi nulo. |
| `artesanosTotal` (mundo) | 4291 → **4630** | ~98 Artesanos por asentamiento vivo. **La diferenciación por clase del impuesto ahora SÍ es calibrable en batch** para Pesants y Artesanos. |
| `palaciosActivos` | **0** | El NPC no construye Palacio (adición manual, `npcGobernanza` no lo pone) → **cero Nobleza**. La tasa de Nobleza sigue siendo palanca solo de jugador. |
| `nivelesAsentamiento` | `{1:25, 2:24, 3:0}` | Aún **0 nivel 3** (su gate exige un recinto/muralla completo, que es adición manual). → el sink de mantenimiento-oro nivel 3 **sigue sin dispararse** para el NPC. |
| `pesantsMedia` | 332 → **386** | Rompe el techo de 225 del pre-fix (los Artesanos desbloquean el crecimiento). |
| `colapsados` acumulados | 56 → **120** | Fuerte estrés económico — la carga de mantenimiento de los edificios de transformación (issue de balance preexistente, no de este bloque). |
| `reclutamientosAcumulados` / `tropasVivas` | 5597 / 3866 | Churn militar alto. 188 conquistas, 147 campamentos de bandidos destruidos, 1017 campañas en la corrida. |

**Implicaciones para el diseño:**

1. **`oroMedio` no mesetea post-fix.** Añadir recaudación pasiva sin un sink de verdad = inflación clara y
   sostenida.
2. **Pesants + Artesanos calibrables en batch; Nobleza no** (0 Palacios NPC). La tasa de Nobleza se fija por
   diseño + razonamiento sobre asentamientos de jugador.
3. **El NPC recluta fuerte y ahora puede armar bronce** (tiene Armería). Gatear el reclutamiento con oro
   **muerde de verdad** el militar NPC — 188 conquistas y 147 campamentos dependen de esa capacidad. Es el
   riesgo #1 a medir.
4. El sink de mantenimiento-oro nivel 3 es letra muerta para el NPC — si se quiere que el mantenimiento sea un
   drenaje involuntario real, hay que adelantarlo (§4.4).

## 2. Decisiones cerradas (3 rondas, 2026-09-08)

### Ronda 1

1. **Recaudación: pasiva y automática, espejo del consumo de comida, + política "Presión Fiscal" desde el
   pase 1.** Sin edificio nuevo.
2. **Orden de clases: Nobleza > Artesanos > Pesants** (rinde más per cápita la Nobleza).
3. **Magnitud: intermedia** — excedente moderado. → revisado en Ronda 2: el sink deja de ser "punto pendiente"
   y pasa a ser parte del mismo bloque (§3).
4. **Control en el pase 1: panel + política de Presión Fiscal.**

### Ronda 2 — el lado de la demanda

5. **El oro es un PRESUPUESTO, y las cuatro palancas se calibran juntas.** Ingreso (impuesto + comercio) debe
   cubrir *uno o dos* de {ejército, flota, intel}, no los tres. Ninguna palanca se calibra aislada → **un solo
   bloque económico, se shippea junto, se calibra en una campaña de batch** (§3). Esto invierte la decisión 3
   de la Ronda 1 ("sink como punto pendiente").
6. **Reclutamiento en oro, salvo la milicia inicial.** `milicia_lanceros` (Centro Urbano) sigue `{madera: 2}`.
   Todo lo demás suma una línea de oro, escalada por escalón (§4.2).
7. **Todos los animales de caravana en oro, buey incluido** (§4.3). Revisa `Revamp_Caravanas` §17.
8. ~~**La primera caravana es gratis: 1 carro + 1 buey al completar el Mercado**~~ — **QUITADA en calibración
   (Paso 6, iteración 1, 2026-09-08).** Se implementó y midió: además de on-ramp era un amplificador de la
   fuente (todo asentamiento comerciando desde el tick 1 → más comisiones → más oro), y con la fundación ya
   entregando 100 oro, el on-ramp no hacía falta. `Revamp_Caravanas` §18 se queda como estaba (bootstrap
   descartado). El punto 7 (buey en oro) sigue en pie.
9. **Intel de taberna = el sink recurrente clave, pero ficha aparte** (§4.5).

### Ronda 3 — cierre

10. **Cargo de "Presión Fiscal" / "Alivio Fiscal": Tesorero.** El Gobernador conserva pool completa y puede
    activarlas igual.
11. **El único reclutamiento sin oro es la milicia inicial** (`milicia_lanceros`, Centro Urbano). Todo lo del
    Barracón/Galería cuesta oro, **incluido el escalón 1** (lanceros de mimbre, honderos). Sin oro solo tienes
    la tropa más débil del roster.
12. **`MANTENIMIENTO.nivelParaOro: 3 → 2` entra en este bloque.**
13. **El oro de reclutamiento y de animales es regla de motor uniforme** — se cobra igual a NPC y jugador
    desde el pase 1. La campaña de batch mide el efecto real, incluido que el militar NPC pueda encogerse.
14. **`leva_forzosa` (×0.7) toca solo el equipo, no el oro** — no se conscribe moneda. (Default, sin objeción.)

## 3. El principio de presupuesto

"Que el oro me limite ir a por todo a la vez" es una afirmación cuantitativa. Solo funciona si:

```
ingreso_oro(asentamiento)  ≈  coste_de_UNA_ambición  +  algo de margen
                           <  coste_de_DOS_ambiciones simultáneas a fondo
```

donde las ambiciones son: **ejército** (reclutar + reponer bajas), **flota** (animales para escalar caravanas
más allá de la #0 gratis), **intel** (suscripción de taberna), y el drenaje involuntario del
**mantenimiento-oro**.

Esto obliga a calibrar a la vez: tasas de impuesto · oro de reclutamiento por escalón · oro por animal ·
precios de intel · `nivelParaOro`. **No hay orden de implementación en el que uno se pueda cerrar antes que
los otros** — de ahí la decisión 5.

## 4. El modelo, palanca por palanca

### 4.1 Recaudación base — `recaudacionOro(asentamiento)`

Espejo exacto de `consumoComidaPoblacion` (`engine/population.ts`), signo opuesto:

```
recaudacionOro(a) =
    ( a.poblacion.pesants   × IMPUESTOS.tasaPesants
    + a.poblacion.artesanos × IMPUESTOS.tasaArtesanos
    + a.poblacion.nobleza   × IMPUESTOS.tasaNobleza )
  × factorPresionFiscal(a)          // política — 1 si ninguna activa
```

- **Se suma a `almacen.oro`** cada tick con `agregarRecurso` (respeta capacidad; el sobrante se pierde, igual
  que la producción de mina).
- **Punto en el tick:** en la pasada por asentamiento de `avanzarSimulacion`, **después** de `crecerPoblacion`
  y **antes** de `avanzarMantenimiento` (que el oro recién recaudado pueda pagar el mantenimiento del mismo
  tick).
- **NO escala por distancia a la capital** — ese eje ya es el "impuesto de cohesión" del lado del COSTE
  (Fase_0_5 §5.1). Dejar el ingreso plano hace que el impuesto **premie crecer alto y cohesionado**, alineado
  con la ficha.
- **NO escala por `nivelActual` degradado** — misma lógica que el mantenimiento (Doc 4.5 §6.2). Acople futuro
  posible (§8).
- **`estabilidad`/`felicidad` = 1** placeholder, igual que ya hacen las 3 fórmulas de `crecerPoblacion`.

### 4.2 Reclutamiento en oro

Hoy `reclutarTropa` (`engine/tropas.ts`) descuenta `costoEquipo × cantidad × factorCostoReclutamiento`. Se
añade una **componente de oro por soldado**, derivada del escalón (igual criterio que `LIDERAZGO.costePorEscalon`
— la élite es deliberadamente cara):

```ts
export const RECLUTAMIENTO_ORO_POR_ESCALON: Record<number, number> = {
  1: 1,    // leva de Barracón/Galería (lanceros de mimbre, honderos)
  2: 2,    // tropa de línea
  3: 4,    // veterana
  4: 7,    // pesada
  5: 11,   // élite
} as const;   // PLACEHOLDER — por soldado; coste total = valor × unidadesPorEscalón × factorCostoReclutamiento(equipo)
```

- **`milicia_lanceros` (Centro Urbano) es la ÚNICA tropa sin oro** — `costoEquipo` sigue `{madera: 2}` y nada
  más. La defensa mínima no depende del tesoro (invariante ya existente: se sacó del Barracón por esto).
  Regla de implementación: exención por `tropa.edificio === 'centroUrbano'`, no por escalón.
- **Todo lo demás cuesta oro**, escalón 1 incluido — sin oro solo tienes la milicia.
- **Reponer bajas de un escuadrón vuelve a pagar oro** por los soldados repuestos (ya paga equipo hoy).
- **`leva_forzosa`** (`factorCostoReclutamiento` ×0.7): toca **solo el equipo, no el oro** (Ronda 3, decisión
  14).
- **Compatible con mercenarios diferidos** (Doc 3.1): "tropa por oro puro, sin cadena de equipo" queda como
  extensión limpia — cambiar tesoro por profundidad industrial.

### 4.3 Caravanas: animales en oro (buey incluido)

| Pieza | Coste | Cambio |
|---|---|---|
| Carro básico (Mercado) / reforzado (Carpintería) | madera (20 / 40) | Sin cambios. |
| Buey | **~12 oro** (PLACEHOLDER; era 30 madera) | Revisa `Revamp_Caravanas` §17. |
| Caballo / Camello | 60 / 40 oro | Sin cambios. |
| ~~Caravana #0 gratis con el Mercado~~ | — | **Probada y quitada** en calibración (§10 Paso 6, iter 1). |

- **El deadlock que puso el buey en madera** ("sin caravana no hay comercio, sin comercio no hay oro, sin oro
  no hay caravana") se corta porque **la fundación entrega 100 oro** (`FUNDACION.materialesIniciales`) y la
  recaudación de oro por población lo repone aunque no haya mina — un asentamiento nuevo se paga su primera
  caravana (20 madera + 12 oro) con lo de fundar.
- **Escalar la flota es el coste de oro:** cada carro extra necesita su animal, y cada animal es tesoro. La
  flota (`cupo` 2/4/6 por nivel de Mercado) compite por el mismo oro que el ejército y la intel. Buey barato
  (~12) para que sea una decisión de *cuántas* caravanas, no un muro.
- **La caravana #0 gratis se implementó y se midió** (Paso 5): además de on-ramp era un amplificador de la
  fuente — todo asentamiento comerciando desde el tick 1 → más comisiones → más oro. Con los 100 oro de
  fundación cubriendo el arranque, no aportaba lo suficiente para justificar el efecto. `Revamp_Caravanas`
  §18 se queda como estaba.

### 4.4 Mantenimiento-oro a nivel 2

`MANTENIMIENTO.nivelParaOro: 3 → 2` (Ronda 3, decisión 12). Hace que ~24/49 asentamientos NPC tengan un coste
de oro **involuntario** real, y da al batch un sink contra el que calibrar de verdad. Una constante. Riesgo:
sube la ya alta tasa de colapso (§1) — se mide en la campaña, y si es inasumible se recalibra `oroBase` a la
baja para el nuevo escalón.

### 4.5 Intel de taberna — ficha aparte, pero es la pieza clave

Reclutamiento y animales son **compras de capital de una vez**: montas el ejército una vez y lo alimentas en
trigo; compras el animal una vez. **La intel es una suscripción** — pagas "saber qué hace la Facción X durante
N ticks", expira, vuelves a pagar si aún importa. Es el **único sink que drena de verdad un tesoro que se
acumula** — sin él, `oroMedio` sube sin techo pase lo que pase con las compras de una vez.

- **Forma** (del apunte al final de `Docs/Mecanicas a desarrollar.md`): edificio Taberna; intel/mapas como
  *asset* — revelado temporal de visibilidad de una zona lejana, o info "interna" (estados de asentamientos,
  tamaños de ejército, almacenes ajenos) que la niebla de guerra no da ni a distancia de contacto.
- **Precio escala con el valor**: intel de una Facción lejana y poderosa cuesta más que la del vecino que ya
  ves. Recurrente = el peaje de jugar informado.
- **Se diseña en su propia ficha** (`Taberna_Intel_Definicion.md`, por crear). Aquí solo se reserva el hueco y
  se fija que es el sink recurrente del presupuesto.

## 5. Política "Presión Fiscal"

**Una sola entrada** en `POLITICA_CATALOGO`, molde de "Edicto de Cosecha":

```ts
{ id: 'presion_fiscal', cargo: 'tesorero', nombre: 'Presión Fiscal',
  factorRecaudacion: 1.6, factorCrecimientoPoblacion: 0.8 },
```

- Sin política activa = baseline (factor 1). Presión Fiscal sube el oro y baja el crecimiento.
- **El coste es un golpe directo al crecimiento de población** — `factorCrecimientoPoblacion` multiplica el
  factor común de las 3 clases en `crecerPoblacion`. No hace falta un sistema de felicidad para tener un
  downside real. Cuando exista felicidad, se migra ahí.
- **Cargo = Tesorero** (Ronda 3, decisión 10). El Gobernador tiene pool completa y puede activarla igual.
- **"Alivio Fiscal" (bajar impuestos por debajo del baseline a cambio de más crecimiento) NO entra en el pase
  1** (revisión ponytail, §10): un jugador que quiere impuestos bajos simplemente no activa Presión Fiscal.
  Solo tiene sentido si el baseline mismo se siente como una carga y quieres ir por debajo — eso lo dice el
  playtest, y es una entrada de catálogo cuando lo diga.
- Números PLACEHOLDER.

## 6. Lo que ya existe y no hay que inventar

- **`consumoComidaPoblacion`** — molde exacto de `recaudacionOro`.
- **`agregarRecurso` / `agregarRecursoConSobrante`** (`engine/almacen.ts`) — sumar recurso respetando capacidad.
- **`POLITICA_CATALOGO` + `productoFactor`** (`engine/politicas.ts`) — "Presión Fiscal" es una entrada más.
- **`reclutarTropa`** (`engine/tropas.ts`) — ya calcula `costoTotal` por recurso y lo descuenta; añadir la
  línea de oro es una entrada más en ese objeto.
- **`ANIMAL_CATALOGO` / `CARRO_CATALOGO`** (`constants.ts`) — ya tienen `costo` por recurso; buey pasa de
  `{madera:30}` a `{oro:12}`.
- **`avanzarTributos`** (`engine/diplomacia.ts`) — precedente de "oro que se transfiere por tick".
- **El panel de Mantenimiento** (`gameStore.mantenimientoInfo`) — donde va la línea "recaudación: +X oro/min"
  desglosada por clase.

## 7. Decisiones que este bloque revisa (al cerrarse, actualizar)

- **`Revamp_Caravanas_Definicion.md` §17** (buey en madera) → **buey en oro (~12)**. Motivo: el impuesto + la
  caravana #0 gratis cortan las dos patas del deadlock que justificaba la madera.
- **`Revamp_Caravanas_Definicion.md` §18** (bootstrap descartado) → **bootstrap DE VUELTA** (1 carro + 1 buey
  gratis al completar el Mercado). Motivo: la medición que lo descartó usó el arnés con el bug de ids; se
  re-mide post-fix.
- **`Docs/Game/3` §3.13.2** (tabla de animales + "El Mercado no regala ninguna caravana") — al cerrar.
- **`Docs/Game/5` §recluta** y `Docs/Game/3` §3.5 — el oro entra como coste de reclutamiento; el oro tiene
  sinks reales.
- **`MANTENIMIENTO.nivelParaOro`** si se adopta §4.4.

## 8. Puntos abiertos

- **Números de arranque** (todo PLACEHOLDER, a calibrar en la campaña — Paso 6): `IMPUESTOS`
  (0.008 / 0.03 / 0.12), `RECLUTAMIENTO_ORO_POR_ESCALON` (1/2/4/7/11), buey (~12), factores de política
  (1.6 / 0.8), `oroBase` del mantenimiento tras bajar `nivelParaOro` a 2.
- **Riesgo #1 a medir:** efecto del oro de reclutamiento sobre el militar NPC (188 conquistas / 147
  campamentos de bandidos dependen de esa capacidad). Si lo estrangula demasiado: bajar
  `RECLUTAMIENTO_ORO_POR_ESCALON`, o enseñar a `npcGobernanza` a priorizar oro para reclutar (hoy no lo hace).
- **Diferido — NPC y Presión Fiscal:** `npcGobernanza` no la activa; el NPC solo ve el baseline. Darle criterio
  (¿sube impuestos si el oro está bajo y la población alta?) es una pasada posterior.
- **Diferido — acople con `nivelActual`:** recaudación reducida en un asentamiento degradado (riesgo de
  espiral, temático). Fuera del primer pase.
- **Diferido — mercenarios** (Doc 3.1), **sueldos de tropa en oro** (upkeep continuo — segundo drenaje militar,
  palanca de reserva si la fricción del reclutamiento no basta).
- **Ficha nueva:** `Taberna_Intel_Definicion.md` — el sink recurrente (§4.5), sin diseñar todavía.

## 9. Invariantes

- `recaudacionOro` es **derivada**, nunca estado guardado.
- El oro recaudado **respeta la capacidad de almacén** (se pierde el sobrante).
- **`milicia_lanceros` nunca cuesta oro.** La defensa mínima no depende del tesoro.
- **La fundación entrega oro suficiente para la primera caravana** (`FUNDACION.materialesIniciales.oro`) — la
  capacidad de comercio de arranque no depende de tener mina ni comercio previo.
- Sin política de Presión Fiscal activa, el comportamiento por defecto no cambia respecto a "tasas base planas".
- La recaudación **no escala por distancia a la capital ni por `nivelActual`**.
- El batch NPC **cambia con este bloque** — es economía nueva, no un refactor. `oroMedio` se moverá; se
  documenta el antes/después en la campaña de verificación.

## 10. Plan técnico

> **Revisado por ponytail.** Se recortó "Alivio Fiscal" (§5 — un jugador que quiere impuestos bajos no activa
> Presión Fiscal y ya; se añade si el playtest la pide). Todo lo demás se apoya en funciones y patrones que ya
> existen (rung 2-3 de la escalera): `recaudacionOro` es el espejo de `consumoComidaPoblacion`, la política es
> una entrada más de `POLITICA_CATALOGO` (cero cambios en `session`/`server` — el comando genérico
> `activarPolitica` la recoge sola), el oro de reclutamiento es una línea más en el `costoTotal` que
> `reclutarTropa` ya arma.
>
> **Correcciones descubiertas al implementar (2026-09-08):**
> - **La guarda de idempotencia de la caravana #0 SÍ va** (contra lo que decía la revisión inicial). Como el
>   id es determinista (`caravana-comercial-<asent>-bootstrap`), un Mercado reconstruido crearía un id
>   DUPLICADO — que es el bug de `Map` colisionado del arnés de batch (`Diario_..._Deadlock_Transformacion`),
>   no un duplicado cosmético. La guarda es un `.some()` de una línea: solo se regala si el asentamiento no
>   tiene ya una caravana comercial.
> - **`avanzarConstruccion` NO cambia de firma.** `simulation.ts` detecta el Mercado completado escaneando
>   `eventosConstruccion` por `codigo === 'construccion.edificio_completado'` con `payload.edificioTipo ===
>   'mercado'` — el evento ya lleva el tipo. Más lazy que el esbozo original.
> - **`crearCaravanaBootstrap` construye el objeto directo** (no reusa `crearCaravanaVacia`/`agregarCarro...`):
>   esas validan cupo/cooldown/recursos y aquí la caravana es un regalo. ~10 líneas en `engine/trade.ts`.

### Separación de capas — se respeta la que ya hay (`CAPAS_PERMITIDAS`, `src/__tests__/arquitectura.test.ts`)

Este bloque cae **entero en `engine` + `constants`**, con una única línea de presentación en el cliente admin.
**No toca `session` ni `server`.** Detalle:

| Pieza | Capa | Por qué ahí |
|---|---|---|
| `IMPUESTOS`, `RECLUTAMIENTO_ORO_POR_ESCALON`, `presion_fiscal` en `POLITICA_CATALOGO`, `buey.costo`, `nivelParaOro` | `constants` | Solo datos; `constants` solo importa `domain`. |
| `recaudacionOro`, su llamada en el tick, los accessors de política, la línea de oro en `reclutarTropa`, la caravana #0 al completar Mercado | `engine` | Funciones puras del motor sobre `Asentamiento`/`Caravana`. `engine` ya importa `domain`/`world`/`constants`. |
| Línea "recaudación: +X oro/min" en el panel | `cliente/` (frontend admin, fuera de las 3 capas del backend, ya destinado a salir del repo) | Se **calcula en el cliente** a partir de lo que la proyección ya manda (`poblacion` + `politicasActivas`) y del catálogo de `/balance`, igual que `mantenimientoInfo` ya calcula los costes de mantenimiento. Cero cambio de proyección. |

**Lo que confirma que `session`/`server` no se tocan:**

- La recaudación es **automática por tick** (motor), no un comando — no hay superficie de comando nueva.
- La política nueva viaja por el comando genérico `activarPolitica` (`session/comandos/cargos.ts`) **sin
  cambios**: `politicaId` es un `string` libre validado contra `POLITICA_CATALOGO` dentro del motor
  (`definicion()`), y el esquema (`esquemas.ts:95`) usa `IDENTIFICADOR` genérico, no un enum. `/balance`
  (`server/rutas/balance.ts`) ya sirve `POLITICA_CATALOGO` entero — la entrada nueva aparece sola.
- Reclutar y comprar animales usan los comandos existentes **sin cambios**: el motor que invocan cobra más, y
  ya.
- **Ninguna migración de snapshot en ningún paso** — todo es derivado, constante o catálogo; la política ya
  cabe en `politicasActivas`.

El guard `src/__tests__/autoridadTemporal.test.ts` tampoco se ve afectado: nada de esto lee reloj ni
aleatoriedad ambiente (la caravana #0 usa un id determinista `caravana-comercial-<asent>-bootstrap`).

**Verificado al implementar:** `npx tsc --noEmit` limpio, suite 1170/1170 (4 tests nuevos + `caravanas.test.ts`
ajustado por el buey en oro + `snapshot_baseline` re-generado por la recaudación — el diff del snapshot es
**solo la cantidad de `oro`**, nada más se movió, lo que confirma que el cableado del Paso 1 está aislado).
`cliente/` compila limpio (`npx tsc --noEmit`). *(Actualizado 2026-09-08: el error preexistente de
`gameStore.ts` — `posicion` en `ParamsFundarAsentamiento` — quedó corregido; el cliente ya no manda `posicion`,
el backend deriva la posición de la columna del fundador.)*

### Paso 1 — `recaudacionOro` + cableado — ✅ HECHO

- `IMPUESTOS` en `constants.ts` (§4.2 tasas).
- `recaudacionOro(asentamiento)` en `engine/population.ts` — espejo de `consumoComidaPoblacion`, desglose por
  clase.
- Llamada en `engine/simulation.ts`, en la pasada por asentamiento, **entre `crecerPoblacion` y
  `avanzarMantenimiento`**: `almacen = agregarRecurso(almacen, 'oro', recaudacionOro(conPoblacion))`.
- Cliente (`cliente/src/app/gameStore.ts`): línea "recaudación: +X oro/min (P/A/N)" junto a
  `mantenimientoInfo`, calculada client-side.
- Test motor: desglose por clase, tope de almacén, oro 0 con población 0.
- **Medido (Paso 1 EN AISLADO, 40 facciones, 1500 ticks, contra la línea base post-fix):** `oroMedio`
  244 → **1319** (×5.4), y con el dinero llega la cascada — `vivos` 47 → 103, `conquistasAcumuladas` 188 →
  547, `tropasVivas` 3866 → 7129, primeros asentamientos nivel 3-4 de la serie, `colapsados` 120 → 437. Es la
  prueba empírica de que **los sinks (Pasos 3/4/5) no son opcionales**: recaudación sin drenaje = inflación
  y todo el mundo comprando de todo a la vez. La medición del estado shippeado (Pasos 1-5 juntos) va en el
  Paso 6.

### Paso 2 — política "Presión Fiscal" — ✅ HECHO

- 1 entrada en `POLITICA_CATALOGO` (`cargo: 'tesorero'`, `factorRecaudacion`, `factorCrecimientoPoblacion`).
- `CampoFactor` += `factorRecaudacion` | `factorCrecimientoPoblacion`; accessors vía `productoFactor` en
  `engine/politicas.ts`.
- `recaudacionOro` multiplica por `factorRecaudacion(a)`; `crecerPoblacion` multiplica su factor común por
  `factorCrecimientoPoblacion(a)`.
- Test motor: con ninguna activa, resultado **bit-idéntico** al Paso 1 (invariante); con Presión Fiscal
  activa, +recaudación y −crecimiento en las 3 clases.
- **Mide:** nada en batch (el NPC no la activa) — el batch debe quedar bit-idéntico al Paso 1.

### Paso 3 — `MANTENIMIENTO.nivelParaOro: 3 → 2` — ✅ HECHO

- Una constante.
- **Medido (Pasos 1+2+3 juntos, 40 facciones, 1500 ticks):** `oroMedio` 1319 (Paso 1 solo) → **1130**. El
  sink involuntario del mantenimiento-oro a nivel 2 recorta ~14% de la inflación, pero sigue en ~4.6× la línea
  base (244). Los sinks voluntarios (Pasos 4-5) y sobre todo bajar las tasas de `IMPUESTOS` (el placeholder
  0.008/0.03/0.12 es claramente muy generoso) es lo que tiene que hacer el grueso en el Paso 6.
- Sin efecto en la línea base de 100 ticks (ningún asentamiento llega a nivel 2 tan pronto).

### Paso 4 — reclutamiento en oro — ✅ HECHO

- `RECLUTAMIENTO_ORO_POR_ESCALON` en `constants.ts` (1/2/4/7/11).
- `reclutarTropa` (`engine/tropas.ts`): añadir al `costoTotal` la línea
  `oro: RECLUTAMIENTO_ORO_POR_ESCALON[tropa.escalon] × cantidad` **salvo si `tropa.edificio === 'centroUrbano'`**
  (milicia exenta). El `factorCostoReclutamiento` **no** toca esa línea (decisión 14).
- `reclutarTropa` (`engine/tropas.ts`): línea `oro` en `costoTotal` salvo `tropa.edificio === 'centroUrbano'`.
  `factorCostoReclutamiento` no la toca (decisión 14). Mensaje de error ampliado a "equipo u oro".
- Regla de motor uniforme: el NPC también paga (decisión 13). Los 100 oro de fundación cubren el arranque
  militar temprano.
- Test motor (`reclutamientoOro.test.ts`): milicia sin oro; tropa de Barracón exige oro = escalón × soldados;
  sin oro, `reclutarTropa` lanza.
- **Mide (Paso 6):** `reclutamientosAcumulados`, `tropasVivas`, `conquistasAcumuladas`,
  `campamentosDestruidosAcumulados`. Es el Riesgo #1.

### Paso 5 — buey a oro (caravana #0 gratis: probada y quitada) — ✅ HECHO

- `ANIMAL_CATALOGO.buey.costo`: `{ madera: 30 }` → `{ oro: 12 }`. `construirCaravanaComercial` (NPC/lab) ahora
  cuesta 20 madera + 12 oro; el NPC lo paga (uniforme). Tests: ajustes en `caravanas.test.ts`.
- **La caravana #0 gratis se implementó, se midió y se quitó (calibración iter 1):** `crearCaravanaBootstrap`
  + detección del Mercado completado en `simulation.ts` (escaneando `construccion.edificio_completado` — sin
  cambiar la firma de `avanzarConstruccion`) + guarda de idempotencia. La medición de Pasos 1-5 la dejó
  `oroMedio` @1200 ≈ **987** (igual que sin los sinks): la #0 gratis hacía comerciar a todo asentamiento desde
  el tick 1 → más comisiones → más oro, cancelando los sinks de los Pasos 4-5. Con los 100 oro de fundación
  cubriendo el arranque, no compensaba. Código y test (`caravanaBootstrap.test.ts`) retirados.

### Paso 6 — campaña de calibración conjunta — EN CURSO

**Iteración 0 (Pasos 1-5, placeholders originales, 40 facciones):** `oroMedio` base 244 → ~987 (×4).
`vivos` ×2, `colapsados` ×3, `caravanasFundacionLanzadas` ×3, `acuerdosActivos` ×4. Mundo desbocado. (Medición
aproximada, no A/B rigurosa; máquina saturada, corridas de >1h.)

**Iteración 1 (2026-09-08):** `IMPUESTOS` a la mitad (0.004/0.015/0.06) + caravana #0 gratis quitada.
`oroMedio` de ×4 a ×1.5 la base, pero `vivos` cayó a 34 (< 47 base). El diagnóstico de `asentamiento.ruinas`
(ver más arriba) mostró que no era por muertes sino por menos expansión + el bug pre-existente de colapso de
madera.

**Iteración 2 (2026-09-08):** + el arreglo de `evaluarViabilidadFundacion` (bosque LIBRE). Medición
(40 facciones, 1500 ticks):

| @ tick 1500 | Base | Iter 1 | **Iter 2** |
|---|---|---|---|
| `oroMedio` | 244 | 372 | **532** (×2.2, sigue trepando 112→259→532) ⚠ |
| `vivos` | 47 | 34 | **61** (recuperado, por encima de la base) |
| ruinas (eventos) | 146 | 130 | **76** (−48%) |
| muertes por MADERA | ~124 | 90 | **21** (−77%) |
| vida mediana antes de caer | 78 min | 78 min | **310 min** (mueren maduros, no recién fundados) |
| nivel al caer | 84% niv 1 | 68% niv 1 | **72% niv 2** |
| `reclutamientosAcumulados` | 5597 | 3897 | **4661** |
| `campamentosDestruidos` | 147 | 146 | **146** (PvE intacto) |
| `pctSueloOcupado` | — | — | **25%** (mapa lejos de lleno) |

**Lo que dice iter 2:**
- **La supervivencia está arreglada.** Ruinas −48%, y las que quedan son asentamientos MADUROS (mediana 310
  min, 72% a nivel 2) que no pueden pagar oro/piedra de mantenimiento — muerte económica legítima, no "recién
  fundado sin madera". El colapso de madera pasó de dominante a residual (21).
- **El mapa solo está al 25% de ocupación** a tick 1500, meseteando. Hay sitio de sobra; las Facciones no se
  fundan pegadas (caja envolvente inicial 90% del mapa, vecino más cercano a 177).
- **El `oroMedio` volvió a subir** (532, ×2.2) porque más asentamientos vivos = más población = más
  recaudación. Sigue sin mesetear. **Es el problema pendiente #1 de la calibración.**
- De las 21 muertes de madera residuales, 17 son "bosque saturado por un vecino DESPUÉS de fundar" — el
  arreglo impide fundar sobre bosque ya lleno, no que un vecino lo llene luego. Cerrarlas = opción (c),
  reservar 1 slot de Leñera por asentamiento.

**Iteración 3 — pendiente: con la ocupación post-conquista (implementada 2026-09-08, Doc 5.12.9 /
`Ocupacion_Post_Conquista_Definicion.md`).** La ocupación mete un dampener nuevo sobre `oroMedio`: una plaza
recién conquistada recauda `× OCUPACION.factorRecaudacion` durante la ventana, y reponer la guarnición de
ocupación drena la mano de obra y el almacén de esa plaza. Además corta el ping-pong de conquistas (−70% en
la primera medición), lo que reduce el churn militar improductivo. Calibrar `OCUPACION.*` e `IMPUESTOS`
**juntos** en la próxima corrida (40 facciones / 1500 ticks) — la pregunta abierta #1 (`oroMedio` no mesetea)
puede aliviarse aquí de forma no trivial.

**Diagnóstico del `vivos` 34 (`BATCH_RUINAS_DIAG=1`, 2026-09-08):**

| ruinas | Base | Iter 1 |
|---|---|---|
| eventos totales | 146 | **130** (menos, no más) |
| mueren a nivel 1 | 84% | 68% |
| **por falta de madera** | **85%** | 69% |
| por falta de oro | 2% | 18% (3 → 23 eventos) |
| vida mediana antes de caer | 78 min | 78 min |

**Conclusiones:**

1. **El `vivos` 34 NO es por más muertes** — hay *menos* ruinas que en la base (130 < 146). La caída de
   `vivos` es de **expansión**: `caravanasFundacionLanzadas` 235 → 173. Lanzar una Caravana de Fundación
   cuesta **100 oro** (`costoCaravanaFundacion` incluye `FUNDACION.materialesIniciales`), y ahora ese oro
   compite con reclutar y comprar animales — los NPC no ahorran los 100 tan seguido. Es *el presupuesto
   funcionando*: fundar una colonia es una de las ambiciones entre las que el oro obliga a elegir. La
   pregunta de diseño: ¿−26% de expansión NPC es aceptable, o fundar debe quedar fuera del presupuesto de
   oro (quitar el oro de `costoCaravanaFundacion` — el asentamiento nuevo sigue naciendo con sus 100)?
2. **El oro del mantenimiento nivel 2 SÍ mató a algunos** (2% → 18%, +20 asentamientos) pero es un efecto
   pequeño en absoluto. No es el problema gordo.
3. **El problema gordo es PRE-EXISTENTE y no es de este bloque — DIAGNOSTICADO (`BATCH_RUINAS_DIAG=1`,
   2026-09-08):** 85% de los colapsos son por falta de madera a nivel 1, ~18 min después de la gracia. La
   causa raíz, ahora medida:
   - **Los bosques están.** El 100% de los asentamientos (40 iniciales + 114 hijos) tiene ≥1 bosque
     alcanzable al radio inicial (30). No es un problema de worldgen ni de dónde se funda.
   - **De los 90 muertos por madera, el 99% (89) nunca tuvo una Leñera** — ni siquiera en cola. Madera en
     almacén al morir: **0.0 de media** (secos).
   - **De esos 89: en 85 (96%) el único bosque de su zona está SATURADO por Leñeras de asentamientos
     vecinos.** `LENERA_POR_BOSQUE` topa 1/2/3 Leñeras por bosque según su tamaño; un bosque pequeño
     (capacidad 1) que un vecino ya trabaja deja al recién fundado sin nada. `bosqueParaLenera` no coloca
     Leñera en bosque lleno → la auto-construcción nunca encola una → el asentamiento se seca y cae.
   - **Viola directamente la regla "1 bosque debería bastar para que un nivel 1 sobreviva solo"** — porque
     hoy un bosque lo puede acaparar UN solo asentamiento.

   **ARREGLO APLICADO (2026-09-08, opción a — a petición del usuario):** `evaluarViabilidadFundacion`
   (`engine/settlement.ts`) gana `bosqueLibreAlcanzable` — descuenta las Leñeras que los asentamientos
   existentes ya tienen sobre cada bosque, y `recomendable` pasa a exigir un bosque con capacidad de Leñera
   LIBRE, no solo geométricamente alcanzable. Nuevo método `Mapa.hayBosqueLibreEnRadio`. Afecta a
   `buscarDestinoFundacionPorDefecto` (expansión) y `buscarPosicionFundacionInicialPorDefecto` (primer
   asentamiento). `bosqueAlcanzable` (geometría) se conserva para la UI. Suite 1167/1167, sin mover
   `snapshot_baseline`.

   **Medido (40 facciones, 1500 ticks):** muertes por madera **90 → 21 (−77%)**. De las 21 restantes, 17
   siguen siendo "bosque saturado" — casos donde el bosque se llenó DESPUÉS de fundar (un vecino levantó
   Leñeras más tarde sobre el bosque compartido). Para cerrar esas haría falta la opción (c) — garantizar
   1 slot de Leñera por asentamiento — o que la contención de bosque también empuje a los vecinos a otro.

   **Métrica nueva en el batch:** `Foto.pctSueloOcupado` (% del suelo habitable dentro de alguna zona de
   influencia) + diagnóstico único de la fundación inicial. **Los 40 iniciales NO se fundan pegados:** caja
   envolvente = 90% del mapa, distancia media al vecino más cercano = 177 (separación mínima 100). La
   hipótesis de "todas pegadas" queda descartada.

   Alternativas no tomadas: subir `LENERA_POR_BOSQUE`; garantizar 1 slot de Leñera por asentamiento con
   sobre-suscripción (candidata para las 21 residuales).

**Ajustes que quedan por probar (iteración 2+):**

1. Decidir sobre el oro de `costoCaravanaFundacion` (punto 1 arriba).
2. Si `oroMedio` sigue trepando: otro recorte de `IMPUESTOS`, o mirar de dónde sale (¿comisiones? ¿mina?).
3. Si el militar NPC cae demasiado (rec −30% vs base): bajar `RECLUTAMIENTO_ORO_POR_ESCALON` en escalón 1-2.
4. El colapso de madera a nivel 1 — pasada aparte, pre-existente.

**Método:** `git stash` + semilla fija + mismos tick-marks, una palanca por corrida, atribuir el delta
(igual que Etapas 1/2/6). Corridas secuenciales, no en pila.

**Objetivos:** `oroMedio` se estabiliza en una banda (no crece sin techo hasta tick 3000+), `colapsados` no
mucho peor que la base, el militar NPC sigue limpiando campamentos (`campamentosDestruidosAcumulados` no se
desploma), y — la prueba de diseño — un asentamiento de jugador no puede sostener ejército + flota + (futura)
intel a la vez.

- Al cerrar: actualizar la ficha §1, escribir el diario de la campaña, y actualizar el canon —
  `Docs/Game/3` §3.13.2 (animales, bootstrap), §3.5 (usos del oro), `Docs/Game/4` §4.5 (`nivelParaOro`),
  `Docs/Game/5` (coste de reclutamiento en oro), y `Revamp_Caravanas_Definicion.md` §17/§18.

### Fuera de este plan

Taberna + intel (`Taberna_Intel_Definicion.md`, sin diseñar) — es el sink recurrente, pero es un edificio +
un sistema de asset con revelado temporal, no cabe aquí. Se diseña y se implementa aparte; la calibración del
Paso 6 se hace **sabiendo que va a llegar** (deja margen de oro para él).
