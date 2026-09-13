# Vista de Asentamiento — Trazado urbano dinámico

Documento vivo. Recoge el estado actual del diseño; lo que sigue sin implementar va al final, marcado como
abierto.

> **Estado real (Etapa 5, última entrada del log de abajo): el sistema de anclas está implementado por
> completo** — árbol único de anclas, orientación intercambiable, variedad residencial, compás + eje rotado.
> El log de "Estado"/"Etapa N" que sigue es cronológico e histórico: la primera entrada ("CERO código de
> anclas") describe el arranque del diseño, no el estado actual. Las secciones numeradas (§1 en adelante) ya
> reflejan la implementación de la Etapa 5, no las etapas anteriores.
>
> **Excepción: la Etapa 6 (calles como celdas) está implementada hasta su Paso 2** — las calles ocupan celdas
> desde 2026-08-31. Las secciones numeradas (§1 en adelante) siguen describiendo el modelo de ARISTAS y están
> pendientes de reescritura. Ver "Etapa 6" al final del log.

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

## Etapa 6 (2026-08-30): las calles pasan de aristas a CELDAS — diseño cerrado, CERO código escrito

> **ESTADO (2026-08-31): implementada hasta el Paso 2 incluido.** Las calles ya corren sobre CELDAS en el
> código. §1-§4, §5.2/§5.3, §6 y §10-§12 más abajo siguen describiendo el modelo de ARISTAS y por tanto **ya
> no describen el código** — se reescriben cuando la Etapa 6 se cierre del todo (ver la lista final de §E6.15),
> no antes, para no dejar el documento a medias entre dos mecanismos. Hasta entonces, la referencia válida
> sobre cómo funciona el trazado HOY es esta sección, no las numeradas.

El disparador declarado (el paso a 3D, enunciado histórico §7 de `Docs/Mecanicas a desarrollar.md`, ya
retirado de esa lista) es que una calle sobre una arista no tiene ancho, y en 3D hay que darle uno. Al medir
el modelo actual antes de diseñar el reemplazo, el problema resultó ser bastante más grande que un detalle de
representación.

### E6.1 La medición

Simulación real con los fixtures del propio repo (`crearMapaDeterminista` / `fundarAsentamientoDeTest`,
300 ticks — el mismo montaje que `engine/__tests__/trazado.test.ts`), tres seeds:

| Medida | seed 99 | seed 7 | seed 42 |
|---|---|---|---|
| Aristas de calle de **ancho cero** (separan dos edificios DISTINTOS, ambos ocupando su celda) | 25/29 = **86%** | 24/29 = **83%** | 78/97 = **80%** |
| Edificios con **frente de calle real** (arista de la red con celda LIBRE al otro lado) | 8/25 = **32%** | 8/28 = **29%** | 18/73 = **25%** |
| Núcleo urbano (sin afueras): caja y ocupación | 7x7, **69%** | 7x7, **73%** | 17x14, **48%** |
| `radioPotencial` disponible | 60 (≈20 celdas Ø) | 60 | 90 |

**Entre el 80% y el 86% de la red de calles no existe físicamente.** Son aristas marcadas como calle que
corren por el muro compartido de dos edificios pegados pared con pared. Solo uno de cada cuatro edificios
tiene delante algo por lo que se pueda caminar. Y el núcleo urbano es un coágulo de 7x7 celdas al 70% de
ocupación dentro de un disco de ~20x20: **no falta espacio, sobra.**

**Corrección al instrumentar el batch (mismo día): a escala real el ancho cero es del 51%, no del 80-86%.**
La tabla de arriba salió de ciudades de UN asentamiento aislado (fixture de `trazado.test.ts`, 300 ticks). La
línea base reproducible —`BATCH_SEED=7 BATCH_FACCIONES=20 BATCH_TICKS=1500`, gobernanza NPC, 18 asentamientos
vivos, el mismo montaje con el que se midieron las Etapas 1 y 2— dice:

| Métrica | Línea base (tick 1500) |
|---|---|
| `callesAnchoCeroPct` | **51.04%** |
| `edificiosConFrenteRealPct` | **32.98%** |
| `ocupacionNucleoPct` | **64.12%** |
| `manzanasCerradasMedia` | 3.89 |
| `dispersionViviendaCentro` | 3.44 |

La diferencia tiene explicación y no invalida el diagnóstico: una ciudad grande abre más anclas, y cada anilla
de ancla aporta tramos que sí dan a suelo libre, así que el porcentaje de ancho cero baja con el tamaño. **Los
números que valen son los del batch** (reproducibles desde el repo); los de la tabla anterior se dejan porque
son los que dispararon la investigación, no como cifra de referencia. Aun con el número bueno: **la mitad de
la red no existe y dos de cada tres edificios no tienen por dónde salir.**

**Ojo con el techo del batch**: las tres métricas quedan CONGELADAS entre el tick 500 y el 1500 (valores
idénticos) — las ciudades dejan de crecer ahí por el colapso de la transición a nivel 2 ya documentado en
"Estado". La línea base es válida para comparar, pero no ejercita ciudades grandes; para eso hace falta
resolver antes ese colapso, o medir con un fixture aparte.

### E6.2 El diagnóstico: el problema es económico, no geométrico

Una arista es **gratis**. Pegar dos edificios pared con pared regala una "calle" en el borde compartido, sin
pagar suelo. Y la atracción dura (§5.3: *"la prioridad de un satélite es estar lo más pegado posible a su
ancla; eso manda sobre todo lo demás"*) empuja activamente a apelotonar. Nada en el sistema paga por el
espacio de circular, así que nadie lo deja: las manzanas emergentes de §10 no emergen, y las que el batch
contaba como cerradas (`manzanasCerradasMedia` 2.78, Etapa 2) se apoyan mayoritariamente en calles de ancho
cero.

De ahí la lectura que ordena todo el rediseño:

> **Las celdas arreglan el 3D como efecto secundario. Lo que arreglan de fondo es que la calle pase a costar
> suelo** — y ese coste es la única presión capaz de producir hileras y manzanas de verdad.

Las tres garantías "gratis" de §1 resultan ser gratis porque no garantizan nada real. Con celdas, dos de las
tres pasan de garantía a obligación:

| Garantía hoy gratis (por aristas) | Con celdas |
|---|---|
| Un edificio nunca cae encima de una calle | **Validación**: las celdas de calle entran en `ocupadas` |
| Una calle nunca cae encima de otra | **Sigue gratis** (`Set` de celdas) |
| Un tramo nunca atraviesa un edificio | **Validación**: el trazado rodea, no cruza |
| Todo edificio toca la red | Cambia de *"comparte una arista"* a *"es adyacente a una celda de calle"* — hoy lo cumpliría el 25-32% |

### E6.3 Decisiones cerradas con el usuario (2026-08-30)

| # | Decisión | Elegido |
|---|---|---|
| 1 | Ancho de calle | **1 celda uniforme** (= media Vivienda; sin jerarquía avenida/callejón por ahora) |
| 2 | Cómo se decide qué celda es calle | **Retículo blando**: preferencia fuerte, nunca reserva dura |
| 3 | Conectividad | **Grafo conexo, invariante duro** — se congela como test |
| 4 | Alcance | **Representación + densidad**: incluye revisar la atracción dura, causa medida del coágulo |
| 5 | Ancla vs. coste de calle en el scoring | **Ancla primero**; la calle entra como GATE duro y como desempate |

### E6.4 La conectividad sale gratis, por inducción

La decisión 3 suena cara (comprobar puntos de articulación en cada colocación). No hace falta ninguna
comprobación:

- Las celdas de calle están en `ocupadas`, así que **ningún edificio puede partir jamás un corredor
  existente**. El riesgo clásico desaparece por construcción.
- La red arranca conexa: el anillo del Centro Urbano.
- Cada edificio nuevo cae en uno de dos casos: **(a)** ya es adyacente a una celda de calle → no añade nada,
  la red no cambia; **(b)** reclama un corredor de celdas libres desde su perímetro hasta la red → un corredor
  es un camino, así que la unión sigue conexa.

Conexa por inducción, sin excepciones. El coste real de la decisión no es el chequeo: es que **(b) puede no
existir** — a 70% de ocupación local puede no quedar corredor libre. Eso obliga a lo siguiente.

### E6.5 La conexión pasa de reparación a CONDICIÓN DE VALIDEZ

`conectarEdificio` (§4) corre hoy DESPUÉS de colocar, y no puede fallar nunca: siempre hay una arista
disponible. Con celdas sí puede fallar, y un fallo posterior sería exactamente el bug que costó la Etapa 5
—*"el edificio se colocaba primero y el ancla se intentaba después, fallando en silencio"*— trasladado a la
capa de calle.

**Un candidato que no se pueda conectar no es un candidato.** La conexión se evalúa dentro del filtro de
candidatos, nunca después de elegir.

### E6.6 La pieza técnica: campo de distancia a la calle

La versión ingenua de §E6.5 sería un BFS por candidato — inviable: `candidatosLibres` produce miles por
colocación. Invertido:

> **Un BFS multi-origen desde TODAS las celdas de calle a la vez, sobre celdas libres, una sola vez por
> colocación**, que rellena `distanciaACalle[celda]` para todo el disco en una pasada.

Con ese campo, el coste de conexión de un candidato es un *lookup*: el mínimo del campo sobre las celdas de su
perímetro. Sustituye de golpe a `conectarEdificio`, al chequeo de conectividad y al nivel 0-3 de `Candidato`, y
entrega gratis el número que pide la decisión 4: **cuántas celdas de suelo cuesta poner el edificio ahí**.
Es más barato que el `continuaFila` por candidato de hoy. El corredor concreto se traza solo para el candidato
ganador, descendiendo el gradiente del campo — un único trazo por colocación.

### E6.7 "Pegado al ancla" pasa de gap 0 a gap 1 (la trampa)

Toda ancla siembra su anillo (§5.2). Con celdas, ese anillo **ocupa** el marco de celdas que rodea al ancla.
Por lo tanto:

> `gapCeldas(satélite, ancla) == 0` se vuelve **geométricamente imposible** para cualquier ancla con anillo.

Si `sitiosPorAtraccionDura` se reescala sin más y sigue buscando gap 0, no encontrará nada nunca y caerá al
fallback — que es **literalmente** el bug corregido en §5.7 (*"38/38 piezas de Mercado con gap 0 respecto a su
ancla; antes algunas quedaban a 2 filas"*), reintroducido entero y en silencio.

**El objetivo pasa a ser gap 1: el satélite mira a su ancla desde el otro lado de la calle del anillo.** Que
además es lo correcto en 3D — un satélite pegado sin calle en medio no tiene puerta. Es una reescritura del
criterio, no un reescalado.

### E6.8 Retículo blando = predicado de celda

`esBordeDeManzana(indice, paso, desfase)` responde hoy *"¿esta LÍNEA es un borde de manzana?"*. Pasa a
responder *"¿esta columna/fila es de CALLE?"*, con período `largoFila + anchoCalle` (el bloque más su calle),
no `largoFila`.

Sigue siendo **preferencia, no reserva** (decisión 2): un candidato que deja su calle sobre la línea del
retículo puntúa mejor, pero un edificio grande —Galería de tiro, 4x8 celdas nuevas— puede atravesarla y
desviar la calle localmente. Se mantiene intacto el principio rector: nada se pre-genera, solo hay un criterio
de dónde caería la calle el día que alguien la necesite.

**Coste de suelo**: bloque de 12x4 celdas de edificio con período 13x5 → **26% de sobrecoste**. El núcleo
medido (34 celdas ocupadas) necesitaría ~46 → sigue siendo una caja de 7x7 dentro de un disco de 20x20.
**Ningún radio necesita recalibrarse** (`ZONA_INFLUENCIA`, `radioAfuerasMin`, `radioMapa` se quedan igual).

### E6.9 Orden del scoring (decisión 5)

Se **mantiene** §5.3: la cercanía al ancla sigue siendo el criterio principal, y la legibilidad de distritos
que costó cinco etapas no se pone en riesgo. La presión anti-coágulo viene del gate, no de invertir la
prioridad:

1. **Gate duro** — coste de conexión ≤ `capCorredor`; si no, candidato inválido (§E6.5).
2. **Cercanía al ancla** (`gapCeldas`, objetivo 1 y no 0 — §E6.7).
3. **Coste de calle** (menos celdas nuevas mejor) — el término que hoy no existe.
4. **Alineación al retículo** (§E6.8).
5. Continuación de fila / borde compartido / semilla determinista — sin cambios.

### E6.10 El cap de corredor es POR CLASE, no global

Un cap único rompería las afueras: Granja y Corral viven a ≥20 celdas del centro por diseño
(`radioAfuerasMin`) y su camino es largo **a propósito**. `esDeAfueras` ya existe como discriminador:

- **Urbano**: cap corto — es lo que impide enterrarse dentro de un coágulo.
- **Afueras**: cap largo o nulo; su corredor es un `camino`, no una calle (§11 se mantiene: son dos clases,
  no dos grosores).

Conectividad (§E6.4) se mide sobre la UNIÓN de calles y caminos, igual que hoy hace `yaConectado`.

### E6.11 La escala ×2 no cuesta nada

> **Nota (2026-09-13):** BA-005 (§E6.24) deshizo el ×2 de las huellas y de las constantes de escala de
> edificio, pero NO la celda de 3. Las cifras de esta sección son las de su momento.

`tamanoCelda: 6 → 3` y todas las huellas ×2. El álgebra se conserva: `puntoDeRectangulo` da **exactamente el
mismo punto local** (`6·col + 3·ancho` en ambos modelos). (El Centro Urbano tenía una excepción de
coordenadas —resuelta en §E6.20, 2026-08-31: su `posicion` ahora es su centro, sin caso especial.)

Consecuencias:

- **`Edificio.posicion` no se mueve ni un decimal → cero migración de snapshot.**
- La red ya es derivada (`GeometriaAsentamientos`, calculada en `RunnerDePartida`, nunca persistida) →
  tampoco migra.
- El único contrato que cambia es `TrazadoAsentamiento.calles` (de `SegmentoTrazado[]` a celdas/rectángulos):
  DTO derivado, así que es esquema + cliente, sin tocar disco. Conviene emitir **tiradas de celdas fusionadas**
  (mismo formato que `huellas`) y no celda a celda, por el presupuesto de payload de doc 6.
- **Coste de CPU: ×4 candidatos** por colocación (`candidatosLibres` barre `(2·radio/T)²`). Es la razón por la
  que ×2 es el factor correcto y no uno mayor: cuadruplicar la resolución sería ×16 y el laboratorio batch
  (100 facciones × 3000 ticks) es una herramienta que no se puede permitir perder. **Medir antes de
  comprometerse.**

| Constante | Hoy | Nueva | Nota |
|---|---|---|---|
| `REJILLA_ASENTAMIENTO.tamanoCelda` | 6 | **3** | preserva coordenadas locales |
| `EDIFICIO_TAMANO.*`, `granja.niveles[n].tamano`, `PUESTO_MERCADO_FORMA` | — | **×2** | |
| `FONDO_MANZANA` | 2 | **4** | sigue siendo 2 hileras espalda con espalda |
| `TRAZADO.largoFilaMin` / `largoFilaMax` | 4 / 8 | **8 / 16** | |
| `TRAZADO.separacionMinimaAnclas` | 6 | **12** | `RADIO_INICIAL_RANURA`/`RADIO_MAXIMO_RANURA` derivan solas |
| `TRAZADO.separacionSeguridadAnclas` | 2 | **4** | |
| `TRAZADO.anchoCalle` | — | **1** (nueva) | decisión 1 |
| `radioAfuerasMin` / `anchoBandaAfueras` / `radioMapa` | 60 / 36 / 150 | **sin cambio** | ya están en unidades locales |

### E6.12 Invariantes a congelar

| `trazado.test.ts` hoy | Etapa 6 |
|---|---|
| ningún par de edificios comparte celda | **se mantiene**, + ningún edificio pisa una celda de calle |
| todo edificio toca la red por una arista de su perímetro | **todo edificio es adyacente a una celda de calle** (hoy lo cumpliría el 25-32%) |
| ningún tramo es diagonal ni se repite | **obsoleto** (no hay tramos) → lo sustituye: la red es **un único componente conexo** |
| Granja y Corral se quedan a las afueras | se mantiene |
| la red es determinista | se mantiene |

Y uno nuevo que hoy no se puede ni formular: **toda celda de calle es alcanzable a pie desde el Centro
Urbano** — que es lo que 3D necesita de verdad y lo que el modelo de aristas nunca garantizó.

### E6.13 Qué NO se toca

El árbol único de anclas (§5.4), `semillaSaturada` / `anclaLlena` (§5.3.1/§5.6), las categorías, el sorteo de
tipo de ancla, el eje rotado (§5.8), las afueras (§11), la orientación intercambiable y el desempate por borde
compartido (Etapa 4, puntos 1-2). Todo eso razona en **celdas y rectángulos**, no en aristas: sobrevive con las
constantes escaladas. El revamp es quirúrgico sobre la capa de red y el filtro de candidatos, no sobre el
sistema de anclas.

### E6.14 Línea base y coste operativo

- ~~**Instrumentar el batch ANTES de tocar nada**~~ — **hecho (2026-08-30)**, igual que hizo la Etapa 0 y antes
  de tocar una línea de `engine/trazado.ts`. `scripts/run-batch-sim.ts` gana `medirCalles` y tres campos en
  `Foto`, junto a `dispersionViviendaCentro`/`manzanasCerradasMedia`, que ya vivían ahí:

  | Campo | Qué mide | Valor esperado tras la Etapa 6 |
  |---|---|---|
  | `callesAnchoCeroPct` | % de tramos que separan dos edificios distintos | **0**, por construcción |
  | `edificiosConFrenteRealPct` | % con un tramo de red y celda LIBRE enfrente | **100**, es el invariante de §E6.12 |
  | `ocupacionNucleoPct` | % de ocupación del núcleo (sin afueras) en su propia caja | **debe bajar** — es el coágulo que afloja la decisión 4 |

  Las tres trabajan sobre `segmentosDeRed` (coordenadas locales) y `celdasDeEdificio`, **nunca sobre las claves
  de arista**: el formato `H i,j`/`V i,j` es interno de `trazado.ts` y desaparece con la Etapa 6, así que la
  instrumentación no puede depender de él o moriría con el cambio que debe medir. Sobreviven al cambio de
  modelo sin tocarse.
- **Las partidas guardadas hay que borrarlas.** No por migración (no hay: §E6.11), sino porque sus edificios
  están colocados bajo reglas viejas y, al recalcularse la red, pueden no satisfacer la conectividad de §E6.4.
  Es la política ya establecida del proyecto (§8, "Resueltos"), pero aquí hay que hacerla explícita en el plan.

### E6.15 Plan de ejecución

Misma convención que `Docs/Arquitectura/4_Plan_Evolucion_Tareas.md`: checkbox por tarea, y cada paso se cierra
anotando aquí lo que se midió de verdad, no lo que se esperaba medir.

**El criterio que ordena los pasos es la atribución**: cada paso cambia UNA cosa medible, para que un
movimiento en las métricas tenga un solo culpable posible. Es el método de `git stash` + seed fija que ya
usaron las Etapas 1 y 2 (§"Etapa 1"/"Etapa 2"), donde separar los cambios fue lo único que permitió leer los
resultados.

#### Paso 0. Línea base instrumentada — ✅ completada 2026-08-30
- [x] `medirCalles` + `celdasSeparadasPor` + `celdaDePunto` en `scripts/run-batch-sim.ts`; tres campos nuevos
      en `Foto` (`callesAnchoCeroPct`, `edificiosConFrenteRealPct`, `ocupacionNucleoPct`)
- [x] Construidas sobre `segmentosDeRed`/`celdasDeEdificio`, nunca sobre claves de arista (§E6.14) — así
      sobreviven al cambio de modelo que tienen que medir
- [x] Corrida de referencia `BATCH_SEED=7 BATCH_FACCIONES=20 BATCH_TICKS=1500`, 18 vivos, 0 excepciones:
      **`callesAnchoCeroPct` 51.04 · `edificiosConFrenteRealPct` 32.98 · `ocupacionNucleoPct` 64.12 ·
      `manzanasCerradasMedia` 3.89 · `dispersionViviendaCentro` 3.44**
- [x] **Corrección registrada**: la cifra de cabecera 80-86% de §E6.1 era de ciudades de un solo asentamiento;
      a escala de batch el ancho cero es 51%. Los números del batch son los válidos
- [x] Anotado el techo del instrumento: las métricas se congelan del tick 500 al 1500 (las ciudades dejan de
      crecer por el colapso de transición a nivel 2) — la línea base no ejercita ciudades grandes
- [x] 685/685 tests, `tsc --noEmit` limpio, `engine/trazado.ts` sin tocar

#### Paso 1. La escala ×2, todavía sobre aristas — ✅ completada 2026-08-31, con un hallazgo que abre el Paso 1b
Aísla la variable de COSTE antes de mezclarla con el cambio de modelo. **No es un no-op**: la rejilla más fina
habilita posiciones a media celda que antes no existían, así que las colocaciones cambian aunque el modelo de
red no.
- [x] `REJILLA_ASENTAMIENTO.tamanoCelda` 6 → 3
- [x] `EDIFICIO_TAMANO.*`, `EDIFICIO_CATALOGO.granja.niveles[n].tamano` y `PUESTO_MERCADO_FORMA` ×2
- [x] `FONDO_MANZANA` 2 → 4; `TRAZADO.largoFilaMin`/`largoFilaMax` 4/8 → 8/16;
      `separacionMinimaAnclas` 6 → 12; `separacionSeguridadAnclas` 4
- [x] `radioAfuerasMin`/`anchoBandaAfueras`/`radioMapa` NO se tocan — están en unidades locales; anotado en la
      propia constante para que no se reescalen por inercia en el futuro
- [x] **El default oculto**: `tamanoEdificio` devolvía el literal `{1,1}` para los tipos ausentes de la tabla
      (Vivienda, Leñera, las 3 minas, Gran Fundición, Maravilla, Muralla). Reescalar solo la tabla habría dejado
      al edificio MÁS NUMEROSO de cualquier ciudad a la mitad de su tamaño físico, con síntoma "las casas
      encogieron" y no un error de tipos. Extraído a `EDIFICIO_TAMANO_POR_DEFECTO` (`constants.ts`)
- [x] Conservación de coordenadas **probada, no afirmada**: `engine/__tests__/escalaRejilla.test.ts` — 40 casos
      (todos los tipos + niveles de Granja + formas de Puesto + rotaciones) generados con el código real a
      `tamanoCelda = 6` y anclados en la celda (1,−2), que siguen en verde a `tamanoCelda = 3` sin editar un
      número. Es una aserción CRUZADA ENTRE VERSIONES; ahí está todo su valor. Incluye un test genérico de que
      `posicion`↔celda son inversas exactas a cualquier escala
- [x] **`CELDA_METRICA` congelada en 6** (`scripts/run-batch-sim.ts`): las métricas de distancia
      (`dispersion*`, `UMBRAL_COMPONENTE_CELDAS`) estaban expresadas EN CELDAS, así que al partir la celda por
      la mitad se habrían duplicado solas —3.44 → 6.88 sin que nada empeorase— y la comparación contra el Paso 0
      y contra los números históricos de las Etapas 1 y 2 habría quedado corrupta en silencio
- [x] Tests: 685 → **688**, `tsc --noEmit` limpio, `cliente/` compila (deriva el tamaño de celda del catálogo)

**Métricas, corrida de referencia idéntica al Paso 0** (`BATCH_SEED=7 BATCH_FACCIONES=20 BATCH_TICKS=1500`):

| Métrica | Paso 0 | Paso 1 | Lectura |
|---|---|---|---|
| **tiempo de corrida** | 1m40s | **8m08s** | **×4.9** — ver Paso 1b |
| vivos / colapsados | 18 / 2 | 18 / 2 | la SIMULACIÓN no se movió |
| `viviendasMedia` / `pesantsMedia` | 13 / 168.17 | 13.06 / 168.11 | idem |
| `ocupacionNucleoPct` | 64.12 | **56.71** | **−7.4 puntos: el coágulo se aflojó solo** |
| `dispersionViviendaCentro` | 3.44 | **3.13** | −9%, más pegado al ancla |
| `dispersionPuestoMercado` | 2.38 | **2.27** | −5% |
| `manzanasCerradasMedia` | 3.89 | 3.50 | −10%, leve regresión del guardián |
| `conAlgunaManzanaCerrada` | 18 | 18 | sin cambio |
| `callesAnchoCeroPct` | 51.04 | 47.13 | −3.9, esperado: el MODELO no cambió |
| `edificiosConFrenteRealPct` | 32.98 | 32.08 | −0.9, idem |

Lo relevante: **la simulación es idéntica** (mismos vivos, colapsos, población y viviendas), así que el
reescalado es puramente geométrico, como se diseñó. Y la rejilla más fina ya afloja el coágulo 7 puntos y
acerca los satélites a su ancla, gratis, antes de tocar el modelo de calles. La regresión de
`manzanasCerradasMedia` queda anotada para vigilarla en el Paso 3 — pero mide ciclos de una red que sigue
siendo falsa en un 47%, así que su valor informativo hoy es limitado.

`snapshot_baseline` re-baselineado con un diff de **3 líneas en todo el fichero**: una Vivienda que arranca
obra un tick más tarde (y sus 10 de madera sin gastar). Todos los cortes posteriores quedaron idénticos byte a
byte ⇒ la obra se hizo y la partida converge; no es una construcción perdida.

Dos tests necesitaron arreglo, ninguno por un bug del motor:
- `mejoraManual`: la Granja al mejorar ya NO cambia de `posicion`. Con la rejilla original, una huella de alto
  par y otra de alto impar no podían compartir centro (habría exigido media celda); ahora sí, y la Granja se
  muda una celda conservando el centro. El edificio sí se mueve — dejó de ser cierto que moverse implique
  cambiar de `posicion`. El test compara ahora la HUELLA.
- `anclasSatelites`: el helper `ocupante(col, row)` prometía "ocupa ESTA celda" y los bucles de relleno
  iteraban celda a celda contando con eso; con Vivienda a 2x2 cada relleno se desbordaba sobre sus vecinos,
  incluido el hueco que el test dejaba libre a propósito. Síntoma: "0 candidatos", que no dice nada de la
  causa. Ahora hay un `rellenar()` que recorre a pasos de la huella real y descarta por solape de rectángulos.
  Además el margen de relleno se DERIVA de `RADIO_MAXIMO_RANURA` (exportada) en vez de ser un literal que el
  reescalado dejó corto.

#### Paso 1b. Coste de la colocación — ✅ completada 2026-08-31 (bloqueo del Paso 2 levantado con evidencia)
El Paso 1 disparó su propio punto de abandono: **×4.9 en tiempo de corrida**. Proyectado al batch por defecto
(100 facciones × 3000 ticks, ~10× esta corrida) son **~17 min → ~80 min**, impracticable para una herramienta
de calibración iterativa. Y el laboratorio de balance no es opcional: es lo que sustituye al equipo de QA que
el proyecto no tiene (doc 10 §5).

**El factor de escala NO se replantea.** ×2 viene exigido por el diseño (`anchoCalle = 1` = media Vivienda,
decisión 1 de §E6.3) y bajarlo mataría el modelo entero. El coste es un artefacto de fuerza bruta, no algo
intrínseco:

- `candidatosLibres` barre `(2·radioPotencial/T + 1)²` posiciones → **×4** al doblar la resolución.
- Y para CADA posición comprueba celda a celda si la huella cabe, `O(ancho×alto)` — y la huella también se
  cuadruplicó. Los dos factores se multiplican; de ahí que 4.9 supere al ×4 previsto.

- [x] **Tabla de sumas acumuladas 2D** (`OcupacionAcumulada`, `engine/trazado.ts`) sobre celdas ocupadas,
      construida una vez por barrido recorriendo `ocupadas` (cientos de entradas) y no la caja (decenas de
      miles). "¿Cabe este rectángulo?" pasa de `O(ancho×alto)` consultas con clave de texto a una resta de
      cuatro enteros; la comprobación de "pared con pared" pasa de `O(ancho+alto)` a cuatro consultas O(1).
      **8m08s → 4m57s.**
- [x] **Decorar-ordenar-desdecorar en los dos `sort`.** `distanciaAlOrigen` (`Math.hypot`) y
      `semillaCandidato` (plantilla de texto + FNV-1a + `Math.sin`) son CONSTANTES por candidato y se estaban
      recalculando dentro del comparador, que corre `O(n log n)` veces. **5m02s → 3m48s.**
- [x] **Hipótesis refutada, y queda escrita en el código.** Se probó evitar el array intermedio de
      `aristasDeRectangulo` con un bucle de salida temprana (`algunaAristaEnRed`), suponiendo que dominaba la
      asignación: **4m57s → 5m02s, o sea nada**. El coste no es el array, son las 2·(ancho+alto) claves de
      texto por candidato. Se dejó la función porque no asigna y hace legible el perfil, pero con el resultado
      negativo anotado para que nadie lo vuelva a intentar por el mismo camino.
- [x] Métricas **idénticas** en las tres iteraciones (`callesAnchoCeroPct` 47.13, `frenteReal` 32.08,
      `ocupacionNucleo` 56.71, `manzanas` 3.50, dispersiones 2.27/3.13, 18 vivos / 2 colapsados) y 688/688
      tests, incluida la línea base de snapshot ⇒ la colocación es bit a bit la del Paso 1. Es la única
      garantía válida para un cambio puramente de rendimiento.

**Resultado: 8m08s → 3m48s (2.1× más rápido).** El objetivo era "volver al orden de 1m40s" y **no se alcanzó**:
quedan 2.3× sobre el Paso 0.

**Pero el objetivo estaba mal planteado, y el perfil lo demuestra.** Medido con `node --cpu-prof` después de la
pasada (total muestreado 26.2s → 19.3s, −26%):

| Función | % del tiempo restante | ¿Sobrevive al Paso 2? |
|---|---|---|
| `algunaAristaEnRed` | **31.1%** | **no** — es la consulta de frente de calle POR ARISTAS |
| `candidatosLibres` (el barrido en sí) | 15.0% | sí, es el coste inherente del ×4 de posiciones |
| `extremosDeArista` | 6.8% | **no** — parsea claves de arista |
| `porDistanciaAlOrigen` | 5.3% | sí |
| `verticesDeRed` | 4.8% | **no** — reconstruye los vértices de la red de aristas |
| `pseudoAleatorio` | 4.1% | sí (era 11.9% antes de la pasada) |

**~43% de lo que queda es maquinaria de ARISTAS que el Paso 2 borra por construcción**: con calles sobre
celdas, "¿tiene frente de calle?" pasa a ser una lectura sobre la misma clase de tabla que `OcupacionAcumulada`
—sin texto, en O(1)— y `extremosDeArista`/`verticesDeRed` dejan de existir. Seguir optimizando la
representación de aristas sería trabajo que el Paso 2 borra.

- [x] **El bloqueo del Paso 2 se levanta con esta evidencia**: el Paso 2 no añade coste sobre un punto de
      partida malo, es *parte de la solución*. Lo que sí queda es volver a medir el tiempo al cerrarlo y, si
      entonces sigue lejos del Paso 0, atacar el 15% de `candidatosLibres` (el ×4 de posiciones es inherente a
      la resolución, pero el barrido podría acotarse a anillos en vez de al disco entero).

#### Paso 2. El modelo de celdas — ✅ completada 2026-08-31 (absorbió el Paso 3 y el Paso 4)
Las tres piezas van juntas porque §E6.4 y §E6.5 son la misma decisión: sin el campo de distancia no hay forma
barata de convertir la conexión en validez, y sin validez la conexión falla en silencio.
- [x] `RedDeCalles` pasa de `Set` de claves de arista a `Set` de CELDAS (`"col,row"`, el mismo espacio que
      `celdasOcupadas`). `calles`/`caminos` siguen separados
- [x] `redDeCalles` reescrita: el anillo del Centro Urbano y el de cada ancla son marcos de celdas
- [x] Campo de distancia (§E6.6): `DistanciaALaCalle`, BFS multiorigen desde todas las celdas de calle a la
      vez, **una pasada por barrido** en vez de un BFS por candidato
- [x] Las celdas de calle entran en `ocupadas` (§E6.2) — centralizado en `sueloOcupado()`, una sola función,
      porque olvidarlo en UNO de los cuatro puntos de colocación bastaba para plantar edificios sobre las
      calles, y el síntoma habría sido visual y tardío, no un error de tipos
- [x] `conectarEdificio` eliminada. La conexión es GATE DURO dentro de `candidatosLibres`: un sitio del que no
      se puede salir a la calle no es un sitio. El corredor se traza solo para el ganador (`corredorHastaLaRed`)
- [x] Invariantes nuevos en `trazado.test.ts` (§E6.12): ningún edificio pisa celda de calle · todo edificio
      tiene celda de calle adyacente · **la red es un ÚNICO componente conexo alcanzable a pie desde el Centro
      Urbano**. Retirado "ningún tramo diagonal ni repetido" (ya no hay tramos)
- [x] 685 → **689 tests**, `tsc` limpio en motor, `scripts/` y `cliente/`, `vite build` verde

**Dos fallos de diseño que solo aparecieron al escribir el código.** Los dos estaban en este documento, dados
por buenos:

1. **La inducción de conectividad de §E6.4 tenía un hueco.** El argumento —"la red arranca conexa y cada
   edificio o ya la toca o abre un corredor, que es un camino"— es correcto pero **solo cubre el corredor**.
   El anillo de un ancla y las franjas del retículo se añadían incondicionalmente y podían nacer sueltas:
   medido, **48 de 95 celdas de calle inalcanzables a pie** en un asentamiento real. Cerrado con
   `anadirConectadas`, que solo añade las celdas del grupo que conectan con la red existente. Resulta MÁS fiel
   al principio rector que la versión anterior: una franja del retículo que todavía no llega a la ciudad
   simplemente no existe aún, que es literalmente lo que dice §10.

2. **§E6.7 (gap 0 → gap 1) NO era separable, y este plan lo tenía mal.** Estaba asignado al Paso 3. En cuanto
   las anclas siembran anillos de celdas, `gapCeldas(satélite, ancla) === 0` se vuelve geométricamente
   imposible y los tests lo demuestran de inmediato — no hay un estado intermedio en el que el Paso 2 esté
   hecho y el Paso 3 pendiente. Resuelto midiendo contra el ancla **expandida por su anillo**
   (`rectAnclaConAnillo`): así "hueco 0" recupera su significado —el satélite mira a su ancla desde el otro
   lado de la calle— y tanto el bucle de anillos como el desempate por borde compartido siguen valiendo sin
   tocarse. **El Paso 3 queda por tanto absorbido en su parte de §E6.7**; lo que sigue abierto de aquel paso es
   solo el orden del scoring de §E6.9 y la calibración de `capCorredor`.

**Un test hubo que rehacerlo, y la razón importa.** El de orientación girada (Etapa 4, punto 1) montaba su
escenario con una simulación real y un hueco "del tamaño exacto de la huella girada". Eso dejó de ser
construible desde fuera: **21 de las 32 celdas del hueco aparecían ocupadas por calles que el propio motor
decide poner ahí**. No es un fallo — es la calle costando suelo, que es el objetivo del rediseño. Pasó a ser
test UNITARIO sobre `sitiosPorAtraccionDura`, con `ocupadas` y `red` construidas a mano.

**`snapshot_baseline` re-baselineado, y el resultado es la mejor evidencia del cambio**: el diff son las MISMAS
3 líneas del Paso 1, en sentido INVERSO — la Vivienda vuelve a arrancar obra en su tick original. Es decir que
un cambio tan de fondo como que la calle pase a ocupar suelo deja la composición de la ciudad, en todos los
cortes de referencia, exactamente donde estaba antes de la Etapa 6. **Cambia cómo se ORDENA la ciudad, no
cuánto construye ni a qué ritmo.**


**Resultados medidos del Paso 2** (misma corrida de referencia que los pasos anteriores):

| Métrica | Paso 0 | Paso 1b | **Paso 2** | |
|---|---|---|---|---|
| `edificiosConFrenteRealPct` | 32.98 | 32.08 | **100** | objetivo alcanzado |
| `componentesDeRedMedia` | *(no medible)* | *(no medible)* | **1** | la red es UNA, se recorre entera a pie |
| `ocupacionNucleoPct` | 64.12 | 56.71 | **41.28** | **−23 puntos**: el coágulo se abrió |
| `manzanasCerradasMedia` | 3.89 | 3.50 | **4.83** | +24%, y ahora cuenta manzanas REALES |
| `dispersionViviendaCentro` | 3.44 | 3.13 | 3.72 | +0.6, ≈ el anillo de calle que ahora hay en medio |
| `dispersionPuestoMercado` | 2.38 | 2.27 | 2.75 | idem |
| vivos / colapsados | 18 / 2 | 18 / 2 | 18 / 2 | la simulación no se movió |
| `viviendasMedia` / `pesantsMedia` | 13 / 168.17 | 13.06 / 168.11 | 13.06 / 168.11 | idem |

Las dos métricas que la Etapa 6 existía para arreglar dieron **exactamente** en el objetivo: 100% de edificios
con salida real a la calle, y la red en UN solo componente. `ocupacionNucleoPct` baja 23 puntos: la ciudad
dejó de ser un coágulo, que era la decisión 4. Y `manzanasCerradasMedia` SUBE — el guardián de regresión de
las Etapas 1 y 2 no solo no empeoró, sino que ahora cuenta ciclos de una red que existe de verdad, no de una
red que era falsa en un 47%.

Las dos dispersiones suben ~0.5-0.6 celdas métricas. **No es deriva, es el anillo**: desde §E6.7 un satélite
mira a su ancla desde el otro lado de su calle, así que está literalmente una calle más lejos. Era la
consecuencia prevista de la decisión, no una regresión de la atracción.

**Coste, y una predicción mía que salió mal.** El Paso 1b cerró afirmando que "~43% de lo que queda es
maquinaria de aristas que el Paso 2 borra por construcción", y de ahí que el Paso 2 fuese *parte de la
solución*. **Falso**: la primera versión del Paso 2 subió de 3m48s a 7m18s. Sí desapareció la maquinaria de
aristas, pero el campo de distancia que la sustituye costaba más que ella — consultaba `ocupadas` y la red con
una clave de texto por celda de la caja y por vecina, unas 75.000 cadenas por barrido. Perfilado:
`tieneFrenteDeCalle` sola era el 16.5%, y el cluster de conexión el 34%.

Reescrito sobre MÁSCARAS INDEXADAS (`Uint8Array` sobre la caja, rellenada recorriendo los conjuntos y no la
caja; `costeDesde` recorre las cuatro franjas por aritmética de índices sin asignar nada), el perfil vuelve a
**19.3s, exactamente el nivel del Paso 1b**. Una segunda pasada sobre `corredorHastaLaRed` (claves numéricas
en vez de texto) lo bajó del 22% al 10% pero dejó el total plano — el coste se trasladó a los closures. Ahí se
paró: rendimientos decrecientes.

**Reloj final: 3m42s**, contra 3m48s del Paso 1b y 1m40s del Paso 0. **El modelo de celdas sale gratis: es
incluso marginalmente más rápido que el modelo de aristas que sustituye**, después de haber costado el doble
en su primera versión.

Lo que NO se recupera es el ×2.2 sobre la línea base del Paso 0: ese factor es el ×4 de posiciones que trae la
rejilla fina, parcialmente compensado por las dos pasadas de optimización. Bajar de ahí exige acotar el
barrido de `candidatosLibres` a anillos en vez de al disco entero — trabajo aparte, no de este paso.

#### Paso 4. Contrato de dibujo y `cliente/` — ✅ completada 2026-08-31 (adelantada: sin ella nada se ve)
- [x] `TrazadoAsentamiento.calles`/`caminos`: de `SegmentoTrazado[]` (líneas) a `RectanguloLocal[]` (ÁREAS),
      con las celdas fusionadas en tiradas horizontales antes de salir — una avenida de 20 celdas viaja como
      UN rectángulo, no como 20 (presupuesto de payload del doc 6)
- [x] `segmentosDeRed` → `rectangulosDeRed`. `SegmentoTrazado` desaparece; `RectanguloLocal` sirve igual para
      la huella de un edificio y para una tirada de calle
- [x] Derivado, sin tocar disco (§E6.11): esquemas y `ProyeccionJugador`/`EstadoAdmin` viajan por
      `GeometriaAsentamientos`, que ya se calculaba en `RunnerDePartida`
- [x] `cliente/src/ui/canvas.ts`: `dibujarTramos` → `dibujarTiradas`, que **rellena** en vez de trazar. Es
      justo lo que el modelo de aristas no podía representar: una línea no tiene ancho
- [ ] **Playtest en la interfaz — PENDIENTE.** No es opcional: los bugs de §5.7 (Mercado pegado al Centro
      Urbano) y de la Etapa 5 (anclas huérfanas) los encontró el usuario mirando la pantalla, no la suite

#### Paso 3. Orden del scoring y cap por clase — ⬜ pendiente (reducido: §E6.7 se lo llevó el Paso 2)
Separado del Paso 2 a propósito: son los cambios que mueven la DENSIDAD, y mezclarlos con el cambio de modelo
haría ilegible cuál movió qué. **Lo que este paso ya NO incluye**: el gap 0 → gap 1 de §E6.7, que resultó
inseparable del Paso 2 (ver allí) y aterrizó con él.
- [x] `sitiosPorAtraccionDura`: objetivo gap **1**, no 0 (§E6.7) — hecho en el Paso 2, por obligación
- [x] `esBordeDeManzana` pasa de "¿esta línea es borde?" a "¿esta columna/fila es de calle?", período
      `largoFila + anchoCalle` (§E6.8) — hecho en el Paso 2: sin él la red de celdas no se podía escribir
- [ ] Orden del scoring de §E6.9: gate duro → cercanía al ancla → **coste de calle** → retículo →
      fila/borde/semilla. Hoy el coste de calle solo actúa como GATE (`capCorredor`), no como término de
      desempate — falta esa mitad
- [ ] `capCorredor` por clase (§E6.10) está IMPLEMENTADO (`capCorredorUrbano` 12 / `capCorredorAfueras` 200)
      pero los dos números están puestos a ojo, sin calibrar
- [ ] Vigilar `manzanasCerradasMedia` y `dispersionViviendaCentro` contra el Paso 0 — son los guardianes de
      regresión de las Etapas 1 y 2, y este paso es el que puede romperlos

#### Paso 5. Calibración — ⬜ pendiente
- [ ] `capCorredor` urbano y de afueras, contra la línea base del Paso 0
- [ ] Reevaluar el pendiente "ancla creada sin garantizar sitio para al menos un satélite" (§"Abierto") bajo el
      criterio de gap 1, que cambia la pregunta
- [ ] Decidir si el retículo blando basta o hace falta endurecerlo (§"Abierto")

#### Antes de dar la Etapa 6 por cerrada
- [ ] Borrar `partidas/` (§E6.14) — los snapshots viejos tienen edificios colocados con reglas viejas
- [ ] Reescribir §1, §2, §3, §4, §5.2, §5.3, §6, §10, §11 y §12 al modelo de celdas y retirar el banner de
      "CERO código escrito" de la cabecera de la Etapa 6 y de §1

### E6.16 Hallazgo del laboratorio: edificios sobre celdas de calle (RESUELTO 2026-09-02)

Al recuperar el laboratorio visual (`lab/`, ver más abajo) y correr su ciudad —seed 1, 200 ticks— aparecen
**2 edificios pisando 6 celdas de calle**, violando el invariante de §E6.12 que la suite da por bueno. El test
`trazado.test.ts` no lo ve porque corre otra ciudad (seed 99) que no lo toca: **es exactamente el tipo de bug
que el laboratorio existe para encontrar y la suite no.**

Aislado a mano, replicando el replay incremental:

```
celda -4,-3  aparece como calle al procesar [orden  9] lenera
celda -5,-3  aparece como calle al procesar [orden 12] lenera
   ... y las pisa  [orden 15] almacen
celda 9,-8   aparece como calle al procesar [orden 33] patioDeGremios
   ... y las pisa  [orden 35] vivienda
```

Las calles existen ANTES, en el orden del array, que los edificios que las pisan. La colocación no las vio.

**Causa raíz, y NO la introdujo la Etapa 6.** `evaluarNecesidades` (`engine/construction.ts`) propone
candidatos uno a uno —cada uno consultando `sueloOcupado` sobre `[...edificiosBase, ...candidatos]`, es decir
en ORDEN DE PROPUESTA— pero los compromete ordenados por score:

```ts
for (const candidato of [...candidatos].sort((a, b) => b.score - a.score))   // construction.ts:809
```

Y además descarta por el camino los que no tienen cupo, materiales o pasan un tope. De modo que **el orden del
array final no es el orden en que cada candidato calculó su sitio**, y `redDeCalles` —que es un replay
dependiente del orden— produce entonces una red distinta de la que cada uno vio.

Con las calles sobre ARISTAS esta divergencia ya existía pero era invisible: una arista no ocupa superficie,
así que "solaparse" con ella no significaba nada. Al pasar la calle a costar suelo, el mismo desajuste se
vuelve un edificio construido encima de una calle.

**Cuatro salidas, ninguna gratis** (decisión pendiente del usuario, porque tres de ellas tocan balance):

1. **Comprometer en orden de propuesta** — una línea, pero el orden por score existe a propósito: es el que
   decide qué se paga primero cuando no hay cupo o materiales para todo (`Edificio.prioridad`).
2. **Proponer ya en orden de score** — exige evaluar la necesidad ANTES de calcular el sitio, o sea partir
   `evaluarNecesidades` en dos fases.
3. **Revalidar tras el commit** — recalcular la red y reubicar o descartar los candidatos que hayan quedado
   sobre una calle. Es el parche más contenido y no toca el orden de pago, pero añade una pasada.
4. **Hacer `redDeCalles` independiente del orden** — el replay incremental es lo que produce el crecimiento
   emergente (§2, §10); quitarlo sería rediseñar la mecánica entera. Descartada salvo sorpresa.

**Antes de arreglarlo hay que congelar el caso**: el fixture de `trazado.test.ts` (seed 99) no lo reproduce,
así que el arreglo no tendría guardián. La ciudad del laboratorio (seed 1, 200 ticks) sí — conviene añadirla
como segundo caso del test.

---

#### RESUELTO (2026-09-02) — salida 3, "revalidar antes de pagar"

**El guardián apareció solo.** Al duplicar la producción base de Granja (rebalanceo de trigo, Doc Game 4.2.1)
las ciudades crecen más, y `perfilesTrazado.test.ts` empezó a fallar con *"gremial: vivienda pisa la celda de
calle -7,-5"*. Comprobado revirtiendo el cambio: sin 2x pasa, con 2x falla. **El 2x no introdujo el bug, lo
hizo alcanzable por la suite** — que es justo lo que este apartado pedía antes de tocar nada.

**Qué se implementó**: `pisaCalleComprometida` (`engine/construction.ts`), llamada dentro del bucle de commit
**después** de las comprobaciones de cupo, tope y fondos, y **antes** de pagar. Recalcula `redDeCalles` sobre
`[...edificiosBase, ...yaComprometidos, candidato]` — es decir, sobre el orden REAL en que va a quedar la
ciudad — y si alguna celda del candidato cae sobre calle, lo descarta con un `continue`. Al ir después de las
otras comprobaciones, corre como mucho `NECESIDADES.maximoEnCola` (4) veces por asentamiento y tick, no una
por candidato propuesto.

**Por qué basta con mirar el prefijo ya comprometido y no hay que revalidar a los anteriores** (esto es lo que
hace viable la salida 3 y no estaba en el análisis original): `anadirConectadas` (`trazado.ts:433`) **nunca
siembra calle sobre una celda ya ocupada**, y el replay marca las celdas de cada edificio como ocupadas ANTES
de sembrar sus calles. Así que ningún edificio puede quedar bajo una calle nacida después de él. Validar cada
candidato contra su prefijo es suficiente **y el resultado es estable** — no hace falta iterar a punto fijo.

**Por qué no las otras salidas**: 1 y 2 cambian QUÉ se paga primero cuando no alcanza para todo, y eso es
balance (el orden por score existe a propósito); 4 sería rediseñar el crecimiento emergente entero.

**Coste medido: cero.** Batch de 15 facciones × 600 ticks, misma seed, antes y después del arreglo:
`vivos`, `colapsados`, `pesantsMedia`, `viviendasMedia`, `granjasActivasMedia`, `conGateNivel2Cumplido`,
`tropasVivas`, `manzanasCerradasMedia`, `edificiosConFrenteRealPct`, `componentesDeRedMedia` y
`ocupacionNucleoPct` salen **idénticos**. El rechazo es raro; cuando dispara, evita la violación sin frenar la
construcción.

**Guardián permanente**: `src/engine/__tests__/edificiosSobreCalle.test.ts`, con la ciudad del laboratorio
(**seed 1**, 200 ticks) que pedía este apartado, más las seeds 60 y 200. Verificado además a mano sobre 15
seeds antes de recortar: ninguna ciudad superviviente pisa calle. 786/786 tests, `tsc --noEmit` limpio.


### E6.17 Las afueras crecen hacia AFUERA (corregido 2026-08-31)

Reportado por el usuario jugando con el laboratorio: *"las granjas al subir de nivel están creciendo hacia el
centro de la ciudad ocupando espacio que era utilizado por otros edificios; estos deben crecer hacia afuera"*.

Medido antes de tocar nada, siguiendo el borde INTERIOR de cada Granja a lo largo de sus mejoras:

```
granja A  nivel 1→2→3→4   borde interior  88.2 → 65.8 → 57.9 → 39.0     (radioAfuerasMin = 60)
granja B  nivel 1→2→3→4   borde interior  88.2 → 60.1 → 70.0 → 64.9
```

A nivel 4 la Granja A tenía su borde a **39**: veintiún unidades DENTRO del radio vedado, comiéndose el suelo
del casco urbano. **Tres causas independientes**, todas anteriores a este reporte:

1. **El veto se medía contra el CENTRO, no contra la huella.** `distanciaMinima` filtraba
   `distancia(centroDelCandidato, origen)`. Una Granja de nivel 4 mide 36 unidades de lado: con su centro
   justo en 60, medio edificio quedaba dentro. Corregido con `distanciaBordeAlCentro` — *"las afueras empiezan
   en 60"* pasa a significar que ninguna CELDA del edificio entra de 60 para dentro.
2. **La reubicación no tenía dirección.** `reubicarPorTamano` elegía el hueco más cercano a donde estaba, y
   como `posicion` es el CENTRO, al crecer el rectángulo se expandía por igual hacia dentro y hacia fuera.
   Ahora descarta primero los huecos que acercarían el edificio al centro, y solo entre los que respetan eso
   elige el más cercano — para que la Granja siga junto a sus campos. Si NINGUNO evita acercarse, se muda
   igual: la mejora manda sobre la dirección.
3. **La banda de afueras no cabía.** `radioMaximoAfueras` topaba el CENTRO en
   `radioAfuerasMin + anchoBandaAfueras` = 96, y la banda mide 36 — exactamente el lado de una Granja de nivel
   4. Literalmente no había sitio para crecer sin retroceder. El tope ahora suma la media diagonal del
   edificio: la banda tiene que poder CONTENERLO, no solo a su punto medio.

**Y un cuarto fallo que estos destaparon, este sí introducido por el Paso 2**: el cap de corredor no se estaba
pasando por clase en `sitiosParaTipo`/`reubicarPorTamano`, así que a las afueras se les aplicaba
`capCorredorUrbano` = 12. Con el veto corregido empujando los candidatos más lejos, **ninguno** pasaba el gate
de conexión y la Granja inicial caía al fallback `(0,0)` — encima del Centro Urbano. §E6.10 avisaba
literalmente de esto (*"un cap único los rechazaría a todos"*) y aun así se coló: escribí la advertencia y
después cometí el error.

**Resultado medido, misma ciudad:**

| | antes | después |
|---|---|---|
| borde interior final de las Granjas | 39.0 / 64.9 | **96 / 96** |
| afueras con alguna celda dentro del radio vedado | varias | **0** |
| mejoras que acercan el edificio al centro | 8 de 9 | 3 de 9, y ninguna entra en la ciudad |

Las 3 que aún retroceden lo hacen dentro de las afueras (de 96 a 88) cuando no hay hueco exterior libre en ese
momento — es el fallback deliberado del punto 2.

`vista_asentamiento.test.ts` se actualizó al criterio nuevo: el veto se comprueba sobre la HUELLA (piso duro) y
por arriba no se exige un techo fijo —§11 dice que las afueras llegan *"al menos"* hasta la banda, y el techo
real sube con `radioPotencial` y con el tamaño del edificio—, solo que el edificio quepa entero en el espacio
local dibujable.

---

### E6.18 Regla de afinidad: los edificios se agrupan con los suyos (añadida 2026-08-31)

A petición del usuario, **un tercer criterio de desempate** en la colocación de satélites, encima de los dos
que ya había:

1. **1º — hueco al ancla** (`gapCeldas`): el anillo concéntrico más pegado al ancla que ofrezca un hueco
   conectado. Sin cambios.
2. **2º — lado compartido con el ANCLA** (`bordeCompartido` contra el ancla expandida por su anillo, §E6.7):
   entre huecos del mismo anillo, el que más borde pega al ancla. Regla original.
3. **3º — lado compartido con los AFINES** (`bordeAfinDe` — NUEVO): entre huecos igual de pegados al ancla, el
   que más lado comparte con edificios *afines* ya construidos.
4. **4º — semilla determinista** (`semillaCandidato`). Sin cambios; ahora se consulta menos.

**Afín** = mismo tipo exacto (Vivienda con Vivienda, Puesto de Mercado con Puesto de Mercado) **o** misma
categoría funcional de `CATEGORIA_POR_TIPO` (Fundición / Curtiduría / Armería, todas `industria`, se buscan
entre sí). Los tipos de ancla puros (Centro Urbano, Plaza de Armas, Patio de Gremios…) no están en esa tabla,
así que un satélite nunca sale "afín" a su propia ancla por esta vía — la adyacencia al ancla ya es el
criterio 2 y contarla otra vez la duplicaría. El ancla concreta a la que se atrae el satélite se excluye
explícitamente (`celdasDeTiposAfines(..., anclaInstancia.id)`).

Es un **desempate puro**: nunca convierte un hueco inválido en válido ni cambia qué se construye, solo cuál de
varios huecos equivalentes gana. La política "Líneas de Producción" (`ampliado`) lo ignora — esa reordena los
candidatos por distancia a sus insumos.

**Medido, 8 semillas × 250 ticks** (script A/B efímero, `bordeAfinDe` → `return 0` para el "antes"):

| | homogeneidad¹ | aristas mismo-tipo | aristas mismo-cat. | aristas mezcladas |
|---|---|---|---|---|
| antes | 80 % | 826 | 276 | 270 |
| después | **89 %** | 964 | 362 | **156** |

¹ `(mismo-tipo + mismo-categoría) / (todas las aristas edificio-edificio)`, media sobre las 8 ciudades.

Las aristas entre categorías distintas caen un 42 % y ninguna semilla empeora. El recuento de edificios queda
casi idéntico (una ciudad pasa de 68 a 66, el resto sin cambio): reordena, no altera el ritmo. Calibración
fina (¿pesar el término en vez de dejarlo lexicográfico?) queda para el pase de balance (§E6.15 Paso 5).

Cobertura: `anclasSatelites.test.ts` → *"Regla de afinidad (2026-08-31)"* (unidad de `tiposAfines` + un
escenario geométrico que prueba que el desempate elige el hueco que toca a los suyos).

---

### E6.19 Calibración por playtest del laboratorio (2026-08-31)

Valores fijados tras jugar con el laboratorio visual. Solo tocan `constants.ts`; ninguna lógica.

| Constante | Antes | Ahora |
|---|---|---|
| `TRAZADO.separacionSeguridadAnclas` | 4 | **6** (borde a borde) — deja sitio para una calle y una hilera de satélites entre dos anclas vecinas |
| `PUESTO_MERCADO_FORMA` 1 / 2 / 3 | 4×4 / 6×4 / 2×2 | **2×4 / 2×6 / 2×2** — tres piezas estrechas, mercadillo de puestos alargados |
| `MERCADO_PUESTOS_POR_NIVEL` (forma×cantidad, acumulativo) | 3 / 10 / 12 piezas | **6 / 13 / 17** — niv1 `1×2 2×2 3×1` · niv2 `1×2 2×2 3×3` · niv3 `1×1 2×2 3×1` |

Tests actualizados: `mercado_zona.test.ts` (conteo 6/13/17), `escalaRejilla.test.ts` (filas doradas de
`puestoMercado` formas 1 y 2 — un cambio de tamaño físico sí actualiza su fila, a diferencia de un cambio de
escala).

**Buena práctica añadida al laboratorio:** en la pestaña Parámetros, todo campo cuyo valor difiera del de
`constants.ts` al abrir la pestaña se pinta en **amarillo** (`input.lab-cambiado`) — así se localizan de un
vistazo los que se han tocado.

**Hallazgos del mismo playtest (seed 60, Mercado a mano en el tick 1, nivel 3 al tick 452):**

1. **Doble Patio de Gremios en el mismo tick, uno de ellos huérfano y sin calle (RESUELTO 2026-08-31, fix
   `b`).** Reproducido: seed 60, Mercado a mano en el tick 1, **tick 120** → aparecen DOS
   `patioDeGremios` a la vez (ids consecutivos `-37` y `-38`); `-37` nace `anclaLlena`, sin calle y sin
   satélites, `-38` conecta y recibe la Armería. Sin el Mercado manual, seed 60 crea un solo Patio en ese
   tick — el Mercado + sus 12 puestos (que también nacen en el tick 120) ocupan el núcleo y empujan la ranura
   de industria fuera del alcance del corredor.

   **Cadena exacta** (bucle de transformación de `evaluarNecesidades`, `construction.ts` ~L775):
   - iteración `curtiduria`: `asegurarAnclaPara` no encuentra Patio → `crearAnclaNueva` coloca **Patio -37**.
     `huecoEnDireccion` solo comprueba colisión + `separacionSeguridadAnclas`, **no** alcance a la red: -37
     cae a más de `capCorredorUrbano` (12) de cualquier calle → nace desconectado. En el replay de
     `redDeCalles`, `anadirConectadas(anillo)` se salta su anillo por no conectar.
   - `sitioEnBarrioLineaProduccion('curtiduria')` → `anclaMasCercana` elige -37 →
     `sitiosPorAtraccionDura(-37, …)` no tiene ningún candidato que alcance la red → `[]` → sin sitio, **sin
     `break`**.
   - iteración `armeria`: `asegurarAnclaPara` de nuevo. `anclaActivaParaCategoria` prueba -37 →
     `sitiosPorAtraccionDura` `[]` → marca **-37 `anclaLlena`** y, como no hay instancia usable, llama otra
     vez a `crearAnclaNueva` → **Patio -38**. Este conecta, la Armería se coloca, `break`.
   - Daño: un `patioDeGremios` permanente en la ciudad, `anclaLlena` para siempre (nunca se desmarca), que no
     hospeda nada. Se "auto-cura" la calle más tarde (tick ~197 la red crece hasta él) pero sigue `anclaLlena`.

   El caso benigno `1→2` en otros seeds (sin Mercado manual, ~tick 200+) NO es este bug: ahí el Patio #1 sí
   tiene 3-4 satélites activos, su núcleo está lleno de verdad y el #2 es expansión correcta.

   **Fix aplicado (opción b):** `asegurarAnclaPara` (`construction.ts`), tras `crearAnclaNueva`, comprueba con
   `sitiosPorAtraccionDura(nuevaAncla, tamaño-del-satélite, …)` que un satélite de `tipo` puede pegarse de
   verdad al ancla recién creada. Si no hay sitio, el ancla **no se commitea** —se devuelve `edificiosBase`
   sin ella (conservando las `semillaSaturada`, que sí son un hecho geométrico)— y la construcción se
   reintenta el tick siguiente, cuando la ciudad haya crecido. Así:
   - nunca queda un `patioDeGremios`/`plazaDeArmas` huérfano ocupando suelo y árbol de anclas;
   - la iteración siguiente del bucle vuelve a intentar `crearAnclaNueva` (mismo slot, determinista), vuelve a
     rechazarlo, y **no encadena** una segunda ancla — como mucho paga el coste de un `redDeCalles` extra
     mientras está atascado, cosa rara.

   Se descartó la opción `a` (chequeo de alcance a la red en `huecoEnDireccion`): un ancla legítima que nace
   en el radio máximo de ranura (`separacionMinimaAnclas × 3` = 36 celdas) queda ~29 celdas de la calle de su
   semilla, por encima de `capCorredorUrbano` (12), así que ese gate rechazaba anclas buenas. La opción `b`
   pregunta lo correcto directamente: *¿cabe el satélite?*, sin depender de un cap.

   Verificado: 30 corridas (15 seeds × con/sin Mercado manual, 320 ticks) — `huérfanos = 0` en todas,
   `industria = 9` sin regresión, seed 60+M ahora crea **un solo Patio conectado** en el tick 126 (6 ticks
   más tarde, esperando a que se abra un hueco de satélite). 691 tests.

2. **Talleres de Carpintería que no se crean (ABIERTO, sin diagnosticar).** Observado en el mismo playtest de
   seed 60; probablemente la misma familia (`crearTalleresDeCarpinteria` con la Carpintería como ancla propia).

3. **Los Puestos de Mercado salían siempre con la misma orientación (RESUELTO 2026-08-31).** `puestoMercado`
   salió de `TIPOS_SIN_ROTACION` — su `nivelInterno` identifica una FORMA *relativa*, no una orientación
   absoluta (el propio Mercado ya nace girado o no, `orientacionesDeAncla`), así que no había ninguna
   identidad que preservar. Ahora `sitiosPorAtraccionDura` ofrece las dos orientaciones y el desempate por
   `bordeCompartido` elige la que pega el lado LARGO al Mercado. `crearPuestosDeMercado` (construction.ts)
   tuvo que empezar a persistir `rotado` — antes lo ignoraba, inofensivo solo porque el puesto nunca giraba.
   Verificado en el lab: 12 puestos, 3 girados, 0 solapes.

---

### E6.20 El `posicion` del Centro Urbano pasa a ser su CENTRO (2026-08-31)

> **Nota (2026-09-13):** desde BA-005 el CU mide 4×4 (columnas y filas -2..1); se eligió par precisamente para
> conservar lo que decidió esta sección — ver §E6.24.

Hasta ahora el Centro Urbano era la ÚNICA excepción de coordenadas del sistema: su `posicion` `(0,0)` no era
su centro geométrico sino el VÉRTICE de una esquina (originalmente a petición del usuario). Efecto medido: con
6×6, su huella caía en las columnas 0..5 y filas -6..-1 — **todo el edificio en un solo cuadrante**, con el
origen pegado a una esquina. Y peor: `cu.posicion` valía `(0,0)` pero `centroDeRectangulo(rectanguloDeEdificio(cu))`
valía `(9,-9)` local — **dos "centros" distintos** según qué función preguntara. `semillaActiva` (más cercana
al origen) y `anclaMasCercana` medían contra `(0,0)`; `crearAnclaNueva` proyectaba sus 8 ranuras desde `(9,-9)`.
De ahí los "comportamientos algo extraños" que reportó el usuario.

**Cambio:** se quitó el caso especial de `celdaMinimaDeEdificio` (`engine/trazado.ts`). Ahora `(0,0)` es el
CENTRO del Centro Urbano, igual que en cualquier otro edificio. Con 6×6 ocupa las columnas -3..2 y las filas
-3..2, **simétrico alrededor del origen**, y `cu.posicion === centroDeRectangulo(rectanguloDeEdificio(cu))`.

Toca solo `celdaMinimaDeEdificio` (una rama menos). `settlement.ts` ya creaba el CU con `posicion: (0,0)` —
sigue igual, solo cambia qué significa. `cliente/` importa la función del motor, no tiene copia. Tests:
`escalaRejilla.test.ts` (fila dorada del CU, ahora con la misma huella de anclaje que el resto; se quitó su
`continue` de la prueba de inversa exacta). Suite verde (691). Partidas guardadas con el CU en `(0,0)` lo verán
desplazado ~4 celdas — pero ya hay que borrarlas por la Etapa 6 (§E6.14), así que no se migra.

---

### E6.21 El núcleo de un ancla es la BANDA DE UNA MANZANA, y "lleno" es geométrico (2026-08-31)

Reportado por el usuario con el laboratorio (seed 60, Mercado a mano en los primeros ticks): **el asentamiento
se quedaba clavado en nivel 1** — no podía construir las viviendas que le faltaban para llegar a 200 pesants
"porque se llena el ancla del Centro Urbano cuando todavía tiene espacio".

**Diagnóstico:** `sitiosPorAtraccionDura` daba el ancla por llena (`[]`) en cuanto no quedaba un hueco **con
frente de calle** (nivel 0/1), aunque la banda tuviera **130+ celdas libres** de segunda hilera (nivel 2/3).
`anclaActivaParaCategoria` marcaba el Centro Urbano `anclaLlena` —un latch permanente, §"Detalle D"— y a partir
de ahí ninguna vivienda más. El fix de §E6.19 lo agravaba: al intentar crear un `plaza`/`pozo`/`parque` de
relevo, su propio gate (`sitiosPorAtraccionDura` del ancla nueva) también veía "sin frente de calle" y lo
rechazaba. Resultado medido: seed 60 y 42 atascadas en nivel 1 con viv≈10 durante 300+ ticks; seed 60+M en
nivel 2 con viv=14 (necesita 40).

**Cambio en `sitiosPorAtraccionDura`:**

1. **El núcleo pasa a ser la BANDA DE UNA MANZANA** alrededor del ancla — el anillo de calle (§E6.7) más
   `FONDO_MANZANA` celdas de fondo (dos hileras de satélites espalda con espalda, lo más hondo posible sin
   traer otra calle). En términos de `hueco` (que se mide desde el ancla ya expandida por su anillo) el tope
   es `FONDO_MANZANA` a secas. Antes era `separacionMinimaAnclas / 2` (= 6), un número sin relación con la
   geometría de manzana.
2. **El ancla está LLENA solo cuando la banda no tiene ni un hueco geométrico libre.** Se aceptan los
   candidatos de nivel 2/3: todos ya alcanzan la red dentro de `capCorredorUrbano` (gate duro de
   `candidatosLibres`), así que `redDeCalles` les estira un corredor y el retículo cierra la manzana según la
   ciudad crece hacia ahí.
3. El bucle de anillos concéntricos (que con `FONDO_MANZANA` = 4 y tope 6 solo iteraba en 0 y 4, dejando
   muerta la franja hueco 5-6 — §"Detalle A") se sustituye por un único filtro de banda + orden:
   **`hueco` ↑ → `nivel` ↑ → `bordeCompartido` con el ancla ↓ → `bordeAfinDe` ↓ → semilla.**

**Resultado medido** (8 escenarios, 15 seeds × con/sin Mercado manual × 400 ticks): todas las semillas antes
atascadas llegan a **nivel 2 con viv = 40**; **0 edificios sin calle adyacente** (los nivel 2/3 se conectan
por corredor); sin regresión en las semillas sanas. Lab verificado: seed 60+M → nivel 2, 81 edificios, banda
del CU llena, ciudad compacta y coherente. 691 tests.

`separacionMinimaAnclas` deja de definir el núcleo (sigue mandando en la búsqueda de ranura del árbol,
`radioInicialRanura`/`radioMaximoRanura`). El latch de `anclaLlena` (§"Detalle D") sigue ahí — pero ahora se
dispara mucho más tarde y con relevo (`plaza`/`pozo`/`parque`) funcionando, así que deja de bloquear.

---

### E6.22 El árbol de anclas pasa de 8 ranuras cardinales a 5 equidistantes (2026-08-31)

A petición del usuario, tras ver en el laboratorio que "rara vez se crean las 8": las **8 direcciones**
(cardinales + intercardinales, 45°) de cada semilla del árbol pasan a **5 ranuras equidistantes a 72°**.

**Por qué:** con 8, casi la mitad de las ranuras de una semilla apuntaban de vuelta al centro ya construido
y fallaban `huecoEnDireccion` casi siempre, así que la semilla se descartaba (`semillaSaturada`) con la mitad
de sus ranuras sin estrenar y el árbol saltaba a otra. 5 a 72° reparten el crecimiento mejor sin dejar tantas
ranuras muertas.

**Cambio** (`engine/trazado.ts`):
- `NUM_RANURAS = 5`, `PASO_RANURA = 2π/5`, `ANGULO_RANURA_0 = -π/2` (la ranura 0 apunta hacia arriba —
  "partiendo del medio" del marco). Se borran `DIRECCIONES_CARDINALES` / `VECTOR_DIRECCION` / `ANGULO_DIRECCION`
  (el sistema de nombres N/NE/E/… ya no aporta nada).
- `direccionesRotadas` genera las 5 por aritmética de ángulo en vez de mapear los cardinales.
- `anguloRotacionEje`: rango `[0, PASO_RANURA)` (una ranura completa) en vez de `[0°, 45°)`.
- `ranuraOcupada`: el margen angular `EPS_ANGULO` pasa a derivarse de `MEDIA_RANURA_DIRECCION` (self-scaling).
- Códigos del árbol en el laboratorio: `a`–`e` (5 letras) en vez de `a`–`h`, dígitos `1`–`5`.

**Medido** (10 seeds × con/sin Mercado manual, 400 ticks): sin regresión — todas llegan a nivel 2 con
viv≈40, 3–5 anclas por asentamiento repartidas en ranuras distintas, `sinCalle≈0`. Lab verificado: seed 13 →
árbol CU + Patio de Gremios (ranura `a`) + Parque (ranura `e`), bien separados. 691 tests.

Experimento: si no convence, revertir es cambiar `NUM_RANURAS` de vuelta a 8 y restaurar el ángulo base
`ANGULO_RANURA_0 = 0` (o volver a los cardinales nombrados).

---

### E6.23 Perfiles de trazado: una política cambia la FORMA de la ciudad (2026-08-31)

A petición del usuario: *"eligiendo una política en específico, generar ciudades de formas diferentes pero
siendo orgánicas al mismo tiempo"*.

#### El principio

Hay dos formas de hacerlo y una mata lo orgánico:

- ❌ **Plantilla global** ("política X → traza una retícula / una estrella"). La forma deja de emerger y pasa a
  estar impuesta — es exactamente el plano pre-generado que se rechazó al empezar la mecánica.
- ✅ **Cambiar qué PREFIERE un edificio suelto** entre huecos igual de válidos. Las reglas siguen siendo
  locales, la forma sigue emergiendo, pero la estadística agregada cambia.

Lo segundo es lo que ya hacía `lineas_produccion` (§5.3, `ampliado`), que es el precedente: **el patrón
"una política sustituye el criterio de orden" ya estaba implementado y probado**. Y `postura_defensiva` llevaba
en el catálogo desde el principio marcada `// flag de layout, Doc 4.2 — sin efecto visual en Fase 0`, o sea:
el hueco estaba reservado.

#### La palanca

El desempate de `sitiosPorAtraccionDura` es **lexicográfico**, así que el primer término domina de forma
absoluta y los cuatro valores **ya se calculaban**. Un perfil es una PERMUTACIÓN de ese orden:

| Perfil | Manda | Silueta |
|---|---|---|
| `nucleos` | `hueco` | Racimos densos concéntricos por ancla — el orden histórico |
| `caminera` | `nivel` | Se encadena a las calles que ya existen |
| `compacta` | `centro` | Cada barrio llena primero su cara interior; ciudad más apretada |
| `gremial` | `bordeAfin` | Barrios monocromos, oficios segregados |

Permutación y no suma ponderada: los pesos habría que calibrarlos, se prestan a que un término se coma a otro
sin que se note, y destruyen la garantía de que un criterio se respeta SIEMPRE.

**Un quinto perfil se probó y se descartó:** «palatina», que hacía dominar `bordeCompartido` con el ancla.
Salió **idéntico a `nucleos`** — ese término solo tiene señal cuando el candidato toca el anillo del ancla
(hueco 0), así que como criterio dominante vale 0 en casi toda la banda y cae al siguiente. Se sustituyó por
`centro` (distancia al origen del asentamiento), el único término que mira la ciudad entera en vez de la
vecindad del ancla — por eso es el que produce una silueta global distinta. `bordeCompartido` sigue en el
desempate de los cuatro perfiles, donde sí sirve, pero no puede encabezar ninguno.

#### De dónde sale el perfil

`resolverPerfil(id, porPolitica)` — precedencia en un solo sitio:

```
TRAZADO.perfilForzado (laboratorio / BATCH_PERFIL)  >  política activa  >  tradición local
```

- **Tradición local** (`perfilPorTradicion`): derivada del id del asentamiento, determinista y permanente.
  Mismo mecanismo que `anguloRotacionEje` y `largoMaxFila`. Es lo que hace que **dos ciudades NPC no salgan
  iguales aunque nadie active ninguna política** — el problema de fondo se arregla también sin jugador.
- **Política**: cuatro ordenanzas del Maestro de Obras (`postura_defensiva` → compacta, `arterias_comerciales`
  → caminera, `barrios_gremiales` → gremial, `plazas_mayores` → nucleos). **Excluyentes sin ninguna regla
  nueva**: `maestroObras` tiene un único slot, así que activar una obliga a esperar a que expire la anterior.
  Compiten en ese mismo slot con Vía Rápida y Líneas de Producción — forma contra velocidad contra logística.

`trazado.ts` no sabe nada de políticas (sigue siendo geometría pura): la decisión *"qué perfil toca"* vive en
`construction.ts::perfilDe`, la de *"qué hace ese perfil"* en `ORDEN_POR_PERFIL`.

#### Estratos, no reformas

Una política dura `duracionMinutosPorDefecto` = **150 ticks** (1 tick = 1 minuto) y nada mueve lo ya
construido. Así que una ordenanza de trazado **no produce una ciudad de esa forma — produce un ESTRATO**: un
núcleo gremial con un anillo compacto encima, etc. La ciudad acaba registrando su historia política en su
geometría. Es deliberado y creo que es mejor que el efecto puro, pero implica que nunca se verá una ciudad
"100 % caminera".

#### Medido

Batch, 12 facciones × 400 ticks, mismas semillas (`BATCH_PERFIL=<perfil>`):

| perfil | frente real | componentes de red | ocupación núcleo | manzanas cerradas |
|---|---|---|---|---|
| `nucleos` | 99.7 % | 1 | 43.0 % | 4.45 |
| `caminera` | 100 % | 1 | 41.9 % | 4.55 |
| `compacta` | 100 % | 1 | **49.4 %** | **5.55** |
| `gremial` | 100 % | 1 | 41.7 % | 5.27 |

- **Ningún perfil rompe nada**: frente real ~100 % y red de UN solo componente en los cuatro.
- `compacta` es medible: +15 % de densidad y +25 % de manzanas cerradas.
- `gremial` cierra más manzanas a la misma densidad (la segregación aprieta los bloques). En medición aparte,
  homogeneidad de vecindad 86-92 % contra 68-76 % de base.
- **`caminera` es el más flojo**: apenas se separa en estas métricas. La causa es la misma que mató a
  «palatina», más suave: `nivel` vale 0 o 1 para casi todos los candidatos de una banda sana, así que
  discrimina poco. Es distinto (ciudad distinta, test lo congela) pero el efecto es sutil — candidato a
  reforzarse o a sustituirse.

Cobertura: `perfilesTrazado.test.ts` — cada perfil da una ciudad distinta (lo que impide que vuelva a colarse
un perfil decorativo), ninguno rompe invariantes, ninguno altera el CENSO de edificios (la forma no puede ser
una ventaja económica encubierta), precedencia, y exclusividad por slot.

#### Pendiente

**Ninguna ordenanza tiene coste/beneficio mecánico propio**, así que hoy compiten en desventaja contra Vía
Rápida (−25 % de tiempo de obra) y son, en la práctica, una elección estética. Una política de forma que solo
cambia la forma es cosmética y nadie la elegirá: **la forma tiene que ser la consecuencia de un trade-off, no
el trade-off**. `barrios_gremiales` es la que más cerca está de tener uno solo — agrupar industria acorta la
distancia a los insumos, que `factorLineaProduccion` ya mide y ya premia. Sin calibrar ni medir.

### E6.24 BA-005: huellas a la mitad, calles igual (2026-09-13)

Origen: propuesta de Codex `Docs/Coordinacion/propuestas/BA-005_huellas_y_escala_asentamiento.md`, tras el
playtest del usuario en el SettlementPreview de Unity. El ancho de calle (una celda = 3 unidades locales) está
validado para combate; los edificios ocupaban demasiado frente a él.

**Decisiones del usuario:**

- Todas las huellas ÷2 en ancho y fondo. Ninguna dimensión impar: todas exactas. Vuelven a ser las cifras de
  la rejilla original (§6), pero sobre la celda de 3 — deshacen el ×2 de §E6.11 sin deshacer la celda.
- **Centro Urbano 4×4, no 3×3.** Con lados impares su centro no puede caer en `(0,0)`: `celdaMinimaDeEdificio`
  redondea `-1.5` a `-1` y la huella queda media celda al lado de su `posicion`. Se descartaron desplazar la
  rejilla media celda (toca todas las conversiones celda↔local) y mover el CU a `(1.5, 1.5)` (rompe §E6.20 y el
  "origen = centro del CU" del contrato con Unity).
- La celda de muralla NO se divide: comparte escala con la calle y ya es correcta.

**Constantes (criterio de Claude, a petición del usuario):** las de escala de EDIFICIO se parten con las
huellas; las de escala de CALLE no se tocan.

| Constante | Antes | BA-005 | Escala |
|---|---|---|---|
| `EDIFICIO_TAMANO.*`, `granja.niveles[n].tamano`, `PUESTO_MERCADO_FORMA`, `EDIFICIO_TAMANO_POR_DEFECTO` | Etapa 6 | **÷2** (CU 6×6 → 4×4) | edificio |
| `FONDO_MANZANA` | 4 | **2** | edificio — dos hileras de Vivienda |
| `TRAZADO.largoFilaMin` / `largoFilaMax` | 8 / 16 | **4 / 8** | edificio |
| `TRAZADO.separacionMinimaAnclas` | 12 | **6** | edificio (las ranuras derivan solas) |
| `TRAZADO.separacionSeguridadAnclas` | 6 | **3** | edificio |
| `REJILLA_ASENTAMIENTO.tamanoCelda`, `TRAZADO.anchoCalle` | 3 / 1 | sin cambio | calle |
| `TRAZADO.capCorredorUrbano` / `capCorredorAfueras` | 12 / 200 | sin cambio | calle — palanca si la ciudad sale esponjosa |
| `MURALLA.*` | — | sin cambio | calle |
| `radioAfuerasMin` / `anchoBandaAfueras` / `radioMapa` | 60 / 36 / 220 | sin cambio | unidades locales |

**Partidas guardadas.** A diferencia de §E6.11, esto SÍ mueve la geometría de lo persistido: una Vivienda 2×2
guardada, leída como 1×1, cae media celda al lado. Sin migración (regla general del usuario): nueva
`LAYOUT_VERSION` (`constants.ts`) en el snapshot, y `cargarPartida` rechaza una distinta igual que
`worldgenVersion`. `BALANCE_VERSION` no servía: solo es registro, no rechaza.

**Tests.** `escalaRejilla.test.ts` deja de congelar la identidad de §E6.11 (ya no aplica) y congela la tabla
BA-005 (27 tipos, 32 combinaciones), el CU centrado en el origen y la inversa exacta posición↔celda.

---

## 1. Geometría de las calles

> **DESACTUALIZADO desde 2026-08-31.** Esta sección y las siguientes describen el modelo de ARISTAS, que YA NO
> es el que corre: la Etapa 6 (Paso 2) lo sustituyó por calles sobre CELDAS. La referencia válida está en esa
> sección del log de arriba; estas se reescriben cuando la Etapa 6 se cierre del todo.

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
`separacionSeguridadAnclas` (fijo; hoy 3 celdas — 6 tras §E6.19, partido por dos en §E6.24) — ahora es el ÚNICO
criterio de rechazo duro para una ranura nueva.

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
| Centro Urbano | 4x4 (3x3 hasta BA-005, §E6.24) | origen — primer ancla |
| Plaza | 2x2 | ancla residencial adicional (1 de 3, sorteada) |
| Pozo | 1x1 | ancla residencial adicional (1 de 3, sorteada) |
| Parque | 3x2 | ancla residencial adicional (1 de 3, sorteada) |
| Plaza de Armas | 2x2 | ancla militar |
| Patio de Gremios | 2x2 | ancla de industria |
| Carpintería (pieza principal) | 4x2 | satélite de Plaza de Armas |
| Fundición | 2x2 | satélite de Patio de Gremios |
| Curtiduría | 2x2 | satélite de Patio de Gremios |
| Armería | 2x3 | satélite de Patio de Gremios |
| Barracón | 2x2 | satélite de Plaza de Armas |
| Galería de tiro | 2x4 | satélite de Plaza de Armas |
| Vivienda | 1x1 | satélite de Centro Urbano / Plaza |
| Leñera | 1x1 | regla genérica |
| Almacén | 2x1 | regla genérica |
| Mercado (pieza principal) | 3x2 | ancla de mercado |
| Puesto de mercado | 1x2, 1x3 o 1x1 según la pieza | satélite de Mercado |
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

## 9. Carpintería: zona de tres piezas (REVERTIDO 2026-09-12)

**Esta sección describe una mecánica que ya no existe en el código.** Una corrida en batch del laboratorio
(`scripts/lab-batch-trazado.ts`) sobre muchas seeds encontró que la Carpintería era, con enorme diferencia, el
tipo que más generaba anclas huérfanas y anclas sin padre reconocible en el árbol único — el doble rol de la
Carpintería (satélite de la Plaza de Armas para su PROPIA colocación, pero ancla primaria de una categoría
`carpinteria` aparte solo para sus talleres) es justo el caso que `sitiosParaTipo`/`construirArbol` no manejaban
bien. Coincide con el bug ya anotado como abierto en el punto 2 de "Abierto" (talleres que no se crean, seed 60).
A petición del usuario se eliminó el rol de ancla, `tallerCarpinteria` y toda esta lógica especial: la
Carpintería vuelve a ser un edificio de transformación normal (categoría `militar`, satélite de Plaza de Armas,
sin piezas dependientes) — el resto de esta sección queda como registro histórico de la decisión revertida.

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
- **Etapa 6 — `capCorredor` sin calibrar, y son dos números.** El cap urbano (§E6.10) es lo único que ejerce la
  presión anti-coágulo una vez que la cercanía al ancla sigue mandando (decisión 5): demasiado alto y la ciudad
  vuelve al coágulo medido en §E6.1; demasiado bajo y las colocaciones empiezan a rechazarse. El de afueras casi
  con seguridad debe ser nulo, pero tampoco está probado.
- **Etapa 6 — ¿basta el retículo blando para que las manzanas emerjan?** La decisión 2 apuesta a que una
  preferencia fuerte converge a rejilla sin reservar celdas. Es exactamente la hipótesis que hay que medir con
  `manzanasCerradasMedia` una vez que la métrica cuente solo calles de ancho real. Si no converge, el
  siguiente escalón es el retículo duro, que ya se evaluó y se descartó por chocar con el principio rector.
- **Etapa 6 — el ×4 de CPU no está medido.** `candidatosLibres` barre el cuadrado del radio en celdas (§E6.11).
  El laboratorio batch es infraestructura crítica (doc 10 §5) y no puede volverse impracticable: hay que medir
  antes de comprometerse, y el campo de distancia de §E6.6 debería compensar parte del coste.
- **Etapa 6 — jerarquía de calles aplazada, no descartada.** La decisión 1 fijó 1 celda uniforme. Las celdas
  hacen posible por primera vez distinguir callejón / avenida (algo que las aristas nunca pudieron expresar);
  queda como ampliación natural si la ciudad en 3D se lee plana.
- **Etapa 6 vs. "ancla creada sin garantizar sitio para un satélite"** (punto de más arriba): el cambio de
  gap 0 a gap 1 (§E6.7) mueve el criterio de "¿tendrá sitio?", así que ese pendiente hay que re-evaluarlo con
  el modelo nuevo en vez de arrastrarlo tal cual.
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
