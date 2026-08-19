# Vista de Asentamiento — Trazado urbano dinámico

Documento vivo. Recoge el estado actual del diseño; lo que sigue sin implementar va al final, marcado como
abierto.

## Principio rector

Nada del trazado urbano se define al fundar el asentamiento. Ni calles, ni manzanas, ni reservas de espacio.
Todo emerge pieza por pieza, a medida que se construyen edificios.

## Estado (2026-08-19): diseño cerrado, CERO código de anclas escrito todavía

§1-12 son la especificación completa de "Anclas y satélites" — revisada por consejo, corregida en varias
rondas con el usuario, cerrada. Pero **nada de eso está implementado**: no existe todavía ni un solo tipo de
ancla nuevo (Plaza, Plaza de Armas, Patio de Gremios, Carpintería-zona), ni el anillo generalizado, ni la
atracción dura. `sitioEnBarrio`/`redDeCalles` siguen exactamente como estaban antes de esta sesión.

Lo que sí se hizo hoy fue el **Plan de 4 etapas** (Etapa 0: instrumentar el batch para tener línea base;
Etapa 1: reanclar `distanciaAlBarrio` a la instancia más cercana, solo con anclas ya reales; Etapa 2: anillo +
atracción dura; Etapa 3: marcadores gratis) y la Etapa 0 se puso en marcha — pero **se salió del carril
casi de inmediato**: la primera corrida de línea base mostró **0 asentamientos alcanzando nivel 2 en 3000
ticks**, lo que dejaba `componentes.industria`/`componentes.militar` en `null` para siempre — sin eso, no hay
línea base real contra la que medir la Etapa 1. El resto de la sesión fue diagnosticar y arreglar POR QUÉ,
tres causas independientes y ninguna era del trazado:

1. **Gate de nivel 3 inalcanzable sin jugador humano** — `issues/nivel_3_inalcanzable_sin_jugador_humano.md`.
   RESUELTO: la gobernanza NPC ahora añade Barracón/Galería a mano.
2. **Selección de posiciones de fundación del batch, ciega a la piedra** — RESUELTO: el batch ahora reutiliza
   el mismo filtro duro + barrido fino que ya usaba el NPC de la interfaz (`MINERALES_BONUS_FUNDACION`
   exportada de `npcGobernanza.ts`). Confirmado: 89/89 posiciones con piedra alcanzable, antes 24/100.
3. **Mano de obra compartida bloqueaba la segunda Granja, y el reclutamiento del NPC sin ningún freno la
   agravaba** — `issues/granjas_no_escalan_con_poblacion.md`. RESUELTO en dos capas: construcción proactiva
   de Granja (motor) + `asegurarGranjasMinimas` (NPC, construcción manual, exenta de la reserva de madera del
   Tesorero) + reordenamiento del tick (población come antes que tropas) + reserva de trigo real antes de
   reclutar (regla del MOTOR ahora, no heurístico del NPC — pensado ya para escala multijugador, ver
   `Docs/5_Sistema_Militar_y_Combate.md` §5.4).

**Con las tres causas resueltas, sigue sin haber línea base limpia**: en un mundo de 20 Facciones de prueba,
11 de 20 asentamientos colapsan durante la transición a nivel 2. La causa —confirmada, no solo sospechada—
es que el primer combate de cada asentamiento contra bandidos, casi simultáneo entre todas las Facciones desde
el tick 1, inflige de golpe el daño agregado que dispara el colapso; ningún gate reactivo (probado con
nutrición y con salud de escuadrones, en varios umbrales) lo evita, porque no hay nada dañado todavía en ese
primer combate contra lo que reaccionar. Detalle completo:
`Consideraciones/NPC_Gobernanza_Facciones_Controladas.md` §"Abierto".

**Lo que esto significa para el plan de anclas**: `residencial`/`mercado`/`almacenaje` ya tenían línea base
razonable desde antes (no dependen de nivel 2). `industria`/`militar` siguen sin poder medirse limpio en
batch mientras el colapso de la transición no se resuelva — pero probablemente ya no vale la pena seguir
persiguiendo esa limpieza antes de arrancar la Etapa 1: el reanclaje de `distanciaAlBarrio` (Centro
Urbano/Mercado/Carpintería, las tres anclas que ya son edificios reales) se puede medir hoy mismo contra
residencial/mercado/almacenaje, que es exactamente donde vive el bug (b) que motivó todo el rediseño. Industria
y militar se validan más adelante, cuando el colapso de transición se resuelva por su cuenta.

## Etapa 1 (2026-08-19): `distanciaAlBarrio` reanclado — implementada y medida

Cambio quirúrgico en `engine/trazado.ts`, sin ningún tipo de ancla nuevo (siguen siendo cero, ver arriba): la
referencia de acreción de `sitiosParaTipo` deja de ser "el edificio más cercano de la misma categoría"
(`existentesBarrio`, el defecto (b) de la nota de honestidad de §5) y pasa a ser, cuando existe, la instancia
más cercana del **ancla real** de esa categoría —`ANCLA_POR_CATEGORIA`: residencial → Centro Urbano, mercado →
Mercado, militar → Carpintería—. Solo esas tres entran, porque son los tres únicos tipos que hoy son edificios
reales y únicos (`EDIFICIOS_UNICOS`); industria y almacenaje no tienen todavía un ancla real que ofrecer (Patio
de Gremios no existe, almacenaje no tiene ancla ni en el catálogo de §5.1) y se quedan exactamente con el
comportamiento de siempre. Si la categoría tiene ancla pero esa instancia concreta todavía no se ha construido
(p. ej. militar antes de la Carpintería), también cae al comportamiento de siempre — nunca se bloquea.

**Medido en batch** (`BATCH_SEED=7 BATCH_FACCIONES=20 BATCH_TICKS=1500`, foto en tick 1500, comparando el mismo
seed con y sin el cambio en `trazado.ts` vía `git stash` — todo lo demás bit a bit idéntico: 9 vivos, 11
colapsados, 72 reclutamientos, mismo `nivelesFaccion`, etc., que es justo lo que confirma que el cambio no toca
nada fuera del trazado):

| Métrica | Antes (barrios) | Después (ancla real) |
|---|---|---|
| `dispersionViviendaCentro` | 3.92 | **3.11** (-20.7%) |
| `manzanasCerradasMedia` | 1.22 | **1.56** (+27.9%, sin regresión) |
| `dispersionPuestoMercado` | 2.66 | 2.62 (± ruido) |

`dispersionViviendaCentro` es exactamente la métrica que se instrumentó para detectar el defecto (b) —la
Vivienda persiguiendo a la última Vivienda en vez de medir contra el Centro Urbano fijo—, y baja un 20%. La
constancia en `dispersionPuestoMercado` es esperable y no un problema: el Mercado ya era, de hecho, la única
instancia de su categoría que participa en este camino (los Puestos nacen gratis por otro mecanismo, §8, nunca
pasan por `sitiosParaTipo`), así que para esa categoría el "antes" ya apuntaba al ancla sin saberlo.
`manzanasCerradasMedia` sube en vez de bajar, así que tampoco hay indicio de la regresión de alineación de
fachadas que preocupaba en §"Abierto".

0 excepciones, 208/208 tests, `tsc --noEmit` limpio. Pendiente: Etapa 2 (anillo generalizado + atracción dura,
§5.2-5.3) y Etapa 3 (marcadores gratis: Plaza, Plaza de Armas, Patio de Gremios) siguen sin ningún código
escrito — esas sí introducen anclas nuevas.

## Etapa 2 (2026-08-19): anillo generalizado + atracción dura — implementada y medida

Sigue sin ningún tipo de ancla nuevo: mismas tres anclas reales de la Etapa 1 (Centro Urbano, Mercado,
Carpintería — `ANCLA_POR_CATEGORIA` en `engine/trazado.ts`). Dos cambios, los dos en `engine/trazado.ts`:

- **§5.2, anillo generalizado.** `redDeCalles` ya le daba a Centro Urbano el trato de "nace con su perímetro
  entero convertido en calle"; ahora `ANCLAS_REALES` (los mismos tres tipos de `ANCLA_POR_CATEGORIA`) extiende
  ese trato a Mercado y Carpintería cuando se construyen: primero se conectan a la red existente igual que
  cualquier edificio (`conectarEdificio`, por si no nacen ya pegados a una calle), y además se añade TODO su
  perímetro, no solo la fachada de entrada.
- **§5.3, atracción dura.** Nueva función `sitiosPorAtraccionDura`: cuando la categoría de un tipo tiene un
  ancla real ya construida, `sitiosParaTipo` deja de usar la cuña de 45° del barrio (§5.8: la cuña ya no
  filtra colocación, solo desempata dónde nace un ancla, que sigue sin implementarse) y busca en anillos
  concéntricos desde el CENTRO GEOMÉTRICO del ancla —no su `posicion` a secas: se añadió `centroDeEdificio`
  porque para Centro Urbano `posicion` es un vértice, no el centro (ver la excepción documentada arriba)—,
  arrancando en `FONDO_MANZANA` y expandiendo de `FONDO_MANZANA` en `FONDO_MANZANA`, capado en
  `radioMaximoNucleo = separacionMinimaAnclas / 2` (nueva constante, `TRAZADO.separacionMinimaAnclas = 6`
  celdas, así que el tope es apenas 3). En cuanto un anillo ofrece un hueco de nivel 0 o 1 (conectado — nivel
  2/3 no cuenta como hueco válido, cuenta como saturación, §5.6), se detiene ahí. Si ningún anillo hasta el
  tope tiene nivel 0/1, el núcleo está saturado y `sitiosParaTipo` cae a la regla de barrio de siempre (la de
  la Etapa 1: ordenada por distancia al ancla) — todavía no existe ancla de saturación (Plaza/Plaza de Armas,
  Etapa 3), así que esto es exactamente el "nunca bloquea" del §5.3 aplicado sin esa pieza.
  `candidatosLibres` pasó a aceptar un `centro: Point` arbitrario (antes siempre implícitamente el origen); los
  cuatro caminos de siempre (afueras, palacio, barrio, `reubicarPorTamano`) pasan `ORIGEN` explícito y quedan
  bit a bit igual que antes — el único código nuevo es el que pasa el centro del ancla.

**Medido en batch** (mismo método que Etapa 1: `BATCH_SEED=7 BATCH_FACCIONES=20 BATCH_TICKS=1500`, comparado
contra el estado post-Etapa-1):

| Métrica (tick 1500) | Etapa 1 | Etapa 2 |
|---|---|---|
| `manzanasCerradasMedia` | 1.56 | **2.78** (+78%) |
| `dispersionViviendaCentro` | 3.11 | 3.13 (± ruido) |
| `dispersionPuestoMercado` | 2.62 | 2.67 (± ruido) |

Población/facciones/colapsos bit a bit idénticos a Etapa 1 (9 vivos, 11 colapsados, 72 reclutamientos, mismo
`nivelesFaccion`...), como es de esperar de un cambio que solo toca trazado. 0 excepciones, 208/208 tests
(incluye `engine/__tests__/trazado.test.ts`, que verifica invariantes genéricos —sin celdas compartidas, todo
edificio toca calle o camino, sin diagonales, red determinista— sobre ciudades producidas por simulación real;
los cinco siguen cumpliéndose bajo el nuevo mecanismo), `tsc --noEmit` limpio.

`manzanasCerradasMedia` sube fuerte y `dispersionViviendaCentro` casi no se mueve respecto a Etapa 1. Lectura
honesta: con el tope en solo 3 celdas, la atracción dura probablemente solo alcanza a colocar uno o dos
satélites muy pegados al ancla antes de saturarse y caer al fallback de barrio (que ya venía de Etapa 1 y por
eso `dispersionViviendaCentro` no cambia mucho más) — el efecto medible parece venir sobre todo del anillo
generalizado (Mercado/Carpintería ahora entregan frente de calle en las cuatro direcciones desde que se
construyen, en vez de una sola fachada), que es lo que más facilita que las filas vecinas cierren manzana. No
se midió por separado cuánto aporta cada uno de los dos cambios — si hiciera falta aislarlo, el mismo método de
`git stash` de Etapa 1 sirve para cualquiera de los dos por separado.

Esto es justo el terreno que `separacionMinimaAnclas` (6 celdas) deja "sin calibrar por simulación" en
§"Abierto": con un tope tan chico, la atracción dura casi no tiene margen para actuar antes de rendirse al
barrio. Subir el número le daría más recorrido al mecanismo nuevo — pero es una decisión de calibración, no
una corrección de bug, y se deja abierta a propósito en vez de tocarla sin que el usuario lo pida.

**Corrección (2026-08-19), mismo día: el párrafo anterior describía un SÍNTOMA, no solo un tope chico.** El
usuario probó en la interfaz y encontró que `sitiosPorAtraccionDura` casi nunca encontraba hueco porque medía
el radio desde el CENTRO del ancla, así que el propio tamaño del ancla se comía el presupuesto — no era (solo)
que `radioMaximoNucleo` fuera chico, es que la métrica de distancia no descontaba el tamaño de ningún edificio.
Arreglado con `gapCeldas` (hueco borde a borde). Detalle completo, con el caso exacto que reportó el usuario y
los números de la reverificación en batch: §5.7 más abajo, "BUG (2026-08-19)".

Sigue sin ningún ancla nueva. Etapa 3 (Plaza, Plaza de Armas, Patio de Gremios como marcadores gratis, más la
semilla de grupo §5.4/5.5 que decide dónde nace cada una) es lo único que falta del plan de 4 etapas.

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

- La red es un **conjunto de aristas unitarias** de la rejilla, separado en `calles` y `caminos` (§11).
- Es **derivada, no persistida**: se reconstruye recorriendo los edificios en su orden de construcción y
  aplicando la regla de crecimiento paso a paso (`redDeCalles`, `engine/trazado.ts`). Mismo input, mismo
  output: no hace falta guardar nada ni migrar partidas guardadas.
- El motor la necesita para decidir dónde colocar cada edificio nuevo, así que se calcula ahí. `ui/canvas.ts`
  solo dibuja lo que recibe (acoplamiento 0: no importa nada de `engine/*` ni de `domain/types`).

## 3. Invariante de conexión

Todo edificio toca una calle o un camino: al menos una arista de su perímetro pertenece a la red. Vale igual
para un 1x1 que para el 6x6 de Granja nivel 4, sin ninguna regla de "fachada" aparte por tamaño.

## 4. Crecimiento de la red

- **Semilla.** Toda **ancla** (§5) convierte en calle las aristas de su perímetro: nace como un anillo cerrado
  y ofrece frente de calle en las cuatro direcciones desde el primer tick. El Centro Urbano es el primer ancla
  del asentamiento y por eso al fundar la red inicial son sus cuatro lados — pero no es un caso especial, es
  la misma regla aplicada al ancla que existe primero.
- Una calle se crea solo cuando hace falta, al colocar un edificio que la necesita.
- Un edificio que queda lejos de la red se conecta extendiendo la calle más cercana hasta él (nunca trazando
  un camino nuevo desde el Centro Urbano).
- Entre las aristas candidatas a fachada se elige la que **prolonga una calle existente en línea recta**, para
  que las fachadas se alineen con la hilera en vez de leerse como cruces sueltos.

## 5. Anclas y satélites

La ciudad no es un centro rodeado de cuñas: es un conjunto de **núcleos**. Un núcleo es un **ancla** más los
**satélites** que orbitan a su alrededor.

Esto **reemplaza al reparto por barrios**, que asignaba a cada categoría funcional una cuña de 45° en una
dirección cardinal sorteada por asentamiento y filtraba por ella cada colocación. El problema de aquel modelo
es que una categoría producía un único borrón que crecía radialmente sin fin, y que la referencia de acreción
era "el edificio más cercano de la misma categoría": cada pieza nueva mide contra su vecino, no contra la
pieza que debería ordenar el grupo, así que la cadena puede irse alejando paso a paso. Los núcleos producen
distritos legibles, con referencia fija y con espacio entre ellos.

> **Nota de honestidad.** Ese segundo defecto está **deducido leyendo el código**, no medido. Antes de dar el
> rediseño por justificado conviene instrumentar el batch (`scripts/run-batch-sim.ts`) con la dispersión
> puesto↔mercado y correr la línea base con barrios. Puede que baste con cambiar la referencia de acreción.

### 5.1 Catálogo de anclas

| Ancla | Naturaleza | Cuándo nace | Satélites | Al saturarse abre |
|---|---|---|---|---|
| Centro Urbano | edificio real, único | al fundar | Vivienda | una **Plaza** |
| Plaza | marcador gratis, sin cola | al saturarse el núcleo residencial | Vivienda | otra **Plaza** |
| Mercado | edificio real, único | construcción normal | Puesto de mercado | **nada** |
| Plaza de Armas | marcador gratis, sin cola | al construirse el 1.º militar | Barracón, Galería de tiro, Carpintería | otra **Plaza de Armas** |
| Patio de Gremios | marcador gratis, sin cola | al construirse el 1.º de industria | Fundición, Curtiduría, Armería, Gran Fundición, Maravilla | otro **Patio de Gremios** |
| Carpintería | edificio real, único | construcción normal | Taller de carpintería | **nada** |

**Invariante: un ancla se repite si y solo si es un marcador gratis.** Centro Urbano, Mercado y Carpintería
son edificios reales y únicos (`EDIFICIOS_UNICOS`) — no se duplican nunca, ni siquiera para desahogar un
núcleo saturado. Por eso el grupo residencial, cuyo primer ancla es el Centro Urbano, se desahoga abriendo una
**Plaza**: un tipo distinto, repetible, que hace el mismo trabajo de trazado sin tocar la unicidad del centro
de poder. El mercado y la carpintería no tienen ese equivalente y simplemente no abren núcleo nuevo (§5.6).

Sin ancla: **Almacén** y **Leñera** — se colocan por la regla genérica (§5.9) y rellenan el espacio entre
núcleos. Se leen como logística repartida, no como distrito, que es lo que son.

Fuera del sistema: **Granja** y **Corral** van a las afueras (§11); **Palacio** sigue pegado al centro;
**Muralla** no orbita nada — una muralla alrededor de una plaza no significa nada geométricamente, así que
usa la regla genérica hasta que tenga geometría propia.

Las anclas gratis (Plaza, Plaza de Armas, Patio de Gremios) siguen el patrón que ya usa Puesto de mercado
(§8): entrada de catálogo con `costo: {}` y `tiempoConstruccionTicks: 0`, nacen ya `activo`, no pasan por la
cola y no se pueden añadir a mano. No compiten por cupo de obra con edificios que sí tienen función
económica. Y son inertes en economía sin ninguna regla nueva, verificado en dos sitios: el mantenimiento no
cuenta edificios —escala con población y distancia (`calcularCostoMantenimiento`, `engine/mantenimiento.ts`)—
y **tampoco empujan la zona de influencia**, porque `edificiosCompletadosEsteTick` solo incrementa cuando un
edificio `en_construccion` agota sus ticks (`avanzarConstruccion`, `engine/construction.ts`), rama por la que
un marcador nacido `activo` no pasa nunca.

### 5.2 El anillo

Toda ancla siembra su anillo de calles (§4). Ese anillo es el frente que sus satélites llenan primero.

### 5.3 Atracción dura: pegado al ancla, y lo demás desempata

**La prioridad de un satélite es estar lo más pegado posible a su ancla.** Eso manda sobre todo lo demás. El
resto de reglas del trazado —niveles de preferencia, filas, manzanas— deciden **entre huecos que empatan en
cercanía al ancla**, no por encima de ella.

En la práctica: el radio de búsqueda arranca en el anillo —las celdas que dan a las calles del ancla— y se
expande de `FONDO_MANZANA` en `FONDO_MANZANA`. En cuanto una expansión ofrece algún hueco, se elige ahí; solo
si hay varios a la misma distancia entran los niveles de preferencia (continúa fila > frente de calle >
medianera > suelto) a desempatar.

**Tope del núcleo.** El radio no crece sin fin: se detiene en `radioMaximoNucleo`. Y ese número no es un
parámetro suelto — se **deriva** de la separación entre anclas:

```
radioMaximoNucleo = separacionMinimaAnclas / 2
```

Con eso dos núcleos vecinos nunca se invaden, y hay un solo número que calibrar en lugar de dos. Es también
lo que da sentido a la saturación (§5.6): sin tope, siempre acabaría apareciendo un hueco de nivel 1 más lejos
y la saturación no dispararía jamás — el sistema de Plazas no arrancaría nunca.

**Y aun así nunca bloquea.** Si dentro de `radioMaximoNucleo` no hay ningún hueco, el satélite no se queda en
espera: sale del núcleo y cae a la regla genérica (§5.9). Un satélite es función, no superficie — a diferencia
de un puesto de mercado, no puede saltarse en silencio.

### 5.4 Semilla de un grupo

El primer edificio de un grupo se construye antes de que exista su ancla. Se coloca en el **mejor hueco que
esté a `separacionMinimaAnclas` o más de toda ancla existente**, y su ancla nace inmediatamente después,
frente a él (§5.5).

Esta regla es la que sostiene todo lo demás. Sin ella, la colocación genérica ("mejor hueco más cercano al
origen") haría nacer cada núcleo pegado al anterior, y la separación mínima entraría en conflicto directo con
la regla que coloca los edificios. Así la separación se cumple **por construcción**, no por validación — mismo
criterio de diseño que el resto del trazado (§1).

### 5.5 Dónde nace el ancla: "frente a él"

El ancla se coloca **al otro lado de la arista de calle a la que da fachada** ese primer edificio. Comparten
esa arista, así que quedan cara a cara con la calle en medio y en manzanas distintas — sale gratis de la
geometría de aristas de §1, sin ninguna regla nueva de separación.

Desempate cuando el edificio da a varias calles: se elige **la cara que mira en dirección contraria al Centro
Urbano**, para que el núcleo crezca hacia afuera y no se estrangule contra la ciudad ya construida.

### 5.6 Ancla adicional por saturación

Si un satélite no encuentra hueco de nivel 0 ni 1 **habiendo llegado el radio a `radioMaximoNucleo`** (§5.3),
el núcleo está saturado: nace el **ancla de saturación** de ese grupo (por la regla de §5.4) y el satélite la
orbita. El tope es lo que hace que esta condición pueda cumplirse alguna vez.

El ancla de saturación es un tipo declarado por grupo, **no "otra del mismo tipo"** — esa distinción es la que
protege la unicidad de Centro Urbano, Mercado y Carpintería. En residencial es la Plaza; en militar e
industria coincide con el ancla inicial porque ya son marcadores gratis repetibles; en mercado y en
carpintería **no hay ninguna**, y ahí el radio simplemente sigue expandiendo. Degradación limpia: la ausencia
de ancla de saturación no es un caso especial, es el valor por defecto de la declaración.

Se autorregula según la geometría real — no hay ningún número de satélites por ancla que calibrar.

### 5.7 Separación mínima entre anclas

`separacionMinimaAnclas` = 6 celdas entre centros de ancla, y es **relajable**: si ningún hueco la cumple, se
reduce progresivamente hasta que aparezca uno.

**Implementado (2026-08-19), y es un piso aparte: `separacionSeguridadAnclas` = 2 celdas, NUNCA relajable.**
A petición del usuario: "siempre tiene que haber, aunque sea 2 celdas a la redonda de un ancla, donde no puede
haber otra ancla — es como una zona de seguridad." La relajación progresiva de `separacionMinimaAnclas` de
arriba es todavía SOLO especificación (vive en §5.4/5.5, la semilla de grupo, que sigue sin código — ver
"Estado" al principio del doc); lo que sí hay código hoy es más simple y más estricto: `sitiosParaTipo`
(`engine/trazado.ts`) descarta CUALQUIER candidato para un ancla real nueva (Mercado, Carpintería —
`ANCLAS_REALES`) que quede a menos de `TRAZADO.separacionSeguridadAnclas` celdas de OTRA ancla real ya
construida. Si ningún hueco la cumple, no hay sitio válido en ese tick — no se relaja ni se ignora, es un piso
duro. Cuando exista la semilla de grupo de §5.4/5.5, `separacionMinimaAnclas` (6, relajable) decide DÓNDE nace
una ancla nueva; `separacionSeguridadAnclas` (2, fijo) seguirá siendo el piso que ninguna relajación puede
cruzar.

**BUG (2026-08-19) medido por el usuario en la interfaz, y corregido el mismo día — la primera versión medía
centro a centro, no borde a borde.** El usuario fundó un asentamiento, dejó que el NPC construyera, y encontró
el Mercado pegado al Centro Urbano (`Mercado[-1,-1]` tocando `CentroUrbano[0,-1]`, cero celdas de hueco) pese al
piso de 2 celdas ya implementado. Causa: `distanciaEntrePuntos(candidato.punto, centroDeEdificio(ancla))` medía
la distancia entre los CENTROS geométricos de los dos edificios, sin descontar el tamaño de ninguno de los dos
— con Centro Urbano (3x3) y Mercado (3x2), cada uno ya ocupa hasta 1.5 celdas desde su propio centro hasta su
borde, así que dos edificios con los bordes ya tocándose podían seguir midiendo ~3.35 celdas de centro a centro
(muy por encima del piso de 2), y el filtro los dejaba pasar sin darse cuenta.

**El mismo bug de fondo afectaba a la atracción dura (§5.3):** el usuario también encontró que la segunda pieza
de un Mercado (`puestoMercado`) no salía pegada al ancla pese a haber celdas libres justo al lado. Causa
idéntica: `sitiosPorAtraccionDura` medía el radio de cada anillo desde el CENTRO del Mercado, así que el propio
tamaño del Mercado ya se comía casi todo el presupuesto de `radioMaximoNucleo` (3 celdas) antes de llegar a
ningún candidato real — el anillo casi siempre salía vacío y la colocación caía al fallback de barrio, que no
garantiza tocar el ancla (solo "lo más cerca posible dentro de la cuña completa").

**Arreglo, aplicado a los dos casos con la misma pieza nueva:** `gapCeldas` (`engine/trazado.ts`) — hueco real
BORDE A BORDE entre dos rectángulos de celdas, Chebyshev (la diagonal cuenta 1 paso, no √2, mismo criterio que
el resto del archivo, que ya razona en celdas enteras): 0 si se tocan o se solapan, si no, el mayor entre el
hueco de columnas y el de filas. `candidatosLibres` pasó de aceptar un punto centro a un `RectanguloCeldas` de
referencia (tamaño cero en el origen para los caminos de siempre — barrio/afueras/palacio/`reubicarPorTamano`,
sin cambio de comportamiento ahí); la zona de seguridad y `sitiosPorAtraccionDura` usan `gapCeldas` en vez de
distancia centro a centro. De paso, `sitiosPorAtraccionDura` corrigió un off-by-one: el primer anillo probado
ahora es hueco 0 (tocando), antes arrancaba directamente en `FONDO_MANZANA` (2 celdas) y nunca probaba la
posición más pegada posible.

**Verificado en batch** (`BATCH_SEED=7 BATCH_FACCIONES=20 BATCH_TICKS=1500`, gobernanza NPC activa en las 20
Facciones): 19 asentamientos con Mercado, **0 violaciones de la zona de seguridad** (antes del arreglo, el caso
reportado por el usuario era justo ese: gap 0 pasando un piso de 2), y **38/38 piezas de Mercado (`puestoMercado`)
con gap 0 respecto a su ancla** — antes algunas quedaban a 2 filas de distancia. 19/20 asentamientos vivos (sin
cambio frente al estado post-fix de mano de obra), 0 excepciones, 208/208 tests, `tsc --noEmit` limpio. Un test
existente (`nucleoMilitarNpc.test.ts`) necesitó que su fixture reflejara un `radioPotencial` de nivel 2 real
(antes forzaba `nivel: 2` sin haber crecido nunca, así que el disco se quedaba en el mínimo de fundación —
5 celdas, donde el propio Centro Urbano ya no deja hueco de 2 celdas libres para el Mercado en ninguna
dirección — mismo patrón que ya hizo falta ajustar para la reserva de trigo, ver Doc 5 §5.4).

**El espacio urbano al fundar es minúsculo, y hay que hacer la cuenta.** El radio de la trama es 30 al fundar
= **5 celdas**, y sube a 60/90/120/150/180 por nivel (10/15/20/25/30 celdas,
`ZONA_INFLUENCIA.radioMaximoPorNivel`). Peor: los cinco edificios de la fundación nacen ya `activo`, así que
tampoco empujan el radio — el asentamiento se queda clavado en 5 celdas hasta que la auto-construcción
complete unas 6 obras (`crecimientoPorEdificioCompletado: 5`). Con el Centro Urbano ocupando 3x3 en el medio,
**al fundar no cabe ninguna segunda ancla a 6 celdas.**

Lo que salva el sistema es que los grupos que piden ancla están **gateados por nivel**: Fundición, Curtiduría
y Armería exigen nivel 2 (radio 15 celdas) y Carpintería nivel 3 (20 celdas). Cuando el Patio de Gremios o la
Plaza de Armas pueden existir, el disco ya da de sobra para 6 celdas de separación.

### 5.7.1 Regla de gate: ningún ancla puede nacer en un disco que no la admite

El párrafo anterior es cierto para la auto-construcción y **falso para la construcción manual**, que es el
camino por el que se cuela el problema. Barracón (30 de madera) y Galería de tiro (50) **no tenían gate de
nivel**, y la fundación entrega 50 de madera: se podía plantar un Barracón en el tick 1, a nivel 1, en un
disco de 5 celdas. Entonces §5.4 no encuentra ningún hueco a 6 celdas, la separación se relaja hasta cero, la
Plaza de Armas nace pegada al Centro Urbano y —como **nada mueve jamás un ancla**— el cuartel se queda en la
plaza mayor el resto de la partida, con todo lo militar orbitando el centro. Una decisión del jugador en el
primer tick anulaba el sistema de distritos entero.

De ahí sale la regla general, que es lo que hay que respetar al añadir cualquier tipo futuro:

> **Todo tipo de edificio capaz de abrir un grupo nuevo —es decir, de arrastrar un ancla tras de sí (§5.4)—
> debe estar gateado al menos a nivel de asentamiento 2**, por los dos caminos: auto-construcción y
> `anadirEdificioManualmente`. Si un tipo puede construirse en un disco donde la separación mínima no cabe,
> ese tipo puede fijar un núcleo mal colocado de forma permanente.

Aplicación concreta: **Barracón y Galería de tiro pasan a `requisitoNivelAsentamientoConstruccion: 2`**, igual
que Fundición/Curtiduría/Armería. A nivel 2 el disco ya mide 15 celdas y la separación cabe de sobra.

**Por qué el gate no le cuesta nada al jugador.** El reclutamiento sí está implementado (`reclutarTropa`,
`engine/tropas.ts`, enchufado a la UI y a la gobernanza NPC) y hay tres tropas de `nivelRequerido: 1`. Pero
`reclutarTropa` **no consulta el nivel del asentamiento**: lo que impide reclutar antes de nivel 2 es
económico, no una regla. Las tres tropas piden `armaMadera`, `armaCobre` o `armaduraBasica`, y los tres los
fabrica **solo la Armería**, que ya exigía nivel 2. Sin proveedor no hay equipo, y salta *"No hay equipo
suficiente para reclutar esta tropa"*. Es decir: **nivel 2 ya era el suelo real**, y un Barracón construido
antes es un edificio decorativo. El gate solo hace explícito lo que la economía ya imponía.

> **Decisión deliberada: el gate va en la construcción, nunca en el reclutamiento.** Un asentamiento que sube
> a nivel 2, construye Armería y Barracón y después se **degrada** a nivel 1 sigue pudiendo reclutar mientras
> tenga materiales. Los edificios y el equipo ya están físicamente ahí; perder nivel no borra lo que ya
> levantaste. Por eso **no** hay —ni debe añadirse— una comprobación de `nivelActualDe` dentro de
> `reclutarTropa`: su ausencia no es un olvido.

La misma regla resuelve la **Plaza residencial**: es el otro ancla que querría nacer temprano, y también
necesita su gate en lugar de nacer relajando siempre la separación.

### 5.8 Identidad de cada ciudad

El sorteo de direcciones cardinales por asentamiento (`direccionesDelAsentamiento`) **sobrevive**, pero cambia
de trabajo: ya no filtra cada colocación, solo **desempata dónde nace un ancla** entre los huecos que cumplen
la separación mínima.

Era lo que hacía que dos ciudades no se parecieran. Sin ello, con las anclas colocadas solo por "mejor hueco
que respete la separación", todas acabarían con la misma silueta y solo variaría el orden de construcción.

### 5.9 Regla genérica (tipos sin ancla)

Mejor hueco por nivel de preferencia, 360°, el más cercano al origen. Es lo que ya usaban los tipos sin
categoría antes de este sistema, sin cambios.

### 5.10 Las anclas encadenan

Un edificio puede ser satélite de un ancla y ancla de sus propias piezas a la vez. La Carpintería es el caso:
orbita la Plaza de Armas y a su vez ancla sus dos talleres (§9). El mapa satélite → ancla es una cadena, no
una partición en dos bandos.

## 6. Tamaños de edificio

Cada tipo ocupa un rectángulo ancho×alto de celdas. El tamaño se deriva de `tipo` (+ `nivelInterno` en Granja
y en las piezas satélite), nunca se persiste en el `Edificio` (`EDIFICIO_TAMANO`, `constants.ts`;
`tamanoEdificio`, `engine/trazado.ts`).

| Tipo | Tamaño | Ubicación |
|---|---|---|
| Centro Urbano | 3x3 | origen — primer ancla |
| Plaza | 2x2 | ancla residencial adicional |
| Plaza de Armas | 2x2 | ancla militar |
| Patio de Gremios | 2x2 | ancla de industria |
| Carpintería (pieza principal) | 4x2 | satélite de Plaza de Armas |
| Taller de carpintería | 2x2 | satélite de Carpintería |
| Fundición | 2x2 | satélite de Patio de Gremios |
| Curtiduría | 2x2 | satélite de Patio de Gremios |
| Armería | 2x3 | satélite de Patio de Gremios |
| Barracón | 2x2 | satélite de Plaza de Armas |
| Galería de tiro | 2x4 | satélite de Plaza de Armas |
| Vivienda | 1x1 | satélite de Centro Urbano / Plaza |
| Leñera | 1x1 | regla genérica |
| Almacén | 2x1 | regla genérica |
| Mercado (pieza principal) | 3x2 | ancla de mercado |
| Puesto de mercado | 2x2, 3x2 o 1x1 según la pieza | satélite de Mercado |
| Palacio | 4x4 | sin ancla, lo más cerca posible del centro |
| Corral | 4x3 | afueras |
| Granja | 2x2 → 2x3 → 4x3 → 6x6 (niveles 1-4) | afueras |

**Tope de Almacenes.** La capacidad de almacenamiento no puede crecer sin límite: cada nivel de asentamiento
admite un número fijo de Almacenes — **4 / 8 / 16 / 24 / 32** (`NECESIDADES.maximoAlmacenesPorNivel`). Cuenta
los de cualquier estado (activo, en obra y en cola), y se aplica igual a la auto-construcción y a la adición
manual: es una regla del juego, no un heurístico interno.

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
- Sigue las mismas reglas de afueras que cualquier Granja/Corral nuevo (§11).

## 8. Mercado: una zona, no un edificio

El Mercado es una **zona**: una pieza principal —el `mercado` de siempre, 3x2— que hace de ancla, más piezas
satélite de tipo `puestoMercado` que aparecen al alcanzar cada nivel interno.

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
- Orbitan el Mercado por la atracción de §5.3, con la pieza principal como referencia fija.
- **Si no hay hueco, el puesto se salta en silencio** y no bloquea la subida de nivel: la zona es superficie,
  no función.
- Las partidas guardadas se rellenan al cargar (`migrarEdificiosAEspacioLocal`), de forma idempotente.

## 9. Carpintería: zona de tres piezas

Misma idea que el Mercado, aplicada al núcleo militar. La Carpintería deja de ser un bloque único de 5x4 y
pasa a ser una zona de **tres piezas**:

| Pieza | Tamaño | Cómo aparece |
|---|---|---|
| Carpintería (principal) | 4x2 | construcción normal: cuesta materiales y pasa por la cola |
| Taller de carpintería ×2 | 2x2 cada uno | gratis y ya activos al completarse la principal, sin pasar por la cola |

- Las tres piezas son **militares**: la principal orbita la Plaza de Armas, y los dos talleres orbitan a la
  principal (§5.10).
- Repartir 5x4 en 4x2 + 2x2 + 2x2 es lo que hace que el núcleo militar tenga contenido: un bloque de 20 celdas
  se comía el distrito entero frente al 2x2 del Barracón y el 2x4 de la Galería de tiro.
- Los talleres son **un tipo distinto** de `carpinteria`, igual que Puesto de mercado lo es de Mercado: así el
  gate de nivel y `EDIFICIOS_UNICOS` siguen viendo exactamente una Carpintería.
- **Ambos talleres nacen al completarse la principal**, no uno por nivel interno. La Carpintería es un
  edificio-gate sin recetas en ninguno de sus dos niveles, así que no hay ninguna función que escalonar — al
  contrario que el Mercado, cuya zona crece porque su función crece.
- Si un taller no encuentra hueco se salta en silencio, mismo criterio que un puesto de mercado.

**Quién abre de verdad el núcleo militar, por cada camino.** Conviene tenerlo presente antes de calibrar nada,
porque los dos caminos dan resultados distintos:

- **Auto-construcción / batch.** Barracón, Galería de tiro, Palacio y Muralla **no se auto-construyen** (la
  política de desbloqueo se retiró, ver `avanzarConstruccion` en `engine/construction.ts`) y la gobernanza NPC
  solo añade `mercado` (`app/npcGobernanza.ts`). El único militar que llega a existir es la **Carpintería**, a
  nivel 3. El núcleo militar simulado será entonces Plaza de Armas + Carpintería + 2 talleres, y **la Plaza de
  Armas nacerá frente a la Carpintería**, no frente a un Barracón.
- **Construcción manual.** El jugador sí puede añadir Barracón o Galería de tiro, y entonces cualquiera de los
  dos puede ser quien abra el grupo y decida dónde cae la Plaza de Armas. Con el gate de §5.7.1 lo más pronto
  que puede pasar es a nivel 2, con el disco ya en 15 celdas — que es exactamente para lo que existe ese gate.

## 10. Manzanas emergentes

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

## 11. Afueras

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

## 12. Dibujo

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
- **Migración de partidas guardadas al modelo de anclas.** Los asentamientos existentes no tienen Plaza de
  Armas, Patio de Gremios ni talleres de Carpintería. El relleno idempotente al cargar ya tiene precedente
  (`completarPuestosDeMercado`) y la regla de §5.5 es geometría pura, así que aplica igual sobre una ciudad ya
  construida por el modelo de barrios — falta confirmarlo contra una partida real. Caso aparte: la Carpintería
  guardada mide 5x4 y pasa a medir 4x2; al encoger nunca colisiona, pero deja un hueco irregular.
- **`separacionMinimaAnclas` = 6 celdas** es un valor propuesto, sin calibrar por simulación. De él se deriva
  `radioMaximoNucleo` (§5.3), así que es el único número del sistema — calibrarlo los mueve a los dos a la vez.
- **A qué nivel se gatea la Plaza residencial.** La regla de §5.7.1 ya dice que necesita gate; falta fijar el
  número. Nivel 2 la alinea con Barracón/Galería, pero un núcleo residencial en un disco de 5-10 celdas puede
  saturarse antes de eso — hay que mirar en el batch a qué altura satura de verdad.
- **Riesgo de la prioridad "pegado al ancla" (§5.3) sobre la alineación de fachadas.** El código actual
  documenta una regresión medida: sin preferir la continuación de fila, la ciudad crece como un borrón
  compacto y solo 3 de 50 edificios llegaban a cerrar manzana. Al pasar la cercanía al ancla por delante de
  los niveles de preferencia se reintroduce esa presión. Se mitiga porque la preferencia sigue desempatando
  dentro de cada expansión, pero es lo primero que hay que mirar en el batch: **número de manzanas cerradas
  por ciudad**, comparado contra la línea base con barrios.
- **Reflow al subir de nivel.** El radio urbano salta 5 → 10 → 15 → 20 celdas. No está definido si los núcleos
  ya saturados se reevalúan al crecer el disco. Sin reflow, la ciudad congela para siempre la forma del early
  game; con reflow, el replay determinista del pilar 2 (§2) deja de ser estable, porque el trazado dejaría de
  depender solo del orden de construcción. Hay que elegir, y la elección afecta a un pilar.
- **Ciclo de vida del ancla.** Qué pasa con los satélites si su ancla desaparece, y cómo se propaga en la
  cadena de §5.10. Hoy es discutible pero no urgente: verificado que **nada destruye edificios activos** en
  Fase 0 — el único sitio que quita un edificio del array es `quitarDeCola`, y solo actúa sobre `en_cola`. Es
  deuda para cuando exista destrucción (incendio, asedio, abandono), no un agujero actual.
- **Política "Líneas de Producción"** (`sitioEnBarrioLineaProduccion`) optimizaba entre todos los huecos de la
  cuña del barrio. Bajo anclas pasa a optimizar entre todos los huecos dentro del radio del núcleo, lo que
  conserva la intención (la logística puede ganarle a la compacidad dentro del distrito) sin dejarla dispersar
  la ciudad — pendiente de implementar.
- **Nombres provisionales.** `Plaza`, `Plaza de Armas`, `Patio de Gremios` y `Taller de carpintería` son los
  nombres de trabajo de los cuatro tipos nuevos; ninguno está fijado en el catálogo todavía.
