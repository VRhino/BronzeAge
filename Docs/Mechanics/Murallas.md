# Murallas — los recintos

> **Reescrito el 2026-10-13** para el mundo reactivo: la obra de un recinto es una obra con evento de fin por celda y el upkeep es un ritmo. Las murallas por nivel (empalizada nivel 2, piedra nivel 3, adarve nivel 4) siguen como en el canon 4.2.1. La jerarquía de puertas se descartó y la demolición de un recinto está en Ideas. El «cómo funcionaba» está en `typescript-final:Docs/Mechanics/Murallas.md`.
>
> **Revisado para la reconstrucción (2026-10-22), con las decisiones del autor del mismo día (Q1-Q5):**
> - **Q1, defiende lo que está en pie.** Mientras un recinto se mejora o se amplía, el mejor recinto completo (con su nivel de antes) sigue dando multiplicador, cupo de guarnición y requisito del nivel 4; lo nuevo cuenta al cerrarse. Es igual con números que en Unity, y el mantenimiento cuenta las celdas en pie. Sustituye la regla del 2026-10-09 de «integridad 0 al mejorar»: el progreso de una mejora ya no se guarda en `avance`, sino en una `mejora` aparte (§2, §6).
> - **Q2.** Tras una conquista la reparación sigue sola a costa del nuevo dueño; un recinto que alguna vez estuvo completo no se puede abandonar (solo los que nunca se cerraron) (§7, §9.6).
> - **Q3.** Ampliar solo exige el arrabal: recinto anterior completo y ≥ 6 edificios extramuros, sin nivel de plaza propio (§8).
> - **Q4, tarifas acumuladas.** Construir directamente en un nivel cuesta lo mismo que subir nivel a nivel: el coste directo del nivel N es la suma de las tarifas de los niveles 1..N (por celda llana: empalizada 22, piedra 52, adarve 82). Se sigue pudiendo comprometer directamente en cualquier nivel (2026-10-10) (§4, §5, §11).
> - **Q5.** Esta fase da solo `recintos_en_pie(plaza)` y el dibujo (`VistaCiudad.murallas`); el ticket del asedio y el informe de inspección de una plaza los congela y proyecta la fase 4H (§9.1).
>
> Además se corrigieron restos del TypeScript (el texto viejo del multiplicador, el recálculo de torres al mejorar, el upkeep de una mejora) y se dejó **una sola copia** del bloque `GUARNICION` (la de [`Heroe_Y_Tropa.md`](Heroe_Y_Tropa.md) §7). La lista de lo resuelto está en §13.

Cómo se traza, se paga, se mejora, se amplía, defiende, cuesta mantener y se daña un **recinto** (la muralla de un asentamiento). Un recinto **no es un edificio**: es un anillo cerrado de celdas
alrededor del casco urbano que se levanta celda a celda.

- **Referencia en TypeScript** (`typescript-final`, verificado contra `e18c0b4`, 2026-10-09): `src/engine/muralla.ts` (≈980 líneas: trazo, comprometer, obra, mejora, ampliación, defensa, upkeep), `src/engine/trazado.ts`, `src/engine/combate.ts`,
  `src/engine/asentamientoQuery.ts`, `src/engine/mantenimiento.ts`, `src/session/comandos/murallas.ts`.
- **En Rust:** el trazo es `trazado::muralla` (`crates/trazado/src/muralla.rs`, §3); la economía de los recintos (obra, mejora, defensa, upkeep) es `asentamiento::recintos` (`crates/asentamiento/src/recintos.rs`) y se guarda en la plaza; el núcleo (`crates/nucleo/src/nucleo/murallas.rs`) los traza, los ordena y los sincroniza con la ciudad.
- **Canon:** Doc 4 «Murallas», Doc 4.5 (nivel 4), Doc 5.2.5 (el multiplicador en el combate), Doc 5.15.3 (guarnición). **Diseño e historia:** `typescript-final:Consideraciones/Murallas_Definicion.md`; contraste y plan: [`../Arquitectura/Murallas_Contraste_y_Plan.md`](../Arquitectura/Murallas_Contraste_y_Plan.md).
- Vocabulario de celdas, rejilla y red de calles: [`Trazado_Urbano.md`](Trazado_Urbano.md).
- «Nivel de recinto» (1 empalizada, 2 muro de piedra, 3 muralla con adarve) no es el «nivel de plaza» (el del asentamiento, 1 a 5).

## 1. Idea

La muralla es una ventaja defensiva abrumadora —**menos puertas benefician al defensor**, que solo defiende un embudo— y por eso tiene que ser cara de obtener **y de mantener**. Eso fija la forma
de las reglas: el coste escala con el perímetro, existe un mantenimiento por celda, y el bono defensivo se reparte entre las puertas. Amurallar pronto da una fortaleza con pocas puertas; amurallar tarde
protege una metrópoli con más frentes que cubrir.

**El jugador no elige dónde va la muralla**: el trazo es automático y determinista (§3). El jugador solo decide el nivel y quién la encarga; una torre no es un edificio, es una clase de celda del anillo.

## 2. Modelo de datos

```
CeldaMuro = { col, row, clase: 'muro' | 'puerta' | 'torre' }

Recinto = {
  id:             (plaza, n)  con n = nº de recintos existentes + 1   (en el contrato de texto: 'recinto-<idAsentamiento>-<n>')
  nivel:          1 empalizada · 2 muro de piedra · 3 muralla con adarve      el nivel del anillo EN PIE
  celdas:         CeldaMuro[]   el anillo COMPLETO en orden de recorrido desde la puerta principal — es el orden en que se levanta. CONGELADO al comprometer
                                (solo cambia la clase de las celdas de muro a torre y viceversa al terminar una mejora)
  avance:         índice de la última celda ya levantada del anillo en pie. −1 = ninguna. Completo ⇔ avance = celdas.length − 1
  mejora?:        { nivel (destino), avance }   solo mientras se mejora: su progreso va APARTE, el anillo en pie conserva su avance y su nivel
  siguienteCeldaEn?: instante antes del cual no se levanta otra celda
  comprometidoEn, completadoEn?:  instante en que se cerró POR PRIMERA VEZ; no se borra (distingue un recinto dañado de uno que nunca se cerró)
}
Asentamiento.recintos?: Recinto[]    del más interior al más exterior
```

- `integridad = (avance + 1) / celdas.length` (0 si no hay celdas), siempre la del anillo en pie: **una mejora no la toca**.
- Un recinto está **completo** si `avance = celdas.length − 1` (aunque se esté mejorando) y **dañado** si tiene `completadoEn` y `avance` por debajo (lo dejó así una conquista, §9.6); uno **en obra** es el que aún no ha llegado a cerrarse nunca.
- Las celdas **no levantadas ocupan suelo igual** desde que se compromete (si no, se construiría encima y el anillo no podría cerrarse). Las **puertas no ocupan**.
- **Las puertas se congelan al trazar**: ni mejorar ni el crecimiento de la ciudad las cambian. Ganar puertas solo se consigue **ampliando**.

## 3. El trazo (`trazar_recinto`, función pura)

Entrada: la ciudad (`Ciudad`) y el `nivel` que se presupuesta (por defecto 1). Salida: `Trazo { celdas (ordenadas), area_encerrada, dentro, fuera, afueras_dentro }` (las puertas y las torres se cuentan de las celdas) o un **error explícito** (no hay
trazo válido: «un recinto que no se puede cerrar no es un recinto»). Determinista: el orden de todo lo que decide es el de la rejilla (fila y columna) y nunca el de un conjunto hash. El coste lo calcula quien conoce las tarifas (§4, §11) con las puertas y las torres del trazo.

```
internos := las piezas de la ciudad;  centro := el Centro Urbano (si no hay → error SinCentroUrbano)
red := la red de calles de la ciudad SIN recintos: el trazo se calcula sobre la ciudad, no sobre sí mismo (y una ampliación no ve el anillo viejo como muro)

1. vetadas := celdas de granjas y corrales;  vetadasConMargen := dilatar(vetadas, 1)
2. núcleoBruto := celdas de las piezas urbanas (no afueras) ∪ celdas de CALLE (nunca caminos: un camino rural arrastraría la región hasta las granjas),
                  todo menos vetadasConMargen
   núcleo := componente conexa (vecindad 4) de núcleoBruto que contiene al Centro Urbano;  si vacío → error NoSePuedeCerrar
3-5. semilla := núcleo
   repetir hasta MAX_ITERACIONES_CONFLICTO (24):
       dilatada := dilatar(semilla, franjaDeRonda)                         // vecindad 4 → esquinas achaflanadas
       región   := rellenarAgujeros( componente conexa del Centro Urbano en (dilatada − vetadasConMargen) )
       muro     := anillo(región) = celdas fuera de la región 4-adyacentes a ella          // si la región toca el borde de la ventana → error FueraDeVentana
       conflictivos := piezas con alguna celda en muro
       si ninguno → resuelto
       si no: quitar de la semilla la vecindad (dilatada franjaDeRonda+1) de esas piezas; si no se quita nada → error NoSePuedeCerrar; semilla := componente del Centro Urbano; si vacía → error
   si no resuelto → error NoSePuedeCerrar
6. validar: un flood fill 4-conexo desde fuera que no cruce el muro no puede alcanzar el Centro Urbano; si lo alcanza → error NoSePuedeCerrar
7. clasificarPuertas:  celda del muro que pisa la red (calle o camino) → 'puerta'; el resto 'muro'
8. ordenarAnillo → colapsarPuertas → marcarTorres
```

- **Ventana.** El trazo trabaja en la rejilla de la ciudad, de ±96 celdas ([`Trazado_Urbano.md`](Trazado_Urbano.md) §2). El anillo (la región más una celda) tiene que caber entera; si no cabe **se rechaza** (`FueraDeVentana`), nunca se recorta. Con el `radioCiudad` máximo (180 u.l. = 60 celdas) cabe con holgura; una ampliación tras mucho crecimiento puede rozar el borde.
- **Dilatación** con vecindad 4 (`franjaDeRonda` pasos): sigue la forma real de la ciudad (una estrella de distritos), no un círculo ni un casco convexo.
- **`rellenarAgujeros`**: toda celda fuera de la región y sin salida al exterior pasa a la región (si no, un patio encerrado generaría un anillo interior). **Excepción conocida:** una
  granja o corral rodeado por los cuatro costados queda **intramuros** (`afueras_dentro`); no es un fallo, ya no son «las afueras». Aparece desde el nivel 3 (0,3–12 % de las celdas urbanas).
- **El anillo** es la capa 4-adyacente exterior: mide una celda y es impermeable (para entrar hay que pisar una celda 4-adyacente a la región, y todas son muro).
- **`ordenarAnillo`**: empieza en la **puerta de menor `(row, col)`** (la «principal»; si no hay puertas, la celda de menor `(row, col)`), y sigue por **vecindad 8** probando `dr = −1..1`, `dc = −1..1` (en ese orden)
  la primera vecina no visitada. Si el paseo se queda sin salida en un punto grueso (una esquina de la capa 4-adyacente deja una celda atrás), sigue desde la **celda sin visitar más cercana** (Chebyshev; a igual distancia, la de menor `(row, col)`), de modo que no se pierde ninguna celda y el desorden queda en un salto local (en 20 ciudades de 35 a 130 piezas salen de 0 a 3 saltos por anillo).
  *Cambio respecto al TypeScript*, que al atascarse añadía al final todas las que faltaban ordenadas por `(row, col)` y dejaba más de la mitad del anillo desordenado en una ciudad pequeña. `// DUDA:` pendiente de confirmar por el autor (no cambia nada del juego salvo el orden de levantamiento).
- **`colapsarPuertas`** (una puerta por cruce): recorre el anillo **circularmente**; de cada tirada contigua de puertas conserva **una** celda —la primera que «atraviesa» (tiene red dentro de la región a un lado y red fuera del muro al otro),
  o la del medio (`floor(n/2)`) si ninguna lo hace— y devuelve el resto a `muro`. Importa porque **el nº de puertas es el divisor del bono defensivo**: un camino que rozara el muro cinco celdas dividiría la defensa por cinco.
- **`marcarTorres`** (solo niveles con `pasoTorres`: 2 y 3; la empalizada no tiene): **primero borra las torres que hubiera** (al mejorar de nivel se recalculan sobre las celdas de muro y no quedan torres juntas), después recorre el anillo ya ordenado con un contador `desdeLaUltima` que empieza en `paso`. En cada celda: si es puerta, solo cuenta; si no,
  se convierte en torre cuando `desdeLaUltima ≥ paso` **y** es **esquina convexa** (la región está en su vertical y en su horizontal), o cuando se llevan `2 × paso` sin ninguna. Una puerta nunca es torre. Marcar las torres del nivel 3 sobre el anillo del nivel 2 da exactamente el trazo directo del nivel 3.
- **Coste del trazo**: `Σ tarifaPorCelda[nivel] × factor(clase)` (la tarifa es la **acumulada**, §11), con `factor` = `factorPuerta` (4) para puertas, `factorTorre` (3) para torres, 1 para muro; redondeado por recurso.
- **Consultas sobre un anillo ya trazado** (todas puras): `interior_de(celdas)` (inundación 4-conexa desde el Centro Urbano que no cruza el anillo, **puertas incluidas**: la puerta es un hueco para quien camina, no un agujero en la contabilidad del interior; da el mismo resultado que la región del trazo), el área encerrada (el tamaño del interior),
  `contiene_a(nuevo, anterior)` (§4) y `edificios_extramuros(interior)` (§4).

## 4. Comprometer

`comprometerRecintoManualmente(asentamiento, cargo, nivel, instante)` (comando `comprometerRecinto`, Gobernador o Maestro de Obras residente con ese cargo):

1. Debe haber alguien con ese cargo. 2. **Gate de nivel de plaza por nivel de recinto** (`MURALLA.nivelAsentamientoPorNivelDeRecinto`): empalizada (1) desde la plaza de nivel 2, muro de piedra (2) desde el 3,
muralla con adarve (3) desde el 4 (el nivel **operativo**; decidido el 2026-10-09). 3. Delega en `comprometerRecinto`:

```
si algún recinto existente está incompleto → error («ya hay un recinto en obra»)          // un recinto dañado por una conquista también lo bloquea hasta repararse
si hay recintos (AMPLIACIÓN):
    extramuros := edificiosExtramuros(último recinto);  si < arrabalMinimo (6) → error      // sin más gate de plaza que el del nivel elegido (Q3)
trazo := trazar_recinto(ciudad, nivel);  si es un error → rechazo (sin Centro Urbano · no se puede cerrar · no cabe en la ventana)
si hay recintos y el trazo no CONTIENE ESTRICTAMENTE al anterior → error
crear Recinto { nivel, celdas: trazo.celdas, avance: −1, comprometidoEn }
```

- **Comprometer es gratis.** Lo que cuesta es levantarlo, celda a celda (§5). Así «anillo a medio cerrar» es un estado normal y visible.
- **Consulta de presupuesto** (sin comprometer): el mismo trazo con sus celdas, puertas, torres, coste, área y extramuros. Hace falta porque el nº de puertas es irreversible y se enseña antes de pagar.
- **`contiene_a(nuevo, anterior)`**: ninguna celda del anillo nuevo coincide con una del viejo, y un flood fill desde fuera del anillo nuevo no llega a ninguna del viejo sin cruzar el muro nuevo. Es el invariante «el área encerrada nunca decrece». Calcar el anillo no es ampliar. Si la ciudad solo ha crecido por un lado, el anillo nuevo coincide con el viejo en los otros y la ampliación se rechaza (en la práctica hace falta haber crecido por todos los lados).
- **Extramuros** = edificios urbanos (no granja/corral) con alguna celda fuera del interior del recinto.
- El interior de un recinto ya congelado es `interior_de` (§3); da el mismo número en cualquier instante posterior (el anillo no cambia).
- Un recinto se puede comprometer **directamente en cualquier nivel permitido por su gate** (el esquema acepta 1, 2 o 3) pagando la tarifa acumulada de ese nivel (§5, §11), sin pasar por los anteriores. Es intencional (2026-10-10): permite defenderse antes con lo que se pueda, o levantar la muralla más tarde con la ciudad más grande. Por la tarifa acumulada (Q4) saltarse niveles **no** sale más barato que subirlos uno a uno.

## 5. La obra

Se corre como cualquier obra de cada asentamiento **después** de `evaluarNecesidades`, para que la construcción normal tenga preferencia sobre los materiales. No ocupa hueco de la cola de edificios. Sirve para tres cosas con el mismo recorrido: **construir** un recinto, **repararlo** (un recinto dañado, §9.6) y **mejorarlo** (§6).

```
para cada recinto en obra, dañado o con mejora:
    pagado := nivel en que se paga:   construir o reparar → el nivel del recinto;   mejorar → el nivel destino
    celda  := la siguiente a levantar:   construir o reparar → celdas[avance + 1];   mejorar → celdas[mejora.avance + 1]
    si siguienteCeldaEn existe y instante < siguienteCeldaEn → nada
    coste := tarifa × factor(clase de la celda):   construir o reparar → tarifaPorCelda[nivel];   mejorar → tarifaPorCelda[destino] − tarifaPorCelda[destino − 1]
    si no hay recursos, o pagarlos dejaría un recurso por debajo de la RESERVA de mantenimiento → nada en este evento (no falla, no se cancela, no hay deuda)
    descontar; avanza una celda;  siguienteCeldaEn := instante + minutosPorCelda[pagado] × factorTiempoMuralla
    si el recinto se cierra por primera vez: completadoEn := instante; evento construccion.recinto_completado
    si se cierra una reparación: no hay evento propio (vuelve a estar completo)
    si se cierra una mejora: nivel := destino; recalcular torres sobre el mismo anillo (marcarTorres con el interior); borrar mejora; evento construccion.recinto_mejorado
```

- La **reserva** es la misma que cualquier obra automática: la obra nunca se come lo que Mantenimiento y la comida van a cobrar. Sin excepción por recurso.
- **`factorTiempoMuralla`**: la política «Postura Defensiva» (Maestro de Obras) lo pone a 0,75 (más rápido). Producto de los factores de las políticas activas.
- Se avanza **una celda por pasada** (el contador `siguienteCeldaEn` espacia el ritmo): 15 / 30 / 60 minutos por celda a empalizada / piedra / adarve.
- **Reloj.** Cada celda es una cita en `siguienteCeldaEn` (obra de fin programado, como las demás: ver «fines exactos» en [`Economia_Construccion.md`](Economia_Construccion.md)). Si cuando llega no hay materiales o se bajaría de la reserva, no se programa otra y el paso de minuto reintenta la obra; con la plaza ocupada tras una conquista el mantenimiento está congelado pero la obra sigue.
- El orden de levantamiento es el del anillo: se ve cerrarse desde la puerta principal.
- **El coste de mejorar es la diferencia de tarifas acumuladas**, de modo que 1→2→3 cuesta lo mismo que comprometer directamente el 3 (por celda llana: 22 + 30 + 30 = 82). La clase de la celda que se paga es la que tiene en el anillo en pie; las torres nuevas se marcan al terminar, así que la suma de subir nivel a nivel puede diferir del directo en las celdas que cambian de clase.

## 6. Mejorar de nivel

`iniciarMejoraDeRecintoManualmente(asentamiento, cargo, recintoId)` (comando `mejorarRecinto`, mismo camino que comprometer):

1. Cargo asignado. 2. **Gate de nivel de plaza del nivel destino** (`nivel + 1`): a piedra desde el 3, a adarve desde el 4. 3. `iniciarMejoraDeRecinto`: el recinto existe, está **completo** (`avance = celdas.length − 1`: un recinto dañado primero se repara),
no tiene ya una mejora en curso y `nivel < nivelMaximo (3)`. Resultado: `mejora = { nivel: nivel + 1, avance: −1 }`. Gratis empezarla.

La obra de mejora es **la misma obra** (§5) pagando la diferencia de tarifas del nivel destino. No se retraza nada: puertas y largo del anillo no
cambian; al terminar solo se recalculan qué celdas de muro pasan a torre (§3). **Mientras dura, el anillo en pie sigue defendiendo con su nivel de antes** (multiplicador, cupo de guarnición y requisito del nivel 4, §9) y su upkeep no cambia (Q1).

## 7. Abandonar

`abandonarRecinto`: solo el **Gobernador**. Solo un recinto **que nunca se ha cerrado** (sin `completadoEn`): se borra entero (celdas levantadas incluidas), su suelo queda libre, **sin devolución**. Es una escotilla de seguridad (una obra atascada por
falta de piedra bloquearía su franja para siempre), no una mecánica. **Un recinto que alguna vez estuvo completo no se destruye**, ni siquiera dañado por una conquista (Q2): se repara.

## 8. Ampliar

Es comprometer otro recinto cuando ya hay uno (§4): el anterior completo, ≥ 6 edificios extramuros (`arrabalMinimo`) y el trazo nuevo envuelve estrictamente al viejo. **No hay más gate que ese** (Q3: ni un nivel de plaza propio; el que se elija de recinto lleva su gate normal) ni tope de anillos, y cada anillo interior paga su upkeep para siempre (no hay demolición: Ideas §47).
Los recintos interiores son «historia en piedra»: **defiende el más exterior de los que han llegado a cerrarse** (§9.1), y **mientras el nuevo está en obra sigue defendiendo el anterior** (Q1).

## 9. Qué hace un recinto

### 9.1 Bono defensivo en el asedio

El **recinto de referencia** es el **más exterior de los que han llegado a cerrarse alguna vez** (`completadoEn`); si ninguno se ha cerrado, el más exterior que haya. Así, mientras se levanta una ampliación o se mejora un recinto, el anillo anterior (con su nivel de antes) sigue siendo el de referencia; el nuevo cuenta al cerrarse (Q1).
`1` si no hay ninguno.

```
1 + (bonoDefensaPorNivel[nivel] − 1) × integridad / nºPuertas        (nºPuertas = 0 → 1)
```

El «1 +» garantiza que un muro nunca perjudique al defensor; `integridad` hace que un anillo a medio cerrar (el primero, a medio levantar, o uno dañado por una conquista) defienda casi nada; dividir por las puertas es lo que hace real el eje fortaleza↔metrópoli. Con el bono de
nivel 3 (2,5) y el anillo entero: ×2,5 con una puerta, ×1,38 con cuatro, ×1,19 con ocho.

- **Solo se aplica en el asedio de una plaza resuelto con números**, es decir, cuando no hay servidores de batalla de Unity ([`Combate.md`](Combate.md) §6: con servidores toda batalla va a Unity, también las de bot contra bot). Multiplica el poder del defensor junto con `MILITAR.ventajaDefensor`. La columna, el pueblo extractor, la caravana y la avanzada siguen con multiplicador 1: **las avanzadas no tienen recintos** (su fortaleza son las guarniciones de serie de cada etapa, [`Combate.md`](Combate.md) §7.4; «fuerte (empalizada)» y «ciudadela (piedra)» son nombres de etapa).
- **El equipo de asedio no lo reduce** (decidido el 2026-10-19: sin efecto en el combate con números). Lo que sí exigen las tecnologías de Carpintería es el predicado «plaza con **muralla completa**» (≥ 1 recinto con integridad ≥ 1) para sus contadores de logros (`asedios.contraMurallaCompleta`, `conquistas.conMurallaCompleta`), que se calcula **antes** del daño de la conquista.
- No consume tirada del generador: el multiplicador es una constante de la plaza en ese instante.

**En una batalla de Unity el recinto no es un multiplicador: viaja como geometría.** La batalla entrega a Unity, por cada recinto, `{ recintoId, nivel, celdas }` con las celdas **levantadas** (`celdas[0..avance]`, al nivel que tiene en pie; una mejora en curso no cambia nada), con `layoutVersion` y `unidadesPorCelda`; un recinto sin celdas en pie no viaja. Lo que la muralla vale dentro de la partida lo decide Unity. En esta fase existen la función `Nucleo::recintos_en_pie(plaza)` y el dibujo (`VistaCiudad.murallas`, levantadas y planificadas, fusionadas por clase); el ticket del asedio con el snapshot de la plaza y el informe de inspección con los `recintos {nivel, celdas, avance}` los congela y proyecta la fase 4H (Q5).

### 9.2 Cupo de guarnición

`cupoGuarnicion` ([`Heroe_Y_Tropa.md`](Heroe_Y_Tropa.md) §7, donde está el bloque `GUARNICION`) suma `recintoCompletoPorNivel[n−1]` (+10 / +15 / +20), con `n` = nivel del **mejor** recinto con integridad ≥ 1 (no se acumulan), y solo si el asentamiento ya tiene cupo por edificios (Barracón, Galería de tiro o Caballerizas; sin ninguno el cupo es 0).
Un recinto en mejora **sigue sumando con su nivel de antes** (Q1). Progresivo por decisión del autor (2026-10-10).

### 9.3 Requisito del nivel 4

`NIVEL_ASENTAMIENTO.requisitos[4]` exige `recintoCompletoNivelMinimo = 2`: un recinto **completo de piedra o superior** (la empalizada no basta: el comentario del TypeScript que decía lo contrario era un resto obsoleto), 1.000 pesants, 800 artesanos y una Sala del Consejo.
En Rust se resume en un entero, el `recinto_nivel` del `Exterior` = el nivel del mejor recinto con integridad ≥ 1; un recinto en mejora cuenta con su nivel de antes.

### 9.4 Mantenimiento (upkeep)

`upkeepDeRecintos`: por cada recinto con celdas levantadas, `upkeepPorCelda[nivel] × (avance + 1)` por recurso y minuto (un **ritmo** de coste que se suma al mantenimiento). Usa el nivel y el `avance` del **anillo en pie**: una mejora no lo cambia (las celdas que se van mejorando no cobran el nivel nuevo hasta cerrarse la mejora)
y un recinto en reparación paga las celdas que quedan en pie. Se **suma** al coste de mantenimiento del asentamiento
(`calcularCostoMantenimiento`) sin el factor de población ni de distancia: el muro cuesta lo que pesan sus celdas. Con la plaza ocupada tras una conquista el mantenimiento está congelado.

### 9.5 Colocación de edificios

Los recintos influyen en dónde caen los edificios: muro y torre ocupan suelo, y con un recinto exterior completo los urbanos se colocan primero intramuros ([`Trazado_Urbano.md`](Trazado_Urbano.md) §6.6).

### 9.6 Daño por conquista

`aplicarConquista`: cada recinto **completo y sin mejora en curso** pierde `floor(fraccionDanoMuralla × celdas.length)` de `avance` (mínimo −1). Un recinto en obra o con una mejora en curso **no recibe daño** y lo sigue el nuevo dueño. La muralla no cae: el recinto dañado deja de dar el multiplicador pleno (su integridad baja), el cupo y el requisito del nivel 4 (integridad < 1) hasta repararse **solo, por la obra normal (§5), a costa del nuevo dueño** (Q2), aunque la plaza esté ocupada.
Mientras tanto bloquea mejorar y ampliar (está incompleto) y **no se puede abandonar** (§7). La obra de ascenso en curso, en cambio, la conquista sí la cancela.

## 10. Comandos y permisos

| Comando (código en Rust) | Quién | Qué |
|---|---|---|
| `comprometerRecinto { asentamientoId, cargo, nivel ∈ {1,2,3} }` (123) | residente con el cargo `gobernador` o `maestroObras` | §4. Evento `construccion.recinto_comprometido`; `creadas` lleva el id del recinto |
| `mejorarRecinto { asentamientoId, cargo, recintoId }` (124) | ídem | §6. Evento `construccion.recinto_mejora_iniciada` |
| `abandonarRecinto { asentamientoId, recintoId }` (125) | solo el Gobernador | §7. Evento `construccion.recinto_abandonado` |

**Rechazos en Rust** (códigos estables `murallas.*`, en el orden en que se comprueban; un rechazo no deja rastro). Siempre antes: `asentamiento.desconocido` y `no_autorizado`. *Comprometer:* `sin_cargo` (no es el Gobernador ni el Maestro de Obras), `nivel_inexistente`, `nivel_de_plaza` (empalizada 2, piedra 3, adarve 4), `en_obra` (hay un recinto sin cerrar, también uno dañado), `arrabal` (ampliar con menos de 6 edificios extramuros), `sin_centro_urbano` / `no_se_puede_cerrar` / `fuera_de_la_ventana` (del trazo) y `no_envuelve`. *Mejorar:* `sin_cargo`, `recinto_desconocido`, `en_el_maximo`, `nivel_de_plaza` (del nivel destino), `incompleto`, `ya_mejorando`. *Abandonar:* `recinto_desconocido` y `ya_cerrado` (alguna vez estuvo completo). Hasta el sistema de cargos (5) las tres órdenes las autoriza `manda_en` como las demás de plaza, el cargo que se invoca se da por ocupado y abandonar no lleva cargo.

**Hechos de la obra** (datos entre paréntesis): `construccion.recinto_comprometido` (recinto, nivel, celdas, puertas), `construccion.recinto_celda` (recinto, levantadas, siguienteEn; −1 si era la última), `construccion.recinto_completado` y `construccion.recinto_mejorado` (recinto, nivel, celdas, puertas), `construccion.recinto_mejora_iniciada` (recinto, nivel destino) y `construccion.recinto_abandonado` (recinto). La reparación de un recinto dañado no tiene hecho de cierre propio (el último `recinto_celda`).

La auto-construcción **no** levanta murallas; la gobernanza de los bots las añade a mano (`bots/cerebro/gobierno.ts`: comprometer el nivel 1 y mejorar a piedra cuando la subida al nivel 4 está al alcance).
Eventos de la obra: `construccion.recinto_completado` y `construccion.recinto_mejorado`, con `{ recintoId, nivel, celdas, puertas }`.

## 11. Parámetros

`tarifaPorCelda` es la tarifa **acumulada** (Q4): lo que cuesta una celda llana construida directamente en ese nivel (22, 52 y 82 en total); mejorar cuesta la diferencia con el nivel anterior (§5).

<!-- parametros:MURALLA -->
```json
{
  "nivelAsentamientoPorNivelDeRecinto": {
    "1": 2,
    "2": 3,
    "3": 4
  },
  "nivelMaximo": 3,
  "franjaDeRonda": 1,
  "pasoTorres": {
    "2": 8,
    "3": 5
  },
  "arrabalMinimo": 6,
  "minutosPorCelda": {
    "1": 15,
    "2": 30,
    "3": 60
  },
  "factorPuerta": 4,
  "factorTorre": 3,
  "tarifaPorCelda": {
    "1": {
      "madera": 20,
      "piedra": 2
    },
    "2": {
      "madera": 25,
      "piedra": 27
    },
    "3": {
      "madera": 35,
      "piedra": 47
    }
  },
  "upkeepPorCelda": {
    "1": {
      "madera": 0.02
    },
    "2": {
      "piedra": 0.02
    },
    "3": {
      "piedra": 0.04
    }
  },
  "bonoDefensaPorNivel": {
    "1": 1.3,
    "2": 1.8,
    "3": 2.5
  }
}
```
<!-- /parametros -->

<!-- parametros:OCUPACION -->
```json
{
  "proteccionMinutos": 1440,
  "duracionMinutos": 90,
  "fraccionSaqueoPoblacion": 0.25,
  "fraccionEdificiosDanados": 0.25,
  "fraccionCosteReconstruccion": 0.5,
  "fraccionDanoMuralla": 0.3,
  "factorRecaudacion": 0.5,
  "factorCrecimiento": 0.5
}
```
<!-- /parametros -->

El bloque `GUARNICION` (cupo por edificios y `recintoCompletoPorNivel`) vive solo en [`Heroe_Y_Tropa.md`](Heroe_Y_Tropa.md) §7.

Los valores de `tarifaPorCelda` (acumuladas desde el 2026-10-22; antes eran las de mejora, 22 / 30 / 30, importadas del TypeScript), `upkeepPorCelda` y los ritmos son **placeholder sin calibrar**: nunca se midieron (se calibran con el batch, al final de la fase de Murallas). `fraccionDanoMuralla` está en `OCUPACION`; `ventajaDefensor` en `MILITAR`.

## 12. Invariantes

| Invariante | Dónde se vigila |
|---|---|
| El anillo aísla el Centro Urbano: un flood fill desde fuera, sin cruzar muro/torre, no lo alcanza | `crates/trazado/tests/muralla.rs` (y validación en el trazo) |
| El anillo no pisa ninguna pieza y es la capa exterior del interior; el recorrido no repite ni pierde celdas | `crates/trazado/tests/muralla.rs` |
| Cada cruce con la red es **una** puerta | `crates/trazado/tests/muralla.rs` |
| La red de calles sigue siendo un solo componente conexo con el muro puesto | `crates/trazado/tests/muralla.rs` |
| Las torres respetan su separación y mejorar de nivel recalcula las torres sin juntarlas | `crates/trazado/tests/muralla.rs` |
| Si el anillo no cabe en la ventana, se rechaza sin recortar; el trazo es determinista | `crates/trazado/tests/muralla.rs` |
| Una ampliación envuelve estrictamente a la anterior | `crates/trazado/tests/muralla.rs` |
| Las celdas sin levantar ocupan suelo desde el compromiso | `crates/nucleo/tests/murallas.rs` (comprometer, abandonar) |
| Un recinto incompleto no se mejora; uno que alguna vez estuvo completo no se abandona | `crates/asentamiento/src/recintos.rs` y `crates/nucleo/tests/murallas.rs` (rechazos) |
| Cada nivel de recinto respeta su gate de plaza (2/3/4) al trazar y al mejorar | ídem |
| Mejorar o ampliar no quita multiplicador, cupo ni requisito del nivel 4 al anillo en pie | ídem (mejora y ampliación con Q1) |
| El bono nunca baja de 1 | `crates/asentamiento/src/recintos.rs` |
| El multiplicador no consume tirada: un asedio sin recinto es el de antes, con y sin anillo el generador queda en el mismo punto | `crates/combate/src/asedio.rs` y `crates/nucleo/tests/murallas_asedio.rs` |
| El multiplicador vale ×2,5 / ×1,375 / ×1,1875 con 1 / 4 / 8 puertas (nivel 3) y solo cambia el poder del defensor; la columna y la avanzada llevan 1 | `crates/combate/src/asedio.rs` y `crates/nucleo/tests/murallas_asedio.rs` |
| La conquista daña el 30 % de cada recinto completo y sin mejora, no toca los que están en obra o en mejora, y la reparación sigue sola con su cita; es igual por los números y por Unity | `crates/combate/src/conquista.rs`, `crates/nucleo/tests/murallas_asedio.rs` y `crates/nucleo/tests/batallas.rs` |
| Lo que viaja a Unity son solo las celdas en pie, con el nivel en pie | `crates/nucleo/tests/murallas_asedio.rs` |
| Subir nivel a nivel cuesta lo mismo que comprometer directamente el nivel (celda llana) | `crates/asentamiento/src/recintos.rs` |
| Todo recinto con una celda esperando tiene su cita en la cola y no hay citas de sobra | `crates/nucleo/tests/murallas.rs` y `inv_fines` del batch |

## 13. Contradicciones resueltas (2026-10-22)

| # | Resolución |
|---|---|
| K1 | El jugador no coloca la muralla: trazo automático (§1; corregido el canon 4.2). |
| K2 | La fórmula del multiplicador, «solo el recinto de referencia» y «solo con números» pasan al canon 5.2.5. |
| K3 | El texto viejo del §9.1 dice ya la regla vigente: el multiplicador solo vale en el asedio resuelto sin servidores de Unity (§9.1). |
| K4, K5 | Q1: defiende lo que está en pie, igual con números que en Unity (§6, §8, §9). |
| K6 | La mejora lleva su progreso aparte; el upkeep cuenta el anillo en pie (§2, §9.4). |
| K7 | `marcarTorres` borra antes las torres (§3); probado. |
| K8 | Q2: la reparación sigue sola; lo que alguna vez estuvo completo no se abandona (§7, §9.6). |
| K9 | Q4: tarifas acumuladas; mejorar cuesta la diferencia (§5, §11). |
| K10 | Q3: solo el arrabal (§8). |
| K11 | El bloque `GUARNICION` queda solo en `Heroe_Y_Tropa.md`. |
| K12 | La empalizada no basta para el nivel 4 (§9.3); el comentario del TypeScript y `nivelMinimoConstruccion` de Ritmo_Crecimiento son restos que no se portan. |
| K13 | `recinto_nivel` = nivel del mejor recinto con integridad ≥ 1 (§9.3). |
| K14 | Id numérico `(plaza, n)` en Rust, texto en el contrato; el snapshot de Unity lo proyecta 4H (Q5, §9.1). |
| K15 | El equipo de asedio no reduce el multiplicador (§9.1). |
| K16 | Las avanzadas no tienen recintos y siguen con multiplicador 1 (§9.1). |

> **Implementado en Rust (2026-10-20, fase 4D): la ocupación.** Al conquistar una plaza (`crates/nucleo/src/nucleo/conquistas.rs`) se guardan `protegida_hasta` (`proteccionMinutos`: inatacable un día; atacarla se rechaza con `combate.plaza_protegida`) y `ocupada_hasta` (`duracionMinutos`: 90 min), cada una con su evento de fin en la cola (prioridad 2, ligados a la generación de la conquista; ninguno recurre). Mientras dura la ocupación, el paso de minuto de la plaza pasa `factorCrecimiento` y `factorRecaudacion` al `Exterior` (`felicidad_extra`, `recaudacion_extra`) y `Asentamiento::ocupado` congela el mantenimiento. `fraccionSaqueoPoblacion`, `fraccionEdificiosDanados` y `fraccionCosteReconstruccion` los aplica `combate::aplicar_conquista`. (`fraccionDanoMuralla` se aplica desde la tercera tanda de Murallas, abajo.)

> **Implementado en Rust (2026-10-22, fase de Murallas, paso 1: geometría).** Módulo `trazado::muralla` (`crates/trazado/src/muralla.rs`): `trazar_recinto(ciudad, ParametrosMuralla, nivel) -> Result<Trazo, ErrorMuralla>`, `interior_de(ciudad, celdas) -> Option<Interior>` (con `contiene` y `area`), `contiene_a(nuevo, anterior)`, `edificios_extramuros(ciudad, interior)` y `marcar_torres(celdas, interior, parametros, nivel)` (borra y vuelve a marcar), más `Recinto::desde_celdas(celdas, completo)` para pasarle el anillo a `Ciudad::set_recintos`. Tipos `ClaseCelda { Muro, Puerta, Torre }`, `CeldaMuro { celda, clase }`, `Trazo { celdas, area_encerrada, dentro, fuera, afueras_dentro }` (`puertas()` y `torres()`) y `ErrorMuralla { SinCentroUrbano, NoSePuedeCerrar, FueraDeVentana }`. Todo en la rejilla de la ventana, sin ordenar nunca un conjunto hash; no hay paridad celda a celda con el TypeScript (otro hash), se vigilan sus invariantes. `ParametrosMuralla { franja_de_ronda, paso_torres }` los construye quien parsee el bloque `MURALLA` (`Default` = 1 y `[0, 8, 5]`). Faltan, en este orden: la economía de los recintos (`asentamiento::recintos`), su estado y órdenes en el núcleo (foto nueva), el trazado con recintos reales, el combate con números y la conquista, las vistas de Unity y la calibración.

> **Implementado en Rust (2026-10-22, fase de Murallas, segunda tanda: pasos 2 y 3, la economía de los recintos y su estado en el núcleo).**
>
> **Modelo y reglas (`asentamiento::recintos`).** `ReglasMuralla` lee el bloque `MURALLA` (entra en `asentamiento::Reglas.muralla` y en la huella de las reglas; `parametros_de_trazo()` da el `ParametrosMuralla` del trazo). `Recinto { id, nivel, celdas, avance, mejora: Option<Mejora { nivel, avance }>, siguiente_celda_en, comprometido_en, completado_en }` se guarda en `Asentamiento.recintos`, del más interior al más exterior; `nivel` y `avance` son siempre los del anillo en pie (Q1) y `completado_en` se fija al cerrarse por primera vez y no se borra. Funciones puras: `coste_de_celda` (tarifa acumulada × factor de la clase), `coste_de_mejora_de_celda` (diferencia con el nivel anterior, ×factor de la clase que la celda tiene en el anillo en pie), `coste_de_trazo`, `multiplicador_defensivo` (ppm; `UNO + (bono − UNO) × celdas levantadas / (celdas × puertas)` con una sola división entera hacia abajo del recinto de referencia, `de_referencia` = el más exterior de los que alguna vez se cerraron), `nivel_del_mejor_completo` (cupo de guarnición y requisito del nivel 4), `upkeep` (Costo en milésimas: `upkeepPorCelda[nivel] × celdas en pie`, 0,02 = 20), `danar_por_conquista(recintos, fraccion)` (completos y sin mejora pierden `floor(fracción × celdas)` de avance; conserva `completado_en`; **aún sin llamador**: lo cablea el paso 5), `proyeccion` (lo que recibe la ciudad). En `Asentamiento`: `comprobar_comprometer` / `comprometer_recinto`, `comprobar_mejora` / `iniciar_mejora`, `abandonar_recinto`, `levantar_recintos` (una pasada de obra si algún recinto toca) y `avanzar_recintos`; los dos primeros no necesitan geometría (la ponen el trazo y `contiene_a`, que llama el núcleo). `asentamiento` depende de `trazado` para usar `CeldaMuro`, `ClaseCelda` y `ParametrosMuralla`.
>
> **El núcleo (`crates/nucleo/src/nucleo/murallas.rs`).** Órdenes 123-125 (arriba), foto **`BAN20`** (`codec_economia` guarda los recintos de cada plaza; `codec_ciudad` **deja de guardarlos** y la ciudad se reconstruye con la proyección derivada de la economía; **`BAN19` se rechaza** con su motivo, y una cabecera `BAN2x` distinta de `BAN20` es «no es un estado del núcleo»), `Nucleo::presupuesto_de_recinto(plaza, nivel) -> Result<Presupuesto, ErrorPresupuesto>` (el trazo, su coste, el arrabal del recinto exterior y si lo envuelve; no toca nada). La sincronía con la ciudad es `Urbe::set_recintos(proyeccion)` y solo se llama si la proyección cambia (al comprometer, al cerrarse o repararse un anillo y al abandonar; no por cada celda ni al mejorar, porque muro y torre ocupan igual). El cupo de guarnición (`Nucleo::cupo_de_guarnicion`), el requisito del nivel 4 (`bloqueos_de_ascenso`) y el mantenimiento (`coste_mantenimiento`, también el de la reserva y el de la solvencia del ascenso) leen ya los recintos reales; `Exterior.recinto_nivel` desaparece.
>
> **El reloj.** La primera celda se levanta en el instante de la orden; cada celda deja una cita `Carga::FinDeObras` (prioridad 1) a `minutosPorCelda[nivel pagado] × factorTiempoMuralla` (en ms exactos, no en minutos redondeados) y `Evento::RecintoCelda` es lo que la programa. Si al llegar la cita faltan materiales o se bajaría de la reserva (la manual del Tesorero incluida), no se programa otra y el paso de minuto reintenta la celda (Paso 8 de la cadena, tras la auto-construcción y con la misma reserva); la cadencia cuenta desde que se levantó de verdad. Ver [`Reloj_Y_Eventos.md`](Reloj_Y_Eventos.md). Al terminar una mejora, las torres se recalculan con `trazado::marcar_torres` sobre el interior que da la ciudad (`Sitios::reclasificar_torres`) y dan exactamente el trazo directo del nivel.
>
> Dudas anotadas en las filas de paridad de [`Mecanicas a desarrollar.md`](../Mecanicas%20a%20desarrollar.md). Pruebas: unitarias en `asentamiento::recintos` e integración en `crates/nucleo/tests/murallas.rs`. Lo que quedaba (el multiplicador del asedio, el daño de la conquista, la colocación con recintos reales, el dibujo y `recintos_en_pie`) lo hizo la tercera tanda, abajo; sigue pendiente el batch y la calibración.
>
> **Implementado en Rust (2026-10-22, murallas, tercera tanda: pasos 4, 5, 6 y 8).**
>
> **Combate con números (`combate::asedio`, `Nucleo::multiplicador_de_plaza`).** `resolver_asedio` recibe el multiplicador del defensor (ppm); `atacar_plaza` le pasa `recintos::multiplicador_defensivo` de la plaza asediada (el recinto de referencia: Q1, §9.1) y todo lo demás pasa [`asedio::SIN_RECINTO`] (1,0): la columna (`atacar_columna`), la avanzada (`avanzada::resolver_ataque`; el pueblo usa su `ventajaAguante`, la caravana no usa `resolver_combate`). El multiplicador no consume tirada: el asedio gasta las mismas dos de siempre, y uno a una plaza sin recintos es bit a bit el de antes. Los ejemplos del documento están en la prueba de `combate::asedio` (×2,5 con una puerta, ×1,375 con cuatro, ×1,1875 con ocho, nivel 3 y anillo entero). El equipo de asedio no lo reduce (Q7). `combate.asedio_conquista` y `combate.asedio_resistido` llevan `multiplicadorMuralla` (ppm) y `murallaCompleta` (0/1, el predicado antes de cualquier daño).
>
> **Daño de la conquista (`combate::conquista::aplicar_conquista`, `Nucleo::conquistar_plaza`).** Tras calcular el predicado `recintos::muralla_completa` (algún recinto con integridad ≥ 1; sin más consumidor que el hecho hasta el sistema 6), cada recinto completo y sin mejora en curso pierde `floor(fraccionDanoMuralla × celdas)` de `avance` y se queda sin cita; los que están en obra o en mejora no se tocan. `Conquistada` lleva `muralla_completa` y `muralla_danada`, y `asentamiento.conquistada` los publica (`murallaCompleta`, `murallaDanada`). Si hubo daño, el núcleo rehace la proyección de la ciudad (el anillo deja de estar completo: sin preferencia intramuros) y **empieza la reparación en ese mismo instante**: levanta la primera celda con el `Exterior` del nuevo dueño y deja la cita de la siguiente (si no hay materiales o se bajaría de la reserva no hay cita y el paso de minuto lo reintenta, como cualquier celda). La plaza ocupada tiene el mantenimiento congelado pero la obra sigue. Es lo mismo por los números y por el resultado de Unity: las dos rutas pasan por `conquistar_plaza`. La reparación no vuelve a anunciar el cierre (`completado_en` no se toca).
>
> **Unity y el dibujo.** `Nucleo::recintos_en_pie(plaza) -> Vec<RecintoEnPie { id, nivel, celdas }>`: de cada recinto con alguna celda en pie, su nivel físico y las celdas `[0..=avance]` con su clase (una mejora en curso no cambia nada; uno sin celdas no viaja); el `layoutVersion` es `trazado::LAYOUT_VERSION` y las `unidadesPorCelda`, `trazado::CELDA_UL`. `VistaCiudad.murallas` (ver [`Trazado_Urbano.md`](Trazado_Urbano.md) §10) dibuja las celdas en pie por clase y las planificadas aparte. **No** van todavía (Q5, 4H): el `SettlementBattleSnapshot` del ticket del asedio y el informe de inspección con `recintos {nivel, celdas, avance}`; ni la foto cambia (`BAN20` sigue: no hay estado nuevo).
>
> **Aún no:** el batch y la calibración (paso 7: `batch población` sigue sin recintos), el predicado «muralla completa» en los contadores de logros (sistema 6) y las cuatro proyecciones al Protobuf (4H). Pruebas: `asentamiento::recintos` (predicado), `combate::{asedio, conquista}` (multiplicador y daño) y, en el núcleo, `crates/nucleo/tests/murallas_asedio.rs` (asedio con y sin anillo con las mismas tiradas, mejora en curso, columna, daño y reparación con su cita, mejora y obra sin daño, `recintos_en_pie`, `VistaCiudad.murallas`, colocación con recintos reales y determinismo normal/acelerado/a trozos) y `crates/nucleo/tests/batallas.rs` (la conquista por Unity).

> **Implementado en Rust (2026-10-23, 4H T4: el ticket y el informe).** Foto `BAN21`. El ticket de un asedio congela la plaza al abrir (`combate::ciclo::Ticket.plaza`: edificios internos y, por cada recinto con alguna celda en pie, su nivel físico y solo esas celdas, con `layoutVersion` y `unidadesPorCelda`; `Nucleo::recintos_en_pie`) y se proyecta como `SettlementBattleSnapshot`; el informe de inspección de una plaza ajena trae sus recintos con alguna celda levantada (`militar::inspeccion::RecintoAvistado`: nivel, anillo entero y `avance`) como `RecintoInforme`. Pruebas: `crates/nucleo/tests/batalla_unity.rs` (el ticket con una empalizada completa y el informe de la plaza). Ver [`Batalla_Unity_Ciclo.md`](Batalla_Unity_Ciclo.md) §3.3 y su última nota.
