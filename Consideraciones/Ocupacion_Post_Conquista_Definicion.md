# Ocupación post-conquista: diseño

> **DISEÑO CERRADO (2026-09-08), con plan técnico (§9, ponytail full). Pendiente de implementar.** Rondas 1-3
> de decisiones. Cierra la pregunta abierta de
> `Consideraciones/Preguntas_Abiertas.md` §1 ("cómo se conquista un asentamiento enemigo exactamente tras
> ganar el asedio") y el punto abierto #2 de `Movimiento_Ejercitos_Definicion.md`. Ata con el bloque
> `Economia_Del_Oro_Definicion.md` (§5, la guerra como sink) vía la recaudación reducida.
>
> **Revisa canon militar:** Doc 5.4/5.8 (reclutamiento ya no es solo-en-residencia) y Doc 5.12.4 ("la
> guarnición es lo único que defiende" — la guarnición ahora puede contener escuadrones de no-residentes,
> posados por un ejército).

## 0. El problema

`aplicarConquista` (`engine/combate.ts`) deja el asentamiento así: `faccionId` nuevo, **guarnición a 0
unidades** (escuadrones congelados, aún de los jugadores desalojados), `casasCompradas: []`,
`jugadoresFundadoresIds: []`, cargos vacíos. Población, edificios, almacén y murallas **intactos**.

Cadena de consecuencias:

1. Guarnición 0 → el siguiente `asediarConEjercito` ve `defensores.length === 0` → **`cae sin un solo
   defensor`, sin combate, sin RNG, sin bajas**.
2. Sin residentes → nadie puede reclutar ahí (`reclutarTropa` exige residencia; `npcGobernanza` también).
3. → El asentamiento queda indefenso **para siempre** y cambia de manos cada vez que pasa un ejército.
   Batch: 176 conquistas con ~61 asentamientos vivos = se reconquistan sin parar.

## 1. Decisiones cerradas

### Ronda 1 (2026-09-08)

1. **El ejército conquistador se convierte en la GUARNICIÓN del asentamiento conquistado** (`absorberColumna`):
   sus escuadrones pasan a `asentamiento.escuadrones`, el carro al almacén, el ejército se consume. Los
   escuadrones siguen siendo de sus jugadores (que NO residen ahí) — posar tropa en una plaza de tu Facción
   sin residir es una acción militar legítima (Ronda 2). Defienden, comen del trigo del asentamiento y se
   pueden reponer y re-movilizar por su dueño (§2.3b). Es el arreglo del ping-pong: tras conquistar hay
   guarnición REAL, no cero.
2. **Saqueo medio, con edificios RECONSTRUIBLES, no destruidos.** −25% de población (pesants+artesanos;
   nobleza protegida, huye/negocia); ~25% de los edificios pasan a `en_cola` **dañados** y hay que
   re-pagarlos PARCIALMENTE para que se reconstruyan (la auto-construcción los levanta de nuevo); las murallas
   pierden integridad (`avance` reducido). Centro Urbano nunca; ver §3 sobre supervivencia.
3. **La ocupación dura un tiempo FIJO** (~90 min de mundo, mismo orden que `MANTENIMIENTO.graciaMinutos`).
   Durante la ventana la guarnición recién instalada **sana y se repone** (los heridos expiran a los 30 min;
   el dueño o el NPC repone bajas de la población del asentamiento), y el asentamiento es **inmune a un nuevo
   asedio** — sin eso, una pila de ejércitos enemigos se lo tradearía cada tick sobre una guarnición aún
   maltrecha del asedio que acaba de librar.
4. **La ocupación reduce la recaudación de oro ~50%** durante la ventana. Conquistar no es un subidón
   inmediato de tesoro: es una inversión que tarda en rendir. Refuerza el bloque de economía del oro.

### Ronda 2 (2026-09-08) — reclutamiento y guarnición

5. **Reclutar ESCUADRÓN NUEVO o cambiar de composición: solo en tu residencia.** Sin cambios respecto a hoy.
6. **En otra plaza de tu Facción, estando presente y con el permiso del asentamiento** (`politicaDeAcceso` ≠
   `cerrado`, sin veto): **solo REPONER** un escuadrón que ya tienes ahí —posado en su guarnición o en tu
   columna— hasta su tope. Gasta la población y el almacén de esa plaza (equipo + oro + reserva de trigo),
   autolimitado igual que `repostarSiPuede` limita el trigo. Ni escuadrón nuevo, ni cambio de roster.
7. **Un ejército puede POSAR sus escuadrones en la guarnición de cualquier plaza de su Facción** — marchando
   allí y guarneciéndola (`guarnecer`), o automáticamente al conquistar (decisión 1). Los escuadrones de
   no-residentes en una guarnición defienden, comen trigo del almacén y **su dueño los puede re-movilizar**
   (relajación del gate de `movilizarEjercito`: residir O tener escuadrones vivos propios posados ahí).
8. **Consolidar de verdad = mover la base allí** (comprar casa + abandonar la anterior). Da reclutamiento
   pleno, cargos y recaudación al 100%. Para el NPC da igual (nunca consolida): sostiene por la guarnición
   mientras la mantenga a flote.

### Ronda 3 (2026-09-08)

9. **Se añade el comando `cambiarResidencia(destinoId)`** (§2.3c) — atómico: dejar la residencia actual +
   tomar la nueva. Es canon de Doc 2.5/2.6 pero prerrequisito de la consolidación de jugador de una conquista.

## 2. El modelo

### 2.1 El estado

Un campo en `Asentamiento` (`domain/types.ts`):

```ts
/** Instante en que termina la ocupación militar tras una conquista (Doc 5.4). Mientras `instante < ocupacionHasta`:
 *  el asentamiento es INMUNE a un nuevo asedio, recauda oro reducido, crece más lento y su medidor de
 *  mantenimiento no degrada. Ausente = no ocupado (caso normal). */
ocupacionHasta?: Instante;
```

Un solo campo basta para la versión de tiempo fijo. Se comprueba AL LEER (patrón de `enTregua`/`heridoHasta`),
salvo el fin de la ventana, que sí necesita un tick que lo resuelva (§2.5).

### 2.2 Qué hace `aplicarConquista` (una vez, al conquistar)

Cambia respecto a hoy:

- **Los escuadrones congelados de los desalojados YA NO se quedan a 0 en la guarnición** — se sacan (siguen
  siendo de sus jugadores, ahora HUÉRFANOS, y "existen" solo como identidad recuperable, Doc 5.4). La
  guarnición del asentamiento conquistado la forman **los escuadrones del ejército conquistador**
  (`absorberColumna(asentamientoConquistado, ejercitoConquistador, devolverSuministro: true)` — la MISMA
  operación que un ejército replegado al llegar a casa, solo que el destino no es su hogar). El carro del
  ejército → almacén del asentamiento.
- `faccionId` → conquistador; cargos vacíos; residencia (`casasCompradas`, `jugadoresFundadoresIds`) vacía.
- **Saqueo de población:** `pesants` y `artesanos` × `(1 - OCUPACION.fraccionSaqueoPoblacion)` (0.25 placeholder).
  Nobleza intacta.
- **Daño a edificios:** una fracción `OCUPACION.fraccionEdificiosDanados` (0.25) de los edificios `activo`
  —elegidos por un orden determinista, NO aleatorio dentro de `aplicarConquista` que es puro/sin rng— pasa a
  `estado: 'en_cola'` con un flag `danado: true`. **Exentos:** Centro Urbano siempre; y al menos una Granja y
  una Leñera activas (ver §3). Un edificio dañado se reconstruye por la vía normal de auto-construcción, pero
  al comprometerlo se cobra solo `OCUPACION.fraccionCosteReconstruccion` (0.5) del costo de catálogo y tarda
  esa misma fracción de tiempo.
- **Daño a murallas:** cada `Recinto` completo baja su `avance` en `OCUPACION.fraccionDanoMuralla` (0.3) de
  `celdas.length` — la muralla no cae, pero deja de dar el multiplicador defensivo pleno hasta repararse
  (la reparación de recintos ya existe, §7 del doc de murallas).
- **Reinicio de mantenimiento:** `medidorMantenimiento: 100`. La degradación queda suspendida mientras dure la
  ocupación (§2.4) — el conquistador hereda una ciudad rota, no una en déficit inmediato.
- **Set `ocupacionHasta`** = `sumar(instante, minutos(OCUPACION.duracionMinutos))`.

### 2.3 `guarnecer` — posar escuadrones en una plaza de tu Facción

> **El comando `guarnecer` es follow-up, NO va en el plan de §9** (revisión ponytail): la conquista guarnece
> sola. Esta sección describe la capacidad general; se implementa aparte cuando toque "defender una plaza
> propia marchando".

Capacidad general, no solo de la conquista (Ronda 2, decisión 7). Un ejército propio en la puerta de una
plaza de su Facción puede **guarnecerla**: sus escuadrones pasan a `asentamiento.escuadrones` (el ejército se
consume, carro → almacén). Es `absorberColumna` con destino ≠ hogar. Sirve para reforzar una frontera o
defender una plaza amenazada sin que el jugador mueva su residencia — cierra el hueco de "proteger una plaza
propia marchando a defenderla" (Doc 5.12.4, hoy un ejército aparcado no ayuda a defender).

Al conquistar, esto pasa **automáticamente** (§2.2). En una plaza que ya es tuya, es un **comando** del
jugador (el NPC guarnece automáticamente al conquistar; rotar guarniciones en frontera es pasada posterior).

### 2.3b Reponer y re-movilizar escuadrones de no-residentes

Un escuadrón en la guarnición de una plaza donde su dueño NO reside:

- **Defiende** — está en `asentamiento.escuadrones`, `asediarConEjercito` lo cuenta sin lógica nueva.
- **Come** — trigo del almacén del asentamiento, `avanzarMantenimientoTropas` sin cambios.
- **Se repone** — su dueño, estando presente en la plaza, repone bajas de la población y el almacén de esa
  plaza (Ronda 2, decisión 6). El NPC lo hace automáticamente para la guarnición de una plaza que conquistó y
  quiere conservar (mientras haya población/almacén). No puede reclutar un escuadrón NUEVO ahí — eso es
  residencia.
- **Se re-moviliza** — su dueño puede sacarlo a campaña otra vez. Gate de `movilizarEjercito` relajado:
  `esResidente(a, j)` **O** `a.escuadrones.some(e => e.jugadorId === j && e.cantidad > 0)`. "Mueves tu propia
  tropa esté donde esté"; solo reclutar/cambiar roster sigue atado a residir.

### 2.3c El comando `cambiarResidencia` (Ronda 3)

Canon de Doc 2.5/2.6, no de la ocupación — pero surge de aquí y es su prerrequisito de consolidación de
jugador. Hoy `comprarCasa` (`engine/faccion.ts`) rechaza si el jugador `resideEnOtroAsentamiento`, y no hay
comando para dejar una residencia (Doc 2.6 lo marca como limitación conocida).

**`cambiarResidencia(destinoId)`** — atómico: dejar la residencia actual + tomar la nueva.

- **Precondiciones:**
  - El jugador reside en algún asentamiento (si no, es `comprarCasa` a secas).
  - `destino.faccionId` = la Facción del jugador (solo se reside en plaza propia).
  - `destino` tiene hueco de vivienda (`capacidadCasas`) y **lo permite** (`politicaDeAcceso` ≠ `cerrado`, sin
    veto) — mismo gate que reclutar/entrar.
  - **Presencia** en el destino cuando exista el jugador situado (§13b); hasta entonces, gateado por "puede
    cruzar la puerta".
  - **Cooldown** (`CIUDADANIA.cooldownCambioResidenciaDias`, placeholder) — evita el salto de residencia cada
    vez que se conquista algo para exprimir valor.
- **Efectos, residencia vieja:**
  - Fuera de `casasCompradas` **y** de `jugadoresFundadoresIds` — ya no reside por ninguna vía; el hueco de
    vivienda se libera.
  - **Cargos locales vacíos** (no se gobierna donde no se vive — misma regla que la conquista).
  - **Los escuadrones propios posados en su guarnición SE QUEDAN** — pasan a ser guarnición de no-residente
    (§2.3b): siguen defendiendo, el dueño los repone y los re-moviliza. No se pierde tropa por mudarse.
- **Efectos, residencia nueva:** exactamente `comprarCasa` (añadir a `casasCompradas`; la ciudadanía de
  Facción no cambia — es la misma Facción).
- **Huérfano** (le conquistaron su única plaza): no tiene residencia de la que salir → usa `comprarCasa` /
  `unirseAFaccion`, no este comando.

Con esto, la consolidación de una conquista por un jugador es: marchar, guarnecer/conquistar, y
`cambiarResidencia` a la plaza tomada — a partir de ahí recluta escuadrones nuevos ahí, ejerce cargos y la
recaudación es del 100% (aunque la ventana de ocupación siga corriendo su tiempo).

### 2.4bis Fin de la campaña del ejército — ya no aplica

`replegarLosQueYaTerminaron` (`npcGobernanza`) repliega ejércitos `estacionado` tras conquistar. Como ahora el
ejército conquistador se **consume en la guarnición** (§2.2), no queda ejército que replegar. El filtro de
`lanzarCampanas` (no salir de un asentamiento ocupado) sí se mantiene: la guarnición recién instalada no debe
volver a salir de campaña hasta que la ventana venza y esté repuesta.

### 2.4 Qué hace la ventana de ocupación mientras está activa

| Efecto | Dónde | Valor placeholder |
|---|---|---|
| **Inmune a asedio** | `iniciarAsedio` / `asediarConEjercito`: si el objetivo está ocupado → rebota (`asedio_resistido`, sin combate ni RNG); el atacante acampa y (NPC) se repliega. Protege a la guarnición recién instalada mientras sana (heridos a 30 min) y se repone. | — |
| **Recaudación de oro reducida** | `recaudacionOro` (necesita `instante`): `base × OCUPACION.factorRecaudacion` | 0.5 |
| **Crecimiento de población reducido** | `crecerPoblacion` (necesita `instante`): el factor `felicidad` × `OCUPACION.factorCrecimiento` | 0.5 |
| **Mantenimiento no degrada** | `avanzarMantenimiento`: misma rama que el período de gracia (`graciaMinutos`), extendida a "o está ocupado" | — |

### 2.5 El fin de la ventana

En la pasada por asentamiento de `avanzarSimulacion`, si `a.ocupacionHasta !== undefined && instante >= a.ocupacionHasta`:

- Se limpia `ocupacionHasta` (`{ ...a, ocupacionHasta: undefined }`).
- El asentamiento vuelve a las reglas normales: recaudación plena, crecimiento normal, mantenimiento degrada,
  se puede asediar.
- **La guarnición (los escuadrones del ex-ejército conquistador) SE QUEDA** — no se retira nada. El
  asentamiento sigue defendido por ella, de forma indefinida, mientras su dueño la reponga y el trigo alcance.
  El ping-pong queda cortado: quien quiera reconquistar tiene que ganar un asedio de verdad contra esa
  guarnición, con sus bajas (→ oro para reponer).
- **Consolidación plena** (reclutar escuadrones nuevos ahí, cargos, recaudación 100%): mover la base
  (§8 — falta el comando "cambiar de residencia"). Para el NPC nunca pasa, pero **no lo necesita**: sostiene
  por la guarnición-ejército mientras la mantenga a flote. Solo la perdería si la deja morir de hambre o si un
  enemigo gana el asedio.

## 3. Supervivencia: no matar la ciudad saqueada

Riesgo: −25% población + 25% edificios dañados + murallas dañadas, todo a la vez, podría colapsar un
asentamiento pequeño pese al respiro de mantenimiento. Mitigaciones en el diseño:

- **Granja y Leñera exentas del daño** (al menos una activa de cada) — sin comida ni madera el saqueo es una
  sentencia, no una penalización.
- **Mantenimiento suspendido** toda la ventana (§2.4) — 90 min para reconstruir sin presión de déficit.
- **Reconstrucción barata** (`fraccionCosteReconstruccion` 0.5) y rápida — la auto-construcción prioriza
  supervivencia, así que Granja/Leñera dañadas (si alguna extra lo fue) se levantan primero.
- Calibración por batch: si aun así colapsa de más, bajar `fraccionEdificiosDanados` o `fraccionSaqueoPoblacion`.

## 4. Constantes nuevas — `OCUPACION` en `constants.ts`, PLACEHOLDER

```ts
export const OCUPACION = {
  duracionMinutos: 90,
  fraccionSaqueoPoblacion: 0.25,   // pesants+artesanos; nobleza intacta
  fraccionEdificiosDanados: 0.25,  // de los `activo`, excluido Centro Urbano + 1 Granja + 1 Leñera
  fraccionCosteReconstruccion: 0.5,
  fraccionDanoMuralla: 0.3,        // reducción de `avance` sobre `celdas.length`
  factorRecaudacion: 0.5,          // oro durante la ocupación
  factorCrecimiento: 0.5,          // crecimiento de población durante la ocupación
};
```

## 5. Lo que ya existe y no hay que inventar

- **`ocupacionHasta` es un `Instante` opcional** — mismo patrón exacto que `enTreguaHasta` (Ejercito),
  `heridoHasta` (Escuadron), `ultimaCaravanaCreadaEn` (Asentamiento). Se lee con `instante < campo`.
- **El período de gracia de mantenimiento** (`avanzarMantenimiento`, rama `transcurrido(fundadoEn, ...) <
  graciaMinutos`) es el molde exacto de "el mantenimiento no degrada mientras X".
- **La reconstrucción de un edificio dañado** = un `en_cola` que la auto-construcción ya sabe procesar; solo
  cambia cuánto cuesta comprometerlo.
- **La reparación de recintos** (`Recinto.mejorandoA`/`avance`, doc de murallas §7) ya sube `avance` con obra
  — un `avance` bajado por el saqueo se repara por esa misma vía.
- **`absorberColumna`** (`engine/ejercitos.ts`) ya hace exactamente "escuadrones del ejército → guarnición,
  carro → almacén". Hoy solo se llama al replegar a casa; guarnecer/conquistar es la misma llamada con otro
  destino.
- **`avanzarMantenimientoTropas`** (ración de la guarnición) itera `asentamiento.escuadrones` sin mirar de
  quién son — un escuadrón de no-residente en la guarnición come igual, sin cambio.
- **`repostarSiPuede`** (trigo del ejército desde una plaza propia al alcance, autolimitado por la reserva de
  comida de la plaza) es el molde exacto de la reposición de escuadrones en campaña (Ronda 2, decisión 6):
  autolimitada por `poblacionDisponibleParaReclutar` + reserva de trigo + equipo/oro del almacén.

## 6. Separación de capas

- `OCUPACION` → `constants` (solo datos).
- `aplicarConquista` (+ `absorberColumna` al conquistar), `recaudacionOro`, `crecerPoblacion`,
  `avanzarMantenimiento`, el guard de asedio, el fin de ventana, `guarnecer`, la reposición en campaña, el
  gate relajado de `movilizarEjercito` → `engine` (funciones puras sobre `Asentamiento`/`Ejercito`).
- El gate de reclutamiento (`reclutarTropa`: residencia para escuadrón nuevo; misma-Facción + permiso para
  reponer) → `engine/tropas.ts` + `engine/pertenencia.ts` (helper `puedeReclutarEn`/`puedeReponerEn`).
- Los comandos `guarnecer` y "reponer en campaña" → `session/comandos/` (envoltorio del motor).
- El criterio del NPC (guarnecer al conquistar, reponer la guarnición de una plaza conquistada, no salir de
  campaña de un asentamiento ocupado) → `session/npcGobernanza.ts`.
- **Migración de snapshot:** `ocupacionHasta` opcional, ausente = no ocupado. `danado` opcional en `Edificio`,
  ausente = sano. **Sin migración** — las partidas viejas no tienen conquistas en curso.

## 7. Invariantes

- `ocupacionHasta` es derivado del combate, nunca se guarda un "estado de ocupación" separado — el `Instante`
  ES el estado. La ventana es **puro tiempo**: no se acorta ni se cancela por nada (revisión ponytail).
- **Un asentamiento ocupado no puede ser conquistado** — el guard de asedio es incondicional durante la ventana.
- **Tras conquistar SIEMPRE hay guarnición** (los escuadrones del ejército conquistador) — nunca se queda a 0.
- Un escuadrón en una guarnición defiende, come y cuenta para el asedio **mire quien mire de quién es** — no
  hay lógica que distinga "escuadrón de residente" de "escuadrón posado por un ejército".
- **Reclutar un escuadrón NUEVO sigue exigiendo residir**; solo la reposición y el movimiento de escuadrones
  propios se desatan de la residencia.
- El saqueo **nunca toca** Centro Urbano ni deja al asentamiento sin al menos una Granja y una Leñera activas.
- El saqueo es **determinista** (orden por id/tipo, no rng) — `aplicarConquista` es pura y sin `RandomFn`.
- Sin conquistas ni `guarnecer` en curso, el comportamiento del motor no cambia.

## 8. Puntos abiertos

- **`cambiarResidencia`: cooldown y coste** — `CIUDADANIA.cooldownCambioResidenciaDias` es placeholder; falta
  decidir si además cuesta oro/recursos y cuánto. §2.3c.
- **Números** — todo `OCUPACION.*` es placeholder, a calibrar en batch (¿el ping-pong baja de verdad? ¿los
  asentamientos saqueados colapsan de más? ¿la guerra de conquista se vuelve irracional para el NPC?).
- **Consolidación explícita** — una mecánica dedicada (enviar colonos, un coste de "anexión" que hace la
  posesión permanente y acorta la ventana) es candidata para después.
- **Rotación de guarnición del NPC** — un NPC que quiere reforzar una frontera con `guarnecer` fuera de una
  conquista. Pasada posterior de `npcGobernanza`.
- **Revuelta** — un asentamiento ocupado/sin residentes con "felicidad" baja podría revertir a independiente o
  a la Facción anterior. Mecánica más grande, fuera de este pase.
- **Re-conquista durante la ventana con fuerza abrumadora** — descartado por ahora (inmunidad pura). Si el
  playtest lo pide, la ventana pasa de inmunidad a multiplicador defensivo decreciente.
- **`movilizarEjercito` desde una plaza donde no resides pero tienes guarnición** — el `origenAsentamientoId`
  del ejército nuevo es esa plaza, así que se repliega ahí, no a la residencia del jugador. Coherente (la
  tropa vuelve de donde salió), pero conviene confirmarlo.

## 9. Plan técnico (ponytail full)

> **Revisión ponytail — lo que se recorta del diseño para este pase:**
>
> - **`guarnecer` como comando aparte se SACA.** El arreglo del ping-pong no lo necesita: la conquista guarnece
>   sola (`absorberColumna` en el sitio del asedio). `guarnecer` (marchar a una plaza propia y volcar los
>   escuadrones) es la capacidad general "defender una plaza propia marchando" + el matiz de canon de Doc
>   5.12.4 — **follow-up propio**, no bloquea nada.
> - **"Reponer los escuadrones de una columna de paso" se SACA.** La guarnición de ocupación se repone con
>   `reclutarTropa` (gate relajado, Paso 2) porque sus escuadrones YA están en `asentamiento.escuadrones`. La
>   reposición de una columna que solo pasa por una plaza amiga es otra capacidad — follow-up.
> - **La ventana de ocupación es PURO TIEMPO.** Se retira el invariante "termina si el ejército de ocupación
>   desaparece" (Ronda 1): ya no hay "ejército de ocupación" separado — la guarnición es del asentamiento. Si
>   la guarnición cae durante la ventana, el asentamiento sigue inmune hasta que el reloj vence; el atacante
>   rebota y lo toma cuando pueda. Un caso de borde raro, sin código extra.
> - **`Edificio.danado`**: un `boolean?` opcional y UNA multiplicación en el punto donde `avanzarConstruccion`
>   cobra un `en_cola`. Nada de estado de "reparación" nuevo — un edificio dañado ES un `en_cola` más barato.
>
> Lo demás se apoya en lo que ya hay: `absorberColumna`, el molde del período de gracia, el patrón `*Hasta:
> Instante` opcional. Ver §5.

Método de siempre: cada paso cambia una cosa medible, `git stash` + semilla fija para atribuir el delta.
**Sin migración de snapshot** (`ocupacionHasta`/`danado` opcionales, ausente = normal; partidas viejas no
tienen conquistas en curso). Capas: todo `engine` + `constants` salvo los comandos nuevos
(`session/comandos/`) y el criterio del NPC (`session/npcGobernanza.ts`).

### Paso 1 — reclutamiento desatado de residencia — ✅ HECHO (2026-09-08)

- `puedeReclutarEn(a, jugadorId, faccionDelJugadorId)` en `engine/pertenencia.ts`:
  `esResidente → 'todo'` · `faccionDelJugador === a.faccionId && !vetado && politicaDeAcceso !== 'cerrado' →
  'solo_reponer'` · resto `→ 'no'`.
- `reclutarTropa` (`engine/tropas.ts`) recibe `faccionDelJugadorId`; si el permiso es `'solo_reponer'`,
  rechaza si el escuadrón `jugadorId+tropaId` no existe ya en `a.escuadrones` (no hay escuadrón nuevo).
- `movilizarEjercito` (`engine/ejercitos.ts`): gate `esResidente(a,j) || a.escuadrones.some(e => e.jugadorId
  === j && e.cantidad > 0)`.
- Ajustar los llamadores (`session/comandos/militar.ts`, `npcGobernanza.reclutarParaTodos` → itera miembros
  de Facción, no `residentesDe`) y los tests que asumían "solo residente recluta".
- **Independiente:** se puede shippear solo (mejora ya de por sí — una columna herida repone en una plaza
  propia).

### Paso 2 — `cambiarResidencia` — ✅ HECHO (2026-09-08)

- `cambiarResidencia(facciones, asentamientos, destinoId, jugadorId)` en `engine/faccion.ts` (hermana de
  `comprarCasa`) → `{ origen, destino }`. Valida (reside en algo distinto; ciudadano de `destino.faccionId`;
  hueco de vivienda; no vetado; `politicaDeAcceso` ≠ `cerrado`; no residir ya ahí). Deja la vieja: fuera de
  `casasCompradas` Y `jugadoresFundadoresIds`, cargos locales vacíos. Los escuadrones posados en la vieja NO
  se tocan.
- Comando `cambiarResidencia` (`session/comandos/cargos.ts`) + esquema + autorización + registro. Evento
  `ciudadania.residencia_cambiada` (`PayloadResidenciaCambiada`).
- Tests: `faccion.test.ts` (mudanza libera fundador+casa+cargo, escuadrones intactos, huérfano rechazado, otra
  Facción rechazada). `api.test.ts` contrato: 65 → 66 comandos.
- **Sin cooldown todavía** (`ponytail:` — necesita `Jugador.ultimoCambioResidenciaEn` + jugadores reales
  §13b; `CIUDADANIA.cooldownCambioResidenciaDias` sigue reservado en §8, añadir cuando el abuso muerda).
- **Independiente.**

### Paso 3 — `OCUPACION` + los dos campos — ✅ HECHO (2026-09-08)

- `OCUPACION` en `constants.ts` (§4). `Asentamiento.ocupacionHasta?: Instante`, `Edificio.danado?: boolean`.
- Helper `estaOcupado(a, instante)` en `engine/asentamientoQuery.ts` (`a.ocupacionHasta !== undefined &&
  instante < a.ocupacionHasta`).
- Sin efecto todavía — solo los tipos y el helper. Nada que medir.

### Paso 4 — `aplicarConquista` reescrito (el corazón) — ✅ HECHO (2026-09-08)

`aplicarConquista(defensor, faccionConquistadoraId, guarnicionEntrante: Escuadron[], suministroEntrante, instante)`
— firma con escuadrones + carro sueltos, no un `Ejercito`, para servir a los dos caminos (comando sin
ejército / llegada de ejército). Cambios frente al plan original: el `absorberColumna` se hace inline (es
`[...guarnicionEntrante]` + `agregarRecurso` del carro); `iniciarAsedio` filtra los escuadrones seleccionados
de la guarnición del atacante y los pasa como `guarnicionEntrante`; `asediarConEjercito` devuelve
`ejercitoConsumido: boolean` y `avanzarEjercitos` hace `continue` (sin `supervivientes.push`) emitiendo
`ejercito.guarnece_conquista` + perdiendo las caravanas adjuntas igual que un ejército deshecho. Tests
reescritos en `combate.test.ts` y `avanzarEjercitos.test.ts` (guarnición del conquistador ≠ 0, saqueo de
población/edificios/murallas, exentos Centro Urbano + 1 Granja/1 Leñera, determinismo, ventana abierta).

Plan original, para referencia:

`aplicarConquista(defensor, faccionConquistadoraId, ejercitoConquistador, instante)`:

1. Escuadrones congelados de los desalojados: **fuera** (huérfanos, Doc 5.4) — no se quedan a 0.
2. `absorberColumna(defensor, ejercitoConquistador, devolverSuministro: true)` → sus escuadrones son la
   guarnición, el carro al almacén. (Para el camino `iniciarAsedio` —atacante = asentamiento, sin ejército—
   los escuadrones atacantes SELECCIONADOS se mueven a la guarnición del conquistado y salen de la del
   atacante. Mismo principio.)
3. `faccionId`, cargos vacíos, residencia vacía (como hoy).
4. **Saqueo** (determinista, sin `RandomFn`): `pesants`/`artesanos` × `(1 - fraccionSaqueoPoblacion)`;
   ordenar `edificios` `activo` por id, excluir Centro Urbano + primera Granja + primera Leñera, marcar
   `danado: true` y `estado: 'en_cola'` a los primeros `ceil(fraccionEdificiosDanados × restantes)`; cada
   `Recinto` completo baja `avance` en `floor(fraccionDanoMuralla × celdas.length)`.
5. `medidorMantenimiento: 100`. `ocupacionHasta = sumar(instante, minutos(OCUPACION.duracionMinutos))`.

`asediarConEjercito`/`iniciarAsedio` pasan `instante` y (el primero) el ejército; devuelven señal de "ejército
consumido" para que `avanzarEjercitos` lo suelte sin evento `disuelto`.

Tests: hay guarnición tras conquistar (≠ 0); población baja la fracción; N edificios `danado`; Centro Urbano y
1 Granja/1 Leñera intactos; murallas con `avance` reducido; determinista (misma entrada → misma salida).
**Mide:** batch — `conquistasAcumuladas` y conquistas por asentamiento único (¿baja el ping-pong?),
supervivencia de saqueados.

### Paso 5 — guard de asedio — ✅ HECHO (2026-09-08)

`iniciarAsedio`/`asediarConEjercito`: si `estaOcupado(defensor, instante)` → rebota (`asedio_resistido`, sin
combate, **sin tocar el RNG**), el atacante acampa. Tests: inmunidad por el comando y por llegada de ejército
(un segundo ejército el mismo tick rebota sin bajas).

### Paso 6 — efectos de la ventana — ✅ HECHO (2026-09-08)

- `recaudacionOro(a, instante?)` y `crecerPoblacion(a, rng, instante?)` — param OPCIONAL (no obligatorio, para
  no romper los ~5 tests que los llaman sin él): ausente = sin comprobación de ocupación. Con `instante` y
  `estaOcupado` → `× OCUPACION.factorRecaudacion` / el factor `felicidad` `× OCUPACION.factorCrecimiento`.
- `avanzarMantenimiento`: la rama de gracia pasa a `... || estaOcupado(a, instante)`.
- `simulation.ts` pasa `instante` a `recaudacionOro` y `crecerPoblacion`.
- El cliente (`gameStore.recaudacionInfo`) NO se tocó — no tiene `instante` a mano y es solo preview; la
  recaudación mostrada durante la ocupación queda ligeramente optimista (`ponytail:` — arreglar si el cliente
  gana un banner de ocupación).
- **Mide:** batch — `oroMedio`.

### Paso 7 — reconstrucción de lo dañado — ✅ HECHO (2026-09-08)

En `avanzarConstruccion`, **Paso 2 (arranque de obra)**: un `en_cola` normal ya está pagado; uno `danado` NO
(el saqueo lo encoló sin cobrar), así que se cobra ahí `ceil(costo_catálogo × OCUPACION.fraccionCosteReconstruccion)`
y `ticks × esa fracción`. Si no hay recursos, espera en cola. Al pasar a `activo` se limpia `danado`. Evento
`construccion.iniciada` dice "reconstrucción" en vez de "construcción".

### Paso 8 — fin de la ventana — ✅ HECHO (2026-09-08)

En la pasada por asentamiento de `avanzarSimulacion`, tras `avanzarMantenimiento`: si `ocupacionHasta !==
undefined && instante >= ocupacionHasta` → `{ ...a, ocupacionHasta: undefined }` + evento
`asentamiento.ocupacion_terminada`. La guarnición se queda. (Tests 7-8 juntos en
`eventosDominioConstruccion.test.ts`.)

### Paso 9 — criterio del NPC — ✅ HECHO (2026-09-08)

- `lanzarCampanas`: `if (estaOcupado(origen, instante)) continue;` — guarnición recién instalada no sale.
- `reclutarParaTodos`: ya cubierto por el Paso 1 (itera dueños de escuadrones posados, no solo residentes) —
  el NPC repone la guarnición de una plaza conquistada sin residentes sin código nuevo.
- `replegarLosQueYaTerminaron`: sin cambios — tras conquistar el ejército ya no está en la lista (lo soltó
  `avanzarEjercitos`), así que la función simplemente tiene menos que procesar. No rompe.
- **Mide:** batch — militar NPC.

### Paso 10 — campaña de calibración — 🔶 EN CURSO (2026-09-08)

Iterar `OCUPACION.*` contra: el ping-pong baja de verdad (conquistas por asentamiento único ↓), los saqueados
no colapsan de más (`colapsados` ~igual), la guerra de conquista no se vuelve irracional para el NPC
(`conquistasAcumuladas` no se desploma a 0), y el efecto neto en `oroMedio`. Diario de la campaña.

**Instrumentación añadida (`BATCH_OCUPACION_DIAG=1`, commit `9ae7c12`):** línea de progreso por `FOTO_CADA`
ticks (plazas/ocupadas/ejércitos/caravanas/conquistas + wall-clock) y, al final, la distribución
"veces conquistada → nº de plazas".

**Primera medición (30 facciones, mismo seed, baseline `f844f7a` vs `c2f21f4`, valores placeholder):**

| tick | baseline: t / plazas / conquistas | ocupación: t / plazas / conquistas |
|---|---|---|
| 300 | 30 s / 38 / 12 | 31 s / 38 / **6** |
| 500 | 53 s / 35 / 25 | 57 s / 35 / **10** |
| 700 | 83 s / 43 / 42 | 85 s / 39 / **13** |
| 900 | 125 s / 50 / 60 | 130 s / 47 / **17** |

- **Ping-pong: −70 %** (a tick 900, 17 cambios de dueño vs 60). El NPC sigue conquistando (no cae a 0) —
  deja de re-tomar la misma plaza cada tick.
- **Coste por tick idéntico** (±5 %): la ocupación no infla el mundo ni el batch. El default 100 fac / 3000
  ticks es ~20× el tamaño de calibración (40 / 1500) — de ahí la corrida de 1h20, no una regresión.
- **Pendiente:** corrida limpia 40/1500 con máquina descargada para `oroMedio`, `colapsados`, la
  distribución de reconquistas y el diario. La contención de CPU (tarea en paralelo) bloqueó la corrida final.

### Independencias

- Pasos **1 y 2** son mejoras autónomas — se pueden shippear antes que el resto.
- Pasos **3-9** son el bloque de ocupación, en orden (cada uno mide una cosa).
- `guarnecer` (comando) y "reponer columna de paso" — follow-ups fuera de este plan.

## 10. Canon a actualizar al cerrar

- **Doc 5.4 / 5.8:** reclutar escuadrón nuevo = residencia; reponer/mover escuadrones propios = cualquier plaza
  de la Facción con permiso, estando presente. Guarnición puede contener escuadrones de no-residentes.
- **Doc 5.12.4:** "la guarnición es lo único que defiende" — matizado: la guarnición puede ser una fuerza
  posada por un ejército (`guarnecer`), no solo tropa de residentes. Cierra "proteger una plaza marchando".
- **Doc 5.4 (conquista):** qué pasa exactamente — guarnición del conquistador, saqueo, ventana de ocupación.
- **Doc 2.5 / 2.6:** el "beneficio de ciudadanía: reclutar" se matiza; anotar el comando "cambiar de
  residencia" pendiente.
- **`Preguntas_Abiertas.md` §1** ("cómo se conquista exactamente") — resuelto.
- **`Movimiento_Ejercitos_Definicion.md`** punto abierto #2 — resuelto.
- **`Economia_Del_Oro_Definicion.md` §5** — nota: la ocupación reduce recaudación; la guerra de conquista
  drena oro (reponer guarniciones y bajas de asedio).
9. Canon: Doc 5.4 (qué pasa exactamente al conquistar), `Preguntas_Abiertas.md` §1, `Movimiento_Ejercitos`
   punto abierto #2, nota en `Economia_Del_Oro_Definicion.md` §5.
