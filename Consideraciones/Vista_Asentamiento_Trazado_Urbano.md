# Vista de Asentamiento — Trazado urbano dinámico

Documento vivo. Recoge el estado actual del diseño; lo que sigue sin implementar va al final, marcado como
abierto.

> **Estado real (Etapa 5, última entrada del log de abajo): el sistema de anclas está implementado por
> completo** — árbol único de anclas, orientación intercambiable, variedad residencial, compás + eje rotado.
> El log de "Estado"/"Etapa N" que sigue es cronológico e histórico: la primera entrada ("CERO código de
> anclas") describe el arranque del diseño, no el estado actual. Las secciones numeradas (§1 en adelante) ya
> reflejan la implementación de la Etapa 5, no las etapas anteriores.

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

## Etapa 3: marcadores gratis + semilla de grupo — implementada, sin batch (no registrado en su momento)

Cierra el plan de 4 etapas: `plaza`, `plazaDeArmas`, `patioDeGremios` y `tallerCarpinteria` nacen como los
"marcadores gratis" que describe §5.1 (costo `{}`, `tiempoConstruccionTicks: 0`, nacen ya `activo`, no pasan
por cola, no se pueden añadir a mano). Carpintería deja de ser un único bloque de 5x4 y pasa a ser la zona de
tres piezas de §9 (pieza principal 4x2 + 2 talleres). Se implementó la semilla de grupo (§5.4/5.5) tal como
estaba especificada: el primer edificio de un grupo sin ancla se coloca por el reparto de barrio, filtrado a
`separacionMinimaAnclas` de toda ancla existente, y **inmediatamente después** nace su ancla "frente a él" —al
otro lado de la arista de calle a la que da fachada, desempatando por la cara que mira lejos de Centro Urbano.
El gate de nivel de §5.7.1 (`NIVEL_GATE_ANCLAS = 2`) se aplicó al MECANISMO de nacimiento, no a tipos de
edificio concretos, para poder proteger también al núcleo residencial (Vivienda) sin gatear su construcción.
Decisión cerrada con el usuario: **sin reflow al subir de nivel** — un núcleo saturado con el disco pequeño
conserva su ancla de saturación para siempre, aunque el disco crezca después (resuelve el punto "Reflow al
subir de nivel" de §"Abierto").

Este entrada se escribe en retrospectiva (durante la Etapa 5, más abajo): la Etapa 3 se implementó y se probó
(`anclasSatelites.test.ts`, verificación manual en la interfaz) pero no se registró aquí en su momento — de ahí
que la Etapa 5 tuviera que redescubrir por análisis, no por este documento, que §5.4/5.5 ya tenían código.
**Todo lo que describen §5.4 y §5.5 más abajo quedó reemplazado por la Etapa 5** — se dejan íntegros más abajo
por valor histórico, pero el mecanismo real hoy es el árbol único de anclas.

## Etapa 4: variedad, orientación, borde compartido y compás — implementada, sin batch

Cuatro cambios independientes, a petición del usuario tras jugar la Etapa 3 en la interfaz:

1. **Orientación intercambiable ancho↔alto.** Nuevo campo `Edificio.rotado`. Al buscar hueco, se ofrece también
   la huella girada (`ancho`↔`alto`) como candidato adicional — mismo criterio de nivel/hueco que la
   orientación normal, con un desempate final sembrado (`semillaCandidato`) para que la elección varíe entre
   ciudades. Excluye `granja` (progresión real de tamaño) y `puestoMercado` (`nivelInterno` ya identifica una
   forma concreta de zona).
2. **Desempate por máximo borde compartido.** `sitiosPorAtraccionDura` desempataba solo por `hueco` mínimo, que
   no distingue tocar por una esquina de compartir un lado entero. Nueva función `bordeCompartido`, aplicada
   como criterio secundario tras el hueco.
3. **Compás + eje rotado por asentamiento.** El sorteo de direcciones cardinales (§5.8) pasó a decidir también
   dónde nace el ancla (antes solo desempataba la cuña de barrio): "frente a él" (§5.5) eligió el lado
   alineado con la dirección de compás de la categoría, no solo el más lejano al origen. Además, nueva
   `anguloRotacionEje`: el eje de las 8 direcciones se rota un ángulo aleatorio (determinista, `[0°, 45°)`) por
   asentamiento, para que dos ciudades no compartan geometría aunque compartan el mismo reparto de categorías.
4. **Variedad de anclas residenciales.** `ANCLA_SATURACION_POR_CATEGORIA` pasa de un tipo único por categoría a
   una lista; residencial sortea (determinista) entre `plaza`, `pozo` (nuevo, 1x1) y `parque` (nuevo, 3x2).
   Militar e industria siguen con un solo tipo — mismo mecanismo, listas de un elemento.

Verificado con 234/234 tests (4 nuevos, uno por punto) y `tsc --noEmit` limpio — sin batch, la calibración fina
de estos cuatro puntos queda para cuando haga falta. **Puntos 1 y 2 sobreviven intactos a la Etapa 5** (viven en
`sitiosPorAtraccionDura`, que no se tocó). **Puntos 3 y 4 quedaron absorbidos/reemplazados por la Etapa 5**: el
compás + eje rotado ahora se aplica a las 8 ranuras del árbol único, no a "frente a él"; la variedad
residencial sobrevive igual (`tipoAnclaParaCategoria`), pero ya no depende de qué edificio disparó la semilla.

## Etapa 5: árbol único de anclas — reemplaza la semilla de grupo (§5.4/5.5)

**El motivo.** Playtesting tras la Etapa 4 encontró edificios sin ningún ancla real cerca —viviendas sueltas,
una Curtiduría sin Patio de Gremios— y, en un caso, un ancla naciendo lejísimos de quien la necesitaba.
Diagnóstico con el usuario, dos causas raíz **preexistentes desde la Etapa 3**, que la Etapa 4 solo hizo más
visibles (más anclas repetibles, playtesting más largo):

1. `sitiosParaTipo` decidía qué ancla usar con `anclaMasCercana(tipos, ORIGEN, ...)` — la referencia era
   SIEMPRE el origen del asentamiento, nunca la posición real de lo que se estaba colocando. En cuanto existía
   más de una instancia del mismo tipo, la más cercana al origen ganaba siempre, aunque estuviera saturada y
   una instancia más nueva tuviera sitio de sobra.
2. La semilla de grupo (§5.4/5.5): el edificio se colocaba primero por barrio, y RECIÉN DESPUÉS se comprobaba
   si necesitaba un ancla que no tenía — momento en el que "frente a él" intentaba plantarla pegada a donde el
   edificio cayó. Si ese lugar estaba apretado (cerca de otra ancla, dentro de `separacionSeguridadAnclas`), el
   intento fallaba EN SILENCIO y nunca se reintentaba: el edificio quedaba sin su ancla para siempre.

**El rediseño, cerrado con el usuario en varias rondas** (incluida una simulación abstracta del árbol de
crecimiento, fuera del motor del juego, que confirmó el diseño antes de escribir una línea de código real —
ver más abajo): **el reparto de barrio y la semilla de grupo desaparecen por completo** para las categorías con
ancla. En su lugar, un **árbol único** que comparten TODAS las anclas del asentamiento, sin distinguir tipo:
una Plaza de Armas puede nacer como hija de un Pozo.

- **Semilla activa**: el nodo del árbol que se usa AHORA MISMO como referencia para decidir dónde nace la
  próxima ancla — no es un edificio, es un puntero que se mueve por el árbol.
- **8 ranuras** por ancla (las 8 direcciones cardinales, eje rotado por asentamiento — mismo mecanismo de la
  Etapa 4, punto 3, reutilizado aquí).
- **Ciclo al proponer una ancla**: ¿la semilla activa tiene alguna ranura con hueco real? Sí → se construye
  ahí. No (las 8 fallaron) → la semilla se marca **saturada para siempre** (`Edificio.semillaSaturada`, estado
  persistido, nunca se revisa dos veces) y se busca, entre TODAS las anclas no descartadas, la más cercana a
  la raíz — sin volver a intentar la que se acaba de descartar.
- **El ancla nace SIEMPRE antes que el edificio que la necesita** (`asegurarAnclaPara`, `engine/construction.ts`,
  llamada antes de calcular sitio) — nunca al revés. Esto es lo que cierra el bug 2 de arriba: ya no hay
  "edificio colocado, ancla intentada después y fallando en silencio".

**Verificado por simulación abstracta** (árbol/grid de 8 direcciones sin geometría del juego, antes de tocar
código real): sin límite de radio, el árbol nunca se traba, aunque se probó hasta 50.000 anclas creadas.
Con radio limitado (simulando `radioPotencial`), se traba EXACTAMENTE cuando el 100% de las celdas del radio
están ocupadas — en los 4 radios probados, `anclas creadas == celdas totales del cuadrado`, sin excepción. El
diseño no deja huecos atrapados ni se rinde antes de tiempo.

**Lógica 2 (satélites de un ancla, `sitiosPorAtraccionDura`) no se tocó** — sigue siendo la Etapa 4 completa
(orientación, borde compartido) sin cambios. La separación entre las dos lógicas fue explícita en el diseño:
Lógica 1 responde "¿dónde nace la PRÓXIMA ANCLA?" (ancla→ancla, sin distinguir tipo); Lógica 2 responde "¿dónde
coloco este EDIFICIO NORMAL?" (ancla→satélite, por tipo/categoría). Comparten la palabra "saturada" para dos
cosas distintas: una ancla puede estar `semillaSaturada` (no produce más anclas) y seguir teniendo hueco de
sobra para sus propios satélites.

**Cambio adicional a petición del usuario**: Almacén y Leñera dejan la cuña de "almacenaje" (que desaparece
junto con el resto del reparto de barrio) y pasan al mismo camino que Palacio — 360°, sin ancla, el hueco más
cercano al origen. Llenan huecos libres desde Centro Urbano hacia afuera en cualquier dirección.

Verificado con 237/237 tests (3 nuevos que prueban el árbol directamente: bootstrap, árbol mixto sin filtrar
tipo, saturación-y-descarte) y `tsc --noEmit` limpio. Sin batch — pendiente si hace falta calibrar
`RADIO_INICIAL_RANURA`/`RADIO_MAXIMO_RANURA` (nuevas constantes, derivadas de `separacionMinimaAnclas`) contra
partidas reales.

**§5.4, §5.5, §5.7 (parte), §5.8 y §5.9 más abajo se reescribieron para reflejar el árbol único** — no se dejó
el texto viejo como referencia histórica esta vez, a diferencia de la nota de la Etapa 3, porque describir dos
mecanismos incompatibles en la misma sección confunde más de lo que documenta.

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

| Ancla | Naturaleza | Cuándo nace | Satélites | Al saturarse, la categoría abre |
|---|---|---|---|---|
| Centro Urbano | edificio real, único | al fundar | Vivienda | **Plaza**, **Pozo** o **Parque** (sorteo) |
| Plaza / Pozo / Parque | marcador gratis, sin cola | por el árbol único de anclas (§5.4) | Vivienda | **Plaza**, **Pozo** o **Parque** (sorteo) |
| Mercado | edificio real, único | construcción normal | Puesto de mercado | **nada** |
| Plaza de Armas | marcador gratis, sin cola | por el árbol único de anclas (§5.4) | Barracón, Galería de tiro, Carpintería | otra **Plaza de Armas** |
| Patio de Gremios | marcador gratis, sin cola | por el árbol único de anclas (§5.4) | Fundición, Curtiduría, Armería, Gran Fundición, Maravilla | otro **Patio de Gremios** |
| Carpintería | edificio real, único | construcción normal | Taller de carpintería | **nada** |

**Invariante: un ancla se repite si y solo si es un marcador gratis.** Centro Urbano, Mercado y Carpintería
son edificios reales y únicos (`EDIFICIOS_UNICOS`) — no se duplican nunca, ni siquiera para desahogar un
núcleo saturado. Por eso el grupo residencial, cuyo primer ancla es el Centro Urbano, se desahoga abriendo una
Plaza, un Pozo o un Parque (sorteo determinista, `tipoAnclaParaCategoria`): tipos distintos, repetibles, que
hacen el mismo trabajo de trazado sin tocar la unicidad del centro de poder. El mercado y la carpintería no
tienen ese equivalente y simplemente no abren núcleo nuevo (§5.6).

**Dónde nace exactamente la ancla nueva ya no depende de qué instancia se saturó** (Etapa 5, §5.4): todas las
anclas del asentamiento —sin importar tipo ni categoría— comparten un único árbol de crecimiento, así que una
Plaza de Armas puede terminar naciendo como hija de un Pozo. El tipo que se sortea sigue siendo por categoría
(`ANCLA_SATURACION_POR_CATEGORIA`); lo que cambió es solo la geometría de dónde se planta.

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

### 5.3.1 Qué instancia de ancla recibe el próximo satélite — instancia activa por categoría (post-Etapa 5)

Cuando una categoría (residencial/militar/industria) ya tiene más de una instancia de ancla —por ejemplo dos
Plazas, una saturada de satélites y otra recién nacida—, hace falta decidir a cuál se atrae el PRÓXIMO
satélite. Hasta corregirse esto, `sitiosParaTipo` usaba `anclaMasCercana(tipos, ORIGEN, ...)`: la instancia más
cercana al ORIGEN del asentamiento, sin memoria de si tenía hueco. Con Centro Urbano en `posicion = (0,0)`
exacto, eso significaba que, en cuanto el núcleo residencial inicial se llenaba de verdad, **ninguna otra
instancia volvía a usarse jamás** — cada ronda de demanda residencial disparaba directamente
`asegurarAnclaPara` (§5.4) y fabricaba un ancla nueva en vez de reutilizar una Plaza/Pozo/Parque con hueco de
sobra. Bug real detectado con una herramienta de inspección visual aparte del juego (no un test): una corrida
de prueba llegó a 8 anclas residenciales, ninguna marcada como saturada de verdad, dos huérfanas.

**La corrección** (mismo patrón que la saturación de semilla del árbol, §5.6, pero para esta pregunta
distinta): nuevo campo persistido `Edificio.anclaLlena` — una instancia queda `anclaLlena` la primera vez que
`sitiosPorAtraccionDura` no le encuentra hueco para el tipo que se está pidiendo; se marca una sola vez y
nunca se revisa (nada libera celdas). `anclaActivaParaCategoria` (`engine/trazado.ts`) recorre las instancias
de la categoría que NO estén `anclaLlena`, de más cerca a más lejos del origen, probando cada una con la misma
`sitiosPorAtraccionDura` real hasta encontrar la primera con hueco — mismo criterio de "búsqueda pareja" que
`semillaActiva` usa para el árbol, pero aplicado a instancias de una categoría, no a todo el árbol.
`asegurarAnclaPara` (`engine/construction.ts`) llama primero a esto: si encuentra instancia con hueco, la usa
y termina (persiste las marcas `anclaLlena` descubiertas en el camino); solo si NINGUNA instancia sirve pasa a
crear un ancla nueva vía el árbol (§5.4).

**Importante: `anclaLlena` (Lógica 2) y `semillaSaturada` (Lógica 1, §5.6) son criterios INDEPENDIENTES,
ninguno dispara al otro** — un error real de una primera versión de esta corrección los mezclaba (que
`anclaLlena` disparara el avance de `semillaActiva`), corregido tras confirmarlo con el usuario: una instancia
puede estar `anclaLlena` y seguir siendo la semilla activa del árbol mientras sus 8 ranuras tengan sitio, o
estar `semillaSaturada` y seguir teniendo hueco de sobra para sus propios satélites.

### 5.4 El árbol único de anclas (Etapa 5)

**El ancla nace SIEMPRE antes que el edificio que la necesita, nunca al revés.** Reemplaza por completo el
mecanismo anterior de "semilla de grupo" (edificio primero, ancla reaccionando después "frente a él") — ver
la nota de la Etapa 5 en "Estado", arriba, con el bug de fondo que motivó el cambio.

Todas las anclas del asentamiento —Centro Urbano, Plaza/Pozo/Parque, Plaza de Armas, Patio de Gremios,
Mercado, Carpintería— comparten un **único árbol de crecimiento**, sin distinguir tipo ni categoría entre
ellas. Una Plaza de Armas puede nacer como hija de un Pozo: el tipo de la ancla que se necesita no influye en
NADA de dónde se coloca, solo en qué se planta al final.

- **Semilla activa**: no es un edificio, es un puntero — la ancla que se usa AHORA MISMO como referencia para
  decidir dónde nace la próxima. Entre todas las anclas que existen y no están descartadas (§5.6), es la más
  cercana a la raíz del asentamiento (`semillaActiva`, `engine/trazado.ts`).
- **8 ranuras**: cada ancla tiene 8 direcciones cardinales alrededor de ella donde puede nacer una ancla hija,
  con el eje rotado un poco por asentamiento (§5.8) — mismas 8 direcciones que ya usaba el compás de la Etapa
  4, reutilizadas aquí en vez de para desempatar "frente a él".
- **Ciclo al proponer una ancla nueva** (`crearAnclaNueva`): la semilla activa prueba sus 8 direcciones, en
  orden aleatorio (determinista), hasta encontrar una con hueco real —el candidato tiene que caber sin
  colisionar y respetar `separacionSeguridadAnclas` (§5.7) frente a TODAS las demás anclas— y construye ahí.
  Si ninguna de las 8 sirve, la semilla se descarta (§5.6) y se repite el ciclo con la siguiente semilla más
  cercana a la raíz.
- **Ranura libre antes que ranura ya usada expandiendo radio (post-Etapa 5).** `direccionesBarajadas` da el
  mismo orden de las 8 direcciones cada vez que se consulta la MISMA semilla (determinista por diseño, §5.8) —
  pero eso significaba que, si la primera dirección de la lista ya tenía un ancla hija, `huecoEnDireccion`
  simplemente EXPANDÍA EL RADIO en esa misma dirección en vez de probar una de las otras 7 todavía libres:
  varias anclas hijas de la misma semilla terminaban alineadas en la misma dirección a distancias crecientes,
  en vez de reparties entre las 8. Corregido con `ranuraOcupada` (geométrico: ¿alguna ancla existente cae casi
  exacto en el ángulo de esta dirección, dentro del rango de radio de una ranura?) — `crearAnclaNueva` prueba
  primero las direcciones libres (a la mínima distancia que cada una permita) y solo recurre a una ya usada,
  expandiendo su radio como antes, si ninguna libre sirve.

**Quién dispara el ciclo.** Antes de calcular dónde va cualquier edificio de una categoría con ancla, se
comprueba si ya hay una instancia alcanzable (§5.3); si no la hay, se dispara el ciclo de arriba
(`asegurarAnclaPara`, `engine/construction.ts`) ANTES de tocar la posición del edificio que lo pidió. El
edificio nunca llega a colocarse sin que su ancla ya exista.

### 5.5 Separación entre huecos de una ranura (piso duro)

Igual que la separación de §5.7 entre anclas ya construidas, cada candidato a ranura tiene que respetar
`separacionSeguridadAnclas` frente a TODAS las demás anclas (incluidas las descartadas por saturación — siguen
siendo edificios reales que ocupan territorio). No hay separación "relajable" aparte para esto: si ninguno de
los radios probados en una dirección la cumple, esa dirección se descarta y se prueba la siguiente de las 8.

### 5.6 Saturación y descarte de una semilla

Si la semilla activa no tiene hueco real en NINGUNA de sus 8 direcciones, queda **saturada para siempre**
(`Edificio.semillaSaturada`, estado persistido, se marca una sola vez y nunca se revisa) — no vuelve a
proponerse como semilla en ninguna búsqueda futura, sin importar qué tipo de ancla se necesite después.

Confirmado por simulación abstracta (ver "Estado", Etapa 5) que esto nunca deja el sistema sin salida mientras
haya espacio físico: la búsqueda de la siguiente semilla (la no descartada más cercana a la raíz, entre TODAS,
otra vez sin distinguir tipo) siempre encuentra algo hasta que el 100% del espacio disponible está ocupado —
en ese punto, y solo en ese punto, no hay dónde crecer más para esa categoría.

**Ojo: esto NO es lo mismo que la saturación de §5.3.** Un ancla puede estar `semillaSaturada` (no produce más
anclas) y seguir teniendo hueco de sobra para que se le peguen sus propios satélites — son dos preguntas
independientes, en dos niveles distintos de la jerarquía (ancla→ancla vs. ancla→satélite).

### 5.7 Separación mínima entre anclas

`separacionMinimaAnclas` = 6 celdas entre centros de ancla, y es **relajable**: si ningún hueco la cumple, se
reduce progresivamente hasta que aparezca uno.

**Implementado (2026-08-19), y es un piso aparte: `separacionSeguridadAnclas` = 2 celdas, NUNCA relajable.**
A petición del usuario: "siempre tiene que haber, aunque sea 2 celdas a la redonda de un ancla, donde no puede
haber otra ancla — es como una zona de seguridad." `sitiosParaTipo`/`crearAnclaNueva` (`engine/trazado.ts`)
descartan CUALQUIER candidato para un ancla real nueva que quede a menos de `TRAZADO.separacionSeguridadAnclas`
celdas de OTRA ancla real ya construida (incluidas las descartadas por saturación de §5.6 — siguen siendo
territorio real). Si ningún hueco la cumple, no hay sitio válido — no se relaja ni se ignora, es un piso duro.

**Actualizado en la Etapa 5: `separacionMinimaAnclas` (6) cambió de rol.** Con la semilla de grupo (§5.4/5.5
viejas, ya reemplazadas) era un umbral de "relajación progresiva" entre el edificio semilla y las anclas ya
construidas. Con el árbol único de anclas (§5.4 actual), no hay nada que relajar: `separacionMinimaAnclas`
pasó a ser el **radio inicial** desde el que `crearAnclaNueva` empieza a probar huecos a lo largo de una
dirección (`RADIO_INICIAL_RANURA`), creciendo de `FONDO_MANZANA` en `FONDO_MANZANA` hasta un tope
(`RADIO_MAXIMO_RANURA = separacionMinimaAnclas × 3`). El único piso que de verdad nunca cede sigue siendo
`separacionSeguridadAnclas` (2, fijo) — ahora es el ÚNICO criterio de rechazo duro para una ranura nueva.

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

La misma regla resuelve la **Plaza residencial** (hoy Plaza/Pozo/Parque): es el otro grupo de anclas que
querría nacer temprano, y también necesita su gate. **Sigue vigente bajo el árbol único (Etapa 5)**, aplicado
ahora al mecanismo entero: `NIVEL_GATE_ANCLAS = 2` gatea `asegurarAnclaPara` (`engine/construction.ts`) — por
debajo de nivel 2, una categoría sin ancla alcanzable simplemente no encuentra sitio ese tick (se reintenta el
siguiente) en vez de crear una ancla nueva mal colocada en un disco todavía pequeño. En la práctica militar e
industria nunca lo notan: sus tipos ya exigen nivel 2 para construcción base, así que el gate siempre está
cumplido de antemano; residencial es el único que podría llegar a frenar una Vivienda si el núcleo de Centro
Urbano se satura antes de nivel 2.

### 5.8 Identidad de cada ciudad

**Reescrito en la Etapa 5** — el mecanismo cambió de sitio, no de intención. El sorteo de direcciones
cardinales por categoría (`direccionesDelAsentamiento`) **desapareció** junto con el reparto de barrio: ya no
hace falta, porque el árbol único de anclas (§5.4) no distingue categoría al elegir dónde nace cada ancla.

Lo que sobrevive es `anguloRotacionEje`: un ángulo aleatorio (determinista, `[0°, 45°)`) por asentamiento que
rota el eje de las 8 direcciones cardinales de **todas** las ranuras del árbol por igual — no una dirección
distinta por categoría, un solo giro compartido. Sigue siendo lo que hace que dos ciudades no compartan
geometría aunque construyan exactamente los mismos tipos en el mismo orden: sin el giro, con solo 8 direcciones
fijas, dos asentamientos con el mismo orden de construcción producirían árboles idénticos.

### 5.9 Regla genérica (tipos sin ancla)

Mejor hueco por nivel de preferencia, 360°, el más cercano al origen. La usan Palacio y Muralla (sin geometría
propia) y, desde la Etapa 5 a petición del usuario, también **Almacén y Leñera** —antes tenían su propia cuña
de barrio ("almacenaje"), que desapareció junto con el resto del reparto por barrios—: llenan huecos libres
desde Centro Urbano hacia afuera, en cualquier dirección, en vez de agruparse en una única cuña.

### 5.10 Las anclas encadenan

Un edificio puede ser satélite de un ancla y ancla de sus propias piezas a la vez. La Carpintería es el caso:
orbita la Plaza de Armas y a su vez ancla sus dos talleres (§9). El mapa satélite → ancla es una cadena, no
una partición en dos bandos.

## 6. Tamaños de edificio

Cada tipo ocupa un rectángulo ancho×alto de celdas. El tamaño se deriva de `tipo` (+ `nivelInterno` en Granja
y en las piezas satélite), nunca se persiste en el `Edificio` (`EDIFICIO_TAMANO`, `constants.ts`;
`tamanoEdificio`, `engine/trazado.ts`).

**Orientación intercambiable (Etapa 4).** El tamaño de la tabla es el "normal"; cualquier tipo no cuadrado
—salvo Granja (progresión real de tamaño) y Puesto de mercado (`nivelInterno` ya identifica una forma
concreta)— puede aparecer girado (ancho↔alto) al colocarse, marcado en `Edificio.rotado`. Es lo que da
variedad de silueta entre ciudades: un Mercado 3x2 puede nacer como 3x2 o como 2x3.

| Tipo | Tamaño | Ubicación |
|---|---|---|
| Centro Urbano | 3x3 | origen — primer ancla |
| Plaza | 2x2 | ancla residencial adicional (1 de 3, sorteada) |
| Pozo | 1x1 | ancla residencial adicional (1 de 3, sorteada) |
| Parque | 3x2 | ancla residencial adicional (1 de 3, sorteada) |
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
- **Sin migración de partidas guardadas** (a petición del usuario): el juego está en desarrollo continuo y no
  se mantiene retrocompatibilidad — una partida de una versión de guardado anterior se rechaza al importar con
  un mensaje explícito (`SimulacionExportada.version`, `app/gameStore.ts`) en vez de rellenarse/migrarse. Ver
  también §"Abierto".

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

**Quién dispara la aparición del núcleo militar, por cada camino.** Conviene tenerlo presente antes de
calibrar nada, porque los dos caminos dan resultados distintos — nota post-Etapa-5: dónde cae exactamente la
Plaza de Armas ya no depende de quién la disparó (§5.4, árbol único), solo IMPORTA quién es el primero en
necesitar una y no encontrarla:

- **Auto-construcción / batch.** Barracón, Galería de tiro, Palacio y Muralla **no se auto-construyen** (la
  política de desbloqueo se retiró, ver `avanzarConstruccion` en `engine/construction.ts`) y la gobernanza NPC
  solo añade `mercado` (`app/npcGobernanza.ts`). El único militar que llega a existir es la **Carpintería**, a
  nivel 3, así que normalmente es ella quien dispara `asegurarAnclaPara` y hace que exista la Plaza de Armas.
  El núcleo militar simulado será entonces Plaza de Armas + Carpintería + 2 talleres.
- **Construcción manual.** El jugador sí puede añadir Barracón o Galería de tiro, y entonces cualquiera de los
  dos puede ser quien dispare la aparición de la Plaza de Armas. Con el gate de §5.7 lo más pronto
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
   (`TRAZADO.largoFilaMin`/`largoFilaMax`), mismo mecanismo determinista (`hashTexto` + `pseudoAleatorio` sobre
   el id del asentamiento) que usa el resto del trazado para variar por ciudad — ver §5.8 —, más un desfase
   propio para que dos ciudades no tengan los bloques alineados a las mismas columnas. La manzana queda de
   N × 2 celdas. El criterio es **posicional**: cuando un edificio cae
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
- **`separacionMinimaAnclas` = 6 celdas** sigue sin calibrar por simulación real. Cambió de rol en la Etapa 5
  (§5.7): ya no es un umbral relajable entre edificio-semilla y anclas, es el radio inicial de la búsqueda de
  ranura (`RADIO_INICIAL_RANURA`) y la base del tope (`RADIO_MAXIMO_RANURA = separacionMinimaAnclas × 3`, nuevo
  en la Etapa 5, tampoco calibrado) — sigue siendo el único número real que calibrar en el sistema.
- **Riesgo de la prioridad "pegado al ancla" (§5.3) sobre la alineación de fachadas.** Sigue sin medirse en
  batch. El código documenta una regresión medida en su momento (sin preferir la continuación de fila, la
  ciudad crece como un borrón compacto), mitigada porque la preferencia de fila sigue desempatando dentro de
  cada expansión — pero nunca se confirmó con **número de manzanas cerradas por ciudad** contra una línea base
  real. Sigue siendo lo primero que mirar si se retoma la calibración en batch.
- **Ciclo de vida del ancla.** Qué pasa con los satélites si su ancla desaparece, y cómo se propaga en la
  cadena de §5.10. Hoy es discutible pero no urgente: verificado que **nada destruye edificios activos** en
  Fase 0 — el único sitio que quita un edificio del array es `quitarDeCola`, y solo actúa sobre `en_cola`. Es
  deuda para cuando exista destrucción (incendio, asedio, abandono), no un agujero actual.
- **Referencia de la Lógica 2 (atracción de satélites) sigue midiendo desde el origen, no desde la posición
  real.** `anclaMasCercana`/`anclaActivaParaCategoria` (§5.3.1) —dentro de `sitiosParaTipo`, deciden a qué
  instancia de ancla se atrae un satélite— siguen ordenando las instancias por distancia al ORIGEN del
  asentamiento, no a la posición del edificio que se está colocando. La consecuencia GRAVE que tenía esto (una
  instancia llena se seguía eligiendo para siempre, fabricando anclas nuevas sin parar) ya se corrigió en
  §5.3.1 — lo que queda pendiente es solo la imprecisión geométrica: un satélite puede atraerse a una
  instancia más lejana-pero-igual-de-válida en vez de la geométricamente más cercana a él. Riesgo leve, no
  correctness — pendiente si algún día vale la pena afinarlo.
- **Ancla creada sin garantizar que tendrá sitio para al menos un satélite.** `asegurarAnclaPara`/
  `crearAnclaNueva` (§5.4) garantizan que el ANCLA en sí cabe (hueco real + separación, §5.5/§5.7) antes de
  colocarla, pero no comprueban que, una vez colocada, `sitiosPorAtraccionDura` (§5.3) vaya a encontrarle sitio
  a ni un solo satélite de la categoría que la disparó. Caso real observado con una semilla de prueba fija: un
  Patio de Gremios nace consistentemente sin que ningún edificio de industria llegue a pegársele nunca —
  huérfano desde el tick en que se crea. Coincide con el diagnóstico original que motivó toda la Etapa 5 (ver
  arriba, "El motivo"): el ancla nace resuelta geométricamente, pero nadie verifica que resuelva el problema
  que la originó. Sin corregir todavía.
- **Nombres provisionales que siguen provisionales.** `Pozo` y `Parque` (Etapa 4) son nombres de trabajo,
  igual que lo fueron en su momento `Plaza`/`Plaza de Armas`/`Patio de Gremios`/`Taller de carpintería` (esos
  cuatro ya están fijados en el catálogo desde la Etapa 3).

**Resueltos desde la última revisión de esta sección** (quedan aquí solo como referencia, no como pendientes):

- ~~Migración de partidas guardadas al modelo de anclas~~ — decisión distinta a la que este documento
  proponía: en vez de rellenar partidas viejas al cargar, se **eliminó toda retrocompatibilidad** (a petición
  del usuario: "estamos en un proceso de desarrollo continuo... lo anterior se borran y se crean nuevas
  partidas"). Una partida de versión de guardado anterior se rechaza al importar (§8).
- ~~A qué nivel se gatea la Plaza residencial~~ — resuelto en la Etapa 3: `NIVEL_GATE_ANCLAS = 2`, uniforme
  para las tres categorías con ancla (§5.7).
- ~~Reflow al subir de nivel~~ — decisión cerrada con el usuario en la Etapa 3: **sin reflow**. Un núcleo
  saturado con el disco pequeño conserva su ancla de saturación para siempre.
- ~~Política "Líneas de Producción" pendiente de implementar~~ — implementada en la Etapa 5:
  `sitiosPorAtraccionDura` acepta `ampliado`, que devuelve todos los candidatos dentro del núcleo del ancla en
  vez de detenerse en el primero, para que la política elija por distancia a la fuente de insumos.
