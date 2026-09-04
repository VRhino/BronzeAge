# Niebla de guerra ("último conocido") — cierre de diseño y plan de ejecución

> **Estado (2026-09-04): diseño cerrado y REVISADO (§2.4-2.6) tras contrastarlo con el resultado que el
> usuario quiere ver en pantalla.** Mismo criterio que
> `Murallas_Definicion.md` y `Movimiento_Ejercitos_Definicion.md`: la especificación se escribe ANTES de tocar
> el motor.
>
> **Las REGLAS DE JUEGO viven en `Docs/Game/`**, no aquí. Este documento cubre el registro de decisiones, la
> representación en el motor, el orden de ejecución y qué se congela en tests. El enunciado original es
> `Docs/Mecanicas a desarrollar.md` §12.

## 1. De dónde viene, y qué había ya

Sale del plan de arquitectura, donde era "C4 Slice 2". La infraestructura estaba hecha desde hace tiempo: la
proyección por audiencia (`proyectarParaJugador`) ya filtra el estado a lo que un jugador puede ver. Lo que
faltaba era **decidir el juego**, y en concreto un número: el radio de visión. Sin él, el filtro espacial no
se podía escribir.

**El movimiento de ejércitos desbloqueó ese número** y de paso implementó la primera mitad: desde el 2026-09-03
un jugador ve los ejércitos ajenos que entran en su zona de influencia o en el radio de visión de uno de los
suyos (`ejercitosAvistados`, Doc 5.12.7), redactados a posición, Facción y nº de participantes.

Así que esto ya no es una mecánica desde cero: es **completar** lo que quedó a medias.

| Pieza | Estado antes de esta mecánica |
|---|---|
| Proyección por audiencia | Hecha (Fase C4) |
| Ver ejércitos ajenos por espacio | **Hecho** (Doc 5.12.7) |
| Ver asentamientos ajenos por espacio | **Hecho** (Paso 1, 2026-09-04) |
| Memoria ("último conocido") | **Hecha** (Pasos 2-3) |
| Terreno tapado donde nunca se estuvo | **Hecho** (Paso 5, en el cliente de jugador) |
| Visión compartida por alianza | Falta |

## 2. Decisiones cerradas con el usuario (2026-09-04)

### 2.1 Un asentamiento ve su zona de influencia MÁS un margen

Decisión: **zona + un radio extra**. La plaza vigila algo más allá de su frontera, como una atalaya.

El margen es **60** unidades de mapa. Tres razones para ese número y no otro:

- **Menos que la vista de un ejército** (150), que es lo que mantiene el valor de explorar: una columna en
  marcha divisa una plaza mucho antes de que la plaza la divise a ella, y conserva la iniciativa.
- **Duplica el área vigilada de una plaza recién fundada**, cuya zona es exactamente 60. El margen se nota
  desde el primer minuto en vez de ser un detalle que solo importa tarde.
- Es la misma distancia a la que un ejército puede repostar en una plaza (`radioReabastecimiento`), lo que da
  una coincidencia útil de leer: **si una columna está lo bastante cerca como para repostar en tu ciudad,
  tu ciudad la ve.**

> **Consecuencia que conviene tener presente**: como la zona crece con el nivel (60 → 180), un asentamiento de
> nivel 5 vigila 240 y por tanto **ve más lejos que un ejército**. Es coherente con la ficción —una capital
> tiene atalayas, un ejército tiene ojos— y con premiar el crecimiento, pero significa que la ventaja de
> explorar se diluye alrededor de las grandes ciudades. Anotado para calibración, no para bloquear.

### 2.2 De un asentamiento rival se sabe su FICHA, no su interior

Decisión: **nombre, Facción, posición y nivel.** Nada más.

Es la misma filosofía de redacción que ya se aplica a los ejércitos ajenos: quién es y dónde está, nunca su
interior. Quedan fuera a propósito el almacén, la guarnición, las colas de construcción y los edificios — eso
es telemetría de un rival, y es justo lo que la proyección existe para impedir.

El **nivel sí entra** porque es visible desde fuera en la ficción: una ciudad grande se ve grande. No dice
cuánta tropa tiene dentro, que es lo que decidiría un ataque.

### 2.3 El conocimiento no caduca, y se refresca cada vez que se vuelve a ver

Decisión: **no caduca; cada vez que vuelves a verlo, la foto se actualiza.**

No hace falta elegir un plazo de caducidad —que sería una constante más sin medir— porque **el dato viejo ya
se delata solo**: la foto viaja con el instante en que se tomó, y la interfaz puede decir "última información:
hace 3 horas". El jugador juzga si fiarse.

Borrar información que el jugador ya sabe se siente arbitrario; dejarla envejecer a la vista, no.

### 2.4 VER es conocer — corrección al plan original (2026-09-04)

El usuario describió el resultado que quiere en tres estados, y al contrastarlo apareció un error de fondo en
la primera versión de este documento:

> 1. **No lo veo y nunca lo he visto**: tapado.
> 2. **No lo veo pero lo vi antes**: se guarda lo último que vi, con un filtro oscuro.
> 3. **Lo estoy viendo**: tal cual. Al dejar de verlo, cae en el estado anterior.

La primera versión solo guardaba memoria del **contacto comercial**. Con eso, ver una ciudad rival con un
ejército y alejarse la hacía **desaparecer del todo** en vez de caer al estado 2 — justo lo contrario del
punto 3.

La corrección deja el modelo **más simple**, no más complicado:

> **Ver algo ES conocerlo.** No hay tres fuentes de conocimiento, hay una regla: lo que esté a tu alcance
> este tick queda grabado. Comerciar no es una vía aparte — es otra forma de "ver" ese asentamiento un
> instante.

Las tres fuentes que arrastraba el documento de arquitectura (espacial, contacto, alianza) colapsan así en un
solo mecanismo, y el contacto pasa a ser un caso particular del mismo.

### 2.5 El terreno también se tapa, y de eso se encarga el CLIENTE DE JUGADOR

Decisión del usuario: **el estado 1 tapa también el terreno**, no solo lo que hay encima. Sin eso, "nunca
visto" y "visto pero vacío" se ven igual y los tres estados no se distinguen.

Pero el enmascarado lo hace el **cliente de jugador**, no el servidor:

- **El terreno no es información táctica.** La geografía es la misma para todos y el cliente ya la descarga
  una vez y la cachea para siempre por su `mapaId` (Fase C11, 125 KB idénticos byte a byte). Ocultarla en el
  servidor rompería esa caché a cambio de nada: lo que no puede salir del servidor son las ENTIDADES, y eso
  ya se filtra.
- **El cliente de administración lo ve todo**, sin máscara. Es una herramienta de operación, no un jugador.

Así que el servidor solo aporta **qué has explorado**; cómo se pinta es cosa de cada cliente.

### 2.6 La exploración es de la FACCIÓN, no del jugador

Decisión tomada al diseñar el almacenamiento: lo explorado y lo conocido se guardan **por Facción**.

Dos razones, y la segunda es la que manda:

- **Coherencia**: la proyección ya enseña la Facción propia COMPLETA a cualquiera de sus ciudadanos. Que un
  miembro no supiera lo que otro ya exploró sería incoherente con todo lo demás.
- **Coste**: en el laboratorio hay 30 Facciones y ~150 jugadores. Guardarlo por jugador multiplica por cinco
  un dato que se escribe en cada snapshot.

### 2.8 Lo que hay en el MUNDO tampoco es público (2026-09-04)

Dos colecciones se proyectaban a todo el mundo sin filtrar, con el argumento de que no son de ninguna
Facción: los **caminos comerciales** y los **campamentos de bandidos**. Era defendible antes de que existiera
la niebla; con ella deja de serlo. Que algo no pertenezca a nadie no lo hace público — un campamento en un
bosque que nadie ha pisado, o una calzada entre dos ciudades al otro lado del mundo, es información que el
jugador no ha ido a buscar. Y `caminos` era además la peor de las dos: cada camino une **dos plazas
identificadas**, así que la lista completa es el mapa de quién comercia con quién.

Cada colección se filtra con la regla que le toca, y no es la misma:

| Colección | Regla | Por qué |
|---|---|---|
| **Caminos** | **Explorado** | Es infraestructura estática, como el terreno: la calzada que recorriste sigue donde estaba aunque hoy no la mires |
| **Campamentos de bandidos** | **Visible ahora**, sin memoria | Aparecen y desaparecen, y sobre algo que va y viene "explorado" miente en las dos direcciones |

**Por qué los campamentos no tienen memoria.** Con "explorado" se enseñaría el campamento que nació la semana
pasada en un bosque que la Facción visitó una vez —información que nadie fue a buscar, que es exactamente la
fuga que se está cerrando— y se seguiría enseñando el que otra Facción ya arrasó. Se consideró darles memoria
propia, como a una plaza: no se mueven, y "aquí había bandidos, hace tres horas" sería una ficha razonable.
Se descartó porque exigiría grabarlos en `memoriaPorFaccion` con su `conocidoEn` y su tránsito del estado 3 al
2, y de eso no hay una sola regla en el canon. Sin memoria se comportan como `ejercitosAvistados`, que ya es
un precedente del propio juego.

**Un camino viaja entero o no viaja.** Recortar el trazado a los tramos explorados no daría "medio camino":
daría una polilínea con agujeros que el cliente uniría con rectas falsas, o trozos sin identidad propia (el
`id` y los dos extremos son del camino, no de cada tramo). Lo que se acepta a cambio es que haber andado un
tramo revele qué dos plazas une — que es, en la ficción, justo lo que una calzada dice de sí misma.

**Y esto cambia el orden de capas del cliente**, que no es estética (§5.4): los caminos siguen BAJO la
máscara, porque lo explorado-y-no-visto se pinta a media luz; los campamentos pasan a pintarse ENCIMA, junto
a lo que se está viendo ahora mismo. De qué lado cae cada cosa lo decide la regla con que el servidor la
filtra, no su aspecto.

### 2.7 Lo que ya estaba decidido y no se toca

- **Se muestra el ÚLTIMO ESTADO CONOCIDO, no el actual** (2026-08-24, doc de arquitectura 6 §3). Es el patrón
  de los RTS y lo que impide que un solo trueque dé telemetría en vivo para siempre.
- **Un aliado ve lo que ves tú, EN VIVO** — no "último conocido". La alianza es cooperación explícita.

## 3. El modelo que sale de esas decisiones

Una sola regla —**lo que alcanzas a ver este tick queda grabado**— produce los tres estados del jugador:

| Estado | Cómo se produce | Qué se envía | Cómo lo pinta el cliente de jugador |
|---|---|---|---|
| **1. Nunca visto** | La celda no está en lo explorado de tu Facción | Nada | Terreno tapado |
| **2. Visto antes** | Explorado, pero fuera de alcance ahora | La FICHA congelada + `conocidoEn` | Terreno visible, entidades con filtro oscuro |
| **3. Viéndolo** | Dentro de zona+margen, de la vista de un ejército tuyo, o de la de un aliado | La ficha EN VIVO | Tal cual |

Y el tránsito que pedía el usuario sale solo: **al dejar de ver algo, deja de estar en la lista de en vivo y
se queda la última foto** — que es exactamente "cae en la categoría anterior".

Cuando las dos coinciden gana la de en vivo: estar mirándolo es mejor información que recordarlo.

## 4. Representación en el motor

### Lo que se guarda, y lo que NO

Se guarda **solo la memoria**: lo explorado y la última foto de cada asentamiento conocido, por Facción. Lo
que se ve AHORA no se guarda — se deriva en cada proyección de dónde están tus asentamientos y tus ejércitos,
exactamente como ya se hace con `ejercitosAvistados`. Guardar lo derivable es la clase de error que este
proyecto lleva evitando desde el principio.

```
MemoriaFaccion {
  exploracion: <celdas exploradas>                    // el estado 1 contra el 2
  asentamientos: Record<asentamientoId, FichaConocida>  // el estado 2
}

FichaConocida {
  asentamientoId, nombre, faccionId, posicion, nivel
  conocidoEn: Instante        // cuándo se tomó la foto
}
```

Vive en `GameSessionState` como `memoriaPorFaccion`. Una Facción sin registro no ha explorado nada — igual
que un jugador sin registro usa el Liderazgo base, así que **las partidas guardadas no necesitan migración**.

### La exploración: una rejilla gruesa

Lo explorado no puede ser una lista de puntos: es un ÁREA. Se discretiza en celdas de
`EXPLORACION.tamanoCelda` (25 unidades de mapa → 80×80 = 6.400 celdas sobre el mundo de 2000).

El tamaño es un compromiso explícito: con celdas de 25, el margen de visión de una plaza (120) son ~5 celdas
de radio — bastante para que la frontera de la niebla se lea como una forma y no como un cuadrado — y el
coste por Facción se queda en **6.400 bits, o sea 800 bytes**. Treinta Facciones son 24 KB en el snapshot,
frente a los 125 KB que ya ocupa el mapa.

Se guarda como bitmap y no como lista de celdas por eso mismo: una lista de 6.400 pares `"col,row"` son ~45 KB
por Facción, doscientas veces más para el mismo dato. La opacidad que eso introduce se acota en un módulo
propio con sus helpers (`marcarExplorado`, `estaExplorado`) y sus tests.

### En la proyección

`ProyeccionJugador` gana tres campos:

```
asentamientosAvistados: FichaAsentamiento[]   // estado 3: en vivo
asentamientosConocidos: FichaConocida[]       // estado 2: congelados, con `conocidoEn`
exploracion: <celdas exploradas>              // estado 1 contra 2: lo que el cliente usa para tapar
```

Dos arrays y no uno, por el mismo motivo que se separaron `ejercitos` y `ejercitosAvistados`: **la diferencia
entre "lo estoy viendo" y "lo recuerdo" es de tipo, no de un campo opcional que el cliente pueda olvidarse de
mirar.** Y solo el segundo lleva `conocidoEn`, porque solo el segundo puede estar viejo.

## 5. Plan de ejecución

- [x] **Paso 1 — Visión espacial de asentamientos ajenos** (estado 3). **HECHO (2026-09-04).**
      `AsentamientoAvistado` + `asentamientosAvistados` en `proyectarParaJugador`, sobre el mismo `seVeAhora`
      que ya servía a los ejércitos. Ver §5.1 para lo que salió por el camino.
- [x] **Paso 2 — `memoriaPorFaccion` en el estado**. **HECHO (2026-09-04).** `engine/exploracion.ts` (la
      rejilla) y `engine/memoria.ts` (la regla), grabando al final del tick con los ejércitos ya movidos.
      Migración de snapshot v6 -> v7. Todavía no se proyecta: solo se acumula. Ver §5.2.
- [x] **Paso 3 — Proyectar la memoria** (estados 1 y 2). **HECHO (2026-09-04).** `exploracion` y
      `asentamientosConocidos` en `ProyeccionJugador`, con lo visto en vivo ganando a lo recordado. Ver §5.3.
- [ ] **Paso 4 — Visión compartida por alianza.** En vivo, y solo mientras la alianza esté activa: al
      romperse, lo que se veía por ella pasa a ser recuerdo (con la fecha de la ruptura) o desaparece —
      **decidir al llegar**, no antes.
- [x] **Paso 5 — Render en el cliente de JUGADOR.** **HECHO (2026-09-04).** Tapa el terreno no explorado y
      pinta lo recordado con filtro oscuro. El de ADMINISTRACIÓN no se toca: no tapa nada, es herramienta de
      operación y no un jugador. Ver §5.4.
- [ ] **Paso 6 — Calibración** del margen (2.1) contra la vista de ejército, con la nota del nivel 5, y del
      tamaño de celda contra cómo se ve la frontera de la niebla.

### 5.1 Lo que salió del Paso 1

Dos cosas que el plan no anticipaba, ambas de simplificación:

**Los dos radios de visión ahora viven juntos, en `VISION`.** `radioVisionEjercito` estaba dentro de
`LOGISTICA`, que es el módulo del avituallamiento de una columna — no su casa. Y el margen del asentamiento
tampoco cabía ahí: la atalaya de una ciudad no es logística. Se sacaron los dos a un grupo propio
(`VISION.ejercito` = 150, `VISION.margenAsentamiento` = 60) porque **la relación entre ellos ES la regla de
juego**: que la columna vea más lejos que la plaza es lo que mantiene el valor de explorar, y eso solo se lee
si los dos números están a la vista uno al lado del otro.

**La vista dejó de depender del polígono de la zona, y pasó a medirse contra el disco de la plaza.** Antes
`seVeAhora` hacía `pointInPolygon` contra la zona de influencia, que viene **recortada por las fronteras con
Facciones rivales** (`computeZonaInfluencia`). Ese recorte es POLÍTICO, no óptico: que un rival plante su
frontera pegada a tu ciudad no puede cegar a tus vigías — si acaso lo contrario. Ahora se mide contra
`radioPotencial + margen`, y como el polígono siempre está contenido en el disco, el cambio solo ensancha la
vista, nunca la recorta.

De rebote, **la visibilidad ya no lee el parámetro `geometria` en absoluto**, que era una entrada
privilegiada (necesita la posición de todos los asentamientos del mundo). Hay un test que lo congela: inyectar
un polígono enorme, propio o ajeno, ya no concede ni un metro de visión.

### 5.2 Lo que salió del Paso 2

**Las dos mitades de la memoria se hicieron a la vez**, en contra del plan, que separaba la rejilla (Paso 2) de
la ficha (Paso 3). Con "ver es conocer" las dos se graban en la MISMA pasada y con los MISMOS ojos: partirlas
habría significado recorrer dos veces la misma lista para escribir en el mismo sitio. El Paso 3 se queda con
lo que de verdad le corresponde, que es proyectar.

**Hexadecimal en vez de base64.** El plan decía base64 (800 bytes por Facción); son 1.600 caracteres hex, el
doble de texto pero **cero dependencias**: ni `Buffer` (que no aparece en ninguna otra parte de `src/`) ni
`btoa`. El resto del motor es aritmética pura y este módulo también lo es. De propina, un volcado hex se
sigue leyendo a ojo en un snapshot.

**La celda que pisa el ojo se marca siempre.** La regla general es "celda vista = celda cuyo centro entra en
el círculo", que es conservadora en la dirección correcta. Sola, dejaba a un ojo de radio pequeño pegado al
borde de su celda sin marcar ni el suelo que pisaba — el único caso en que la regla del centro da un
resultado absurdo. Hay un test que lo congela.

**Un jugador huérfano no aporta memoria a nadie.** No tiene Facción a la que grabarla. Su columna sigue
viendo en vivo, porque `proyectarParaJugador` resuelve eso por escuadrón y no por Facción, pero lo que ve no
queda registrado hasta que vuelva a tener bandera. Es una divergencia pequeña y consciente entre la vista y
la memoria, no un descuido.

**Medido en vivo** (servidor real, 30 Facciones en el laboratorio): una plaza recién fundada explora 44
celdas, y 76 tras 31 ticks — la zona se ensancha al completarse edificios. 800 bytes por Facción tope, y el
batch de 300 ticks sigue en 0 excepciones con las cifras de juego intactas, que es lo esperable de algo que
solo mira.

### 5.3 Lo que salió del Paso 3

**Son DOS máscaras, no una, y viajan con su geometría.** `exploracion` es
`{ tamanoCelda, columnas, filas, celdas, visibles }`:

- `celdas` = explorado alguna vez. `visibles` = lo que se ve ahora mismo, siempre un subconjunto del anterior.
- De ahí salen los tres estados sin que el cliente sepa una sola regla: fuera de `celdas` = nunca visto; en
  `celdas` pero no en `visibles` = visto antes; en `visibles` = viéndolo.

La segunda máscara se añadió al implementar el Paso 5 y es lo que hace que el estado 2 exista de verdad. El
cliente podría deducirla a partir de dónde están sus plazas y sus columnas, pero para eso necesitaría los
radios de visión y el algoritmo de marcado — o sea, una copia de las reglas del juego dentro del cliente.
Ochocientos bytes de más salen mucho más baratos que eso.

Y la geometría va con ellas porque el cliente necesita el tamaño de celda para saber qué tapa cada bit: si
tuviera que ir a buscarlo a `GET /v1/balance` bastaría una versión de más para que pintase la niebla
DESPLAZADA sobre el mapa sin que nada fallara de forma visible.

**Lo proyectado es memoria MÁS vista, no solo memoria.** Quien graba es el tick, así que entre un comando y el
siguiente tick hay una ventana en la que lo recién visto —una plaza recién fundada, por ejemplo— todavía no
está en `memoriaPorFaccion`. Sin unir ambas cosas al proyectar, el cliente pintaría niebla justo encima de lo
que el jugador acaba de hacer: la clase de agujero que nadie relaciona con un desfase de un tick. Cuesta unas
pocas decenas de celdas por proyección.

**Una plaza aparece en una lista o en la otra, nunca en las dos.** `asentamientosConocidos` es lo recordado
MENOS lo que se ve ahora y MENOS lo que entretanto pasó a ser propio (eso ya viaja completo en
`asentamientos`). Es el invariante 3, y es lo que hace que el cliente pueda pintar cada lista con su estilo
sin comprobar nada.

**Medido en vivo** (servidor real, HTTP real, dos Facciones): antes del primer tick la proyección ya trae la
plaza propia destapada y el otro extremo del mundo tapado, con rejilla 80x80 de 25; tras el tick la plaza
rival sale en `asentamientosAvistados` y NO en `asentamientosConocidos`, aunque su ficha sí está grabada en
el estado con su `conocidoEn`. 1.600 caracteres hex por Facción, como se había calculado.

### 5.4 Lo que salió del Paso 5 (el cliente de jugador)

**El orden de capas ES la niebla**, y así queda escrito en `render.ts`. Lo que se pinta antes de la máscara
queda tapado donde nunca se estuvo y a media luz donde solo se recuerda; lo que se pinta después se ve tal
cual. De ahí el criterio: geografía y lo RECORDADO van antes; lo tuyo y lo que estás VIENDO, después. Mover
una capa de un lado al otro cambia lo que el jugador sabe — no es refactor, y el comentario del archivo lo
dice con esas palabras.

**La máscara se compone a resolución de REJILLA y se estira.** 80x80 sobre el mundo de 2000, dibujada con
interpolación hasta el tamaño del lienzo. Eso da bordes de niebla suaves gratis; pintando celda a celda sobre
el lienzo grande, el mundo se vería a cuadros y la frontera delataría la rejilla en vez de parecer niebla.

**Relleno lo que ves, hueco lo que recuerdas.** Un asentamiento avistado usa el mismo glifo relleno que uno
propio —es la misma clase de cosa, y el color de su Facción ya dice que no es tuya—; uno recordado va hueco, y
además le cae encima el filtro oscuro porque se dibuja DEBAJO de la máscara. El hueco dice literalmente "aquí
hay una ciudad, no sé cómo está ahora".

**El color de la niebla es el del fondo de la página** (`--bg-primary`), no negro: así lo no explorado no se
lee como un agujero quemado en el mapa sino como mapa que todavía no está.

**Encontrado por el camino**: `caminos` y `campamentosBandidos` se proyectaban a todo el mundo sin filtrar.
El cliente los dibujaba bajo la máscara, así que en pantalla ya no flotaban sobre tierra sin pisar — pero el
dato seguía viajando. **Cerrado (2026-09-04)**: se filtran en el servidor, cada uno con su regla (§2.8), y el
campamento pasó a pintarse por encima de la máscara al dejar de tener memoria.

**Medido en vivo** (backend real en :3000, cliente real en :5174, partida de tres Facciones):

| Estado | Medido |
|---|---|
| Nunca visto | `rgb(10,14,23)` exacto: el terreno desaparece, y con él la ciudad de la tercera Facción |
| Visto antes | `rgb(67,70,75)` sobre un control gris 160 — el 42% de su brillo, justo el 62% de filtro |
| Viéndolo | `rgb(160,160,160)`: intacto |

Y el escalón entre ver y recordar, promediando en círculos alrededor de la plaza para que el terreno no meta
ruido, cae en **radio ~120** — que es exactamente `radioPotencial (60) + margenAsentamiento (60)`. La
geometría de la máscara sale bien sin que el cliente sepa ninguno de los dos números.

### 5.5 Las fronteras ajenas (2026-09-05)

Petición del usuario, después de ver la niebla funcionando: *"deberían estar los polígonos reales, pero se
siguen ocultando bajo la niebla de guerra; si paso cerca de una frontera yo debería poder verla, y también
debería poder saber si estoy en el territorio de otro cuando voy caminando."*

**Se eligió el polígono REAL frente a uno estimado.** La alternativa que se propuso —recortar la silueta solo
contra los asentamientos que el jugador conoce, para no delatar a terceros— se descartó: enseñaría una
frontera que no es la de verdad. Entre "exacta con una pista de más" y "limpia pero falsa", el usuario
eligió la primera.

**La fuga que eso deja, anotada y aceptada.** La silueta viene recortada contra TODOS los vecinos de otra
Facción, incluidos los que no has visto, y el corte se reparte según el `radioPotencial` de cada uno: un lado
plano delata que hay un tercero en esa dirección y con cuánta fuerza relativa. Está acotada —solo viaja la
zona de una plaza que YA se ve— y escrita en el propio tipo (`AsentamientoAvistado.zona`) para que nadie la
descubra por sorpresa.

**Lo que acota el contorno es el orden de capas, no un filtro.** La frontera se dibuja BAJO la máscara, así
que del trazo solo se llega a ver el tramo que cae en tierra explorada. No hizo falta escribir ni una línea
de lógica para "si paso cerca de una frontera la veo": sale del sitio en el que se pinta.

**Lo recordado guarda el RADIO, no la silueta.** Dos razones, y las dos mandan. La barata: 48 vértices son
~1,2 KB por plaza recordada y una Facción viajera recuerda decenas — cientos de KB en cada snapshot, treinta
veces. La buena: la silueta real está recortada contra vecinos que quizá no conozcas, así que congelarla
sería congelar información de terceros. Un radio es lo que de verdad recuerdas, y el cliente lo pinta
punteado para que se vea que es una aproximación.

**"¿En tierra de quién estoy?" lo responde el SERVIDOR** (`territorioPorEjercito`), aunque el cliente tenga
los polígonos. Es un hecho del juego, no una preferencia de dibujo — y el cliente solo tiene las zonas de lo
que ve, así que se equivocaría justo en el caso que lo justifica: una capital vigila 240 y una columna ve
150, o sea que se puede entrar en su territorio sin llegar a ver la ciudad. Sale solo el `faccionId`: pisar
la tierra de alguien te dice de quién es, no dónde tiene la capital.

**Medido en vivo** (tres Facciones, backend y cliente reales): la frontera compartida entre dos plazas
vecinas llega con **39 vértices en vez de 48**, o sea recortada de verdad y no un círculo; la mitad de la
zona rival que cae fuera de lo explorado se desvanece bajo la máscara; una plaza recordada sale como círculo
punteado a media luz; y una tercera Facción sin descubrir no manda ni un vértice, aunque su nombre sí esté en
`facciones` como metadato público.

## 6. Invariantes a congelar en tests

Los nueve están congelados. Entre paréntesis, dónde.

1. Un asentamiento rival **fuera** de radio+margen y de la vista de todo ejército propio **no aparece en
   absoluto** en la proyección — ni redactado. (`proyecciones/__tests__/jugador.test.ts`)
2. Lo proyectado de un rival **nunca** incluye almacén, escuadrones, edificios ni colas. Ni en vivo ni
   recordado. (ídem, por `Object.keys` sobre lo avistado: un campo nuevo rompe el test en vez de colarse)
3. Un asentamiento visto en vivo y además recordado aparece **una sola vez**, y como visto. (ídem)
4. La foto congelada **no cambia** cuando cambia el asentamiento real; solo cuando se vuelve a ver.
   (`engine/__tests__/memoria.test.ts`, subiendo de nivel la plaza mientras nadie la mira)
5. Una Facción sin registro de memoria proyecta listas vacías y exploración vacía, nunca `undefined`.
   (`proyecciones/__tests__/jugador.test.ts`, con un jugador sin Facción)
6. **Lo que se ve, se graba**: un asentamiento que entra en alcance y vuelve a salir sigue proyectándose como
   recordado, nunca desaparece del todo. Es el tránsito del estado 3 al 2, y era el error de la primera
   versión de este documento. (`engine/__tests__/memoria.test.ts`)
7. La exploración solo CRECE. Nada la reduce: lo explorado no se desexplora.
   (`engine/__tests__/exploracion.test.ts` y `session/__tests__/memoriaNiebla.test.ts`, a 21 ticks)
8. La zona que viaja de una plaza avistada es la SILUETA REAL del motor, recortada contra sus vecinos — no un
   círculo ni una estimación. (`proyecciones/__tests__/jugador.test.ts`, contando vértices: recortar quita
   arco y deja cuerda, y los vértices que quedan siguen todos sobre la circunferencia, así que la distancia
   al centro no distingue nada y contar sí)
9. `territorioPorEjercito` acierta **aunque la ciudad que manda en esa tierra no se vea**: es el caso que lo
   justifica. (ídem, con una plaza de radio 300 y la columna a 200, fuera de su alcance de vista)
8. Un camino **entero en tierra sin explorar no viaja**, y uno que cruza lo explorado viaja **entero**, con
   todos sus puntos. (`proyecciones/__tests__/jugador.test.ts`)
9. Un campamento de bandidos en un rincón **explorado pero que ahora no se ve no viaja**, aunque el camino
   que pasa por ese mismo rincón sí. Es la asimetría de §2.8 en un solo test. (ídem)
