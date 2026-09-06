# Mecánicas por desarrollar

Solo lo **pendiente**. El estado completo de todas las mecánicas del juego —las hechas, las descartadas y
estas— vive en `Consideraciones/Checklist_Mecanicas.md`, contrastado contra el código el 2026-09-06.
Cuando una entrada de aquí se cierra, se borra de este archivo y se marca allí.

## Índice

| # | Área | Mecánica | Código hoy |
|---|---|---|---|
| 1 | MOTOR | Impuestos: generación de oro por población | ✘ nada |
| 3 | CARAVANAS | Rutas de caravana avanzadas | ◐ solo el pathfinder base |
| 5 | TRUEQUE | Trueque compuesto de varios materiales | ✘ nada |
| 8 | CARAVANAS | Revamp de caravanas (carros, animales y escoltas) | ✘ nada |
| 9 | ASENTAMIENTO | Eventos de asentamiento | ✘ nada |
| 10 | WORLDGEN | Landmarks reconocibles | ✘ nada |
| 11 | JUGADOR | Progresión de Liderazgo del jugador | ✘ nada |
| 12 | VISIBILIDAD | Niebla de guerra — visión de alianza y calibración | ◐ pasos 1-3 y 5 hechos |
| 13 | POLÍTICA | La capital como decisión del jugador | ✘ hay un proxy placeholder |
| 13b | JUGADOR | El jugador como entidad situada en el mundo | ✘ nada |
| 14 | JUGADOR | Movimiento libre del jugador por el mapa | ✘ nada |
| 15 | COMERCIO | Comerciar con una plaza ajena desde su puerta | ✘ nada |

Fuera de este índice, siguen sin código pero **sin ficha propia todavía** (ver checklist): tecnología y el
árbol de desarrollo propio, los 4 gremios, el exilio, el Attack Timer, el spawn aleatorio de onboarding, los
materiales exóticos, el ciclo de servidor de 12 meses y la curva de onboarding gradual.

> **Cerradas y retiradas de este índice:**
> - **Movimiento de ejércitos por el mapa — HECHO (2026-09-04).** Reglas en
>   `Docs/Game/5_Sistema_Militar_y_Combate.md` §5.11-5.13 y en el glosario; decisiones, plan y mediciones en
>   `Consideraciones/Movimiento_Ejercitos_Definicion.md`. Se llevó por delante dos entradas más: la **escolta
>   de caravanas** (Doc 3.10, que llevaba años marcada como "no modelada") y la **fuente espacial** de la
>   niebla de guerra, que estaba bloqueada por faltar el radio de visión.
> - **Políticas de ubicación de construcción (antes §4) — HECHO.** Las cuatro ordenanzas de perfil de trazado
>   del Maestro de Obras (`postura_defensiva` / `arterias_comerciales` / `barrios_gremiales` /
>   `plazas_mayores`) más `lineas_produccion` son exactamente esto: el jugador orienta *cómo* se distribuye la
>   ciudad. Elegir la parcela concreta de un edificio sigue siendo imposible **por invariante de diseño**, no
>   por falta de trabajo. Lo único que queda es darles coste/beneficio mecánico propio, anotado ya en
>   `constants.ts` y recogido en el checklist.

## 1. Impuestos — generación de oro por población

El oro hoy entra por **dos** vías: la `mina` (`produccionBaseOro: 4`) y las comisiones de comercio
(`COMISION`). Un asentamiento sin mina cerca y sin comercio activo no genera oro, pase lo que pase con su
población.

Falta la tercera: recaudación en el asentamiento **según cuánta población tiene y de qué clase**. Pesants,
Artesanos y Nobleza no deberían rendir lo mismo. Es también lo que daría sentido económico a hacer crecer un
asentamiento más allá de su producción de materiales.

## 3. Rutas de caravana avanzadas

El pathfinder base ya está (`world/rutas.ts`, `engine/caminos.ts`): calcula por coste de terreno, el agua es
infranqueable, y el camino generado da bonus de velocidad a las caravanas que lo siguen. Falta todo lo demás.

El pathfinder de las rutas debe buscar evitar bosques (rodearlos) o ríos (no los puede atravesar).

Cuando hay muchos caminos que pasan cerca en el mapa general debido a rutas de caravana se deberían juntar
para formar caminos unificados; y si estos caminos, para ir de A a C, tienen a B justo en el camino o cruzan
la zona de influencia de B, deben pasar por la ciudad B de camino a C y dejar una pequeña comisión. Cuando una
caravana está cruzando una ciudad neutral o aliada no puede ser atacada.

Cuando se forma un camino se evalúa la proximidad con otros caminos; cada otra caravana que use ese camino le
agrega 1 punto. Mientras más puntos, más grande se ve en el mapa real y atrae con más fuerza a otras rutas
para que se desvíen, aunque sea un poco, de su camino — creando caminos principales.

## 5. Trueque compuesto de varios materiales

`AcuerdoTrueque` es hoy un intercambio de **un** recurso por **un** recurso (`recursoA` / `recursoB`). Un
acuerdo debería poder llevar varios materiales por lado.

## 8. Revamp de caravanas

Cuando construyes tu primer mercado te cuesta 50 de oro y te da un carro de caravana básico y un animal de
arrastre.

Se pueden comprar animales de arrastre (cuestan oro) u obtener vía cría. Están los bueyes, lentos pero llevan
mucha carga y son más económicos; los caballos, más rápidos pero menos carga; y los camellos, más lentos que
los caballos pero más rápidos que los bueyes, y no se mueren en los desiertos.

Los carros hay de varios tipos: el más básico se fabrica en el mercado por 20 de madera, y en la carpintería
se pueden fabricar mejores.

El mercado mantiene el cap de caravanas activas. Una caravana puede tener muchos carros asignados, y cada
carro, dependiendo de su tipo, puede tener uno o varios animales asignados.

**Una caravana se arma con TRES partes** (a petición del usuario, 2026-09-06):

| Parte | Qué aporta |
|---|---|
| **Carros** | Capacidad de carga y, el más lento, la velocidad de toda la caravana |
| **Animales de carga** | Qué carros pueden moverse y a qué ritmo — bueyes, caballos, camellos |
| **Escuadrones de escolta** | Defensa propia, sin héroe |

**La escolta por defecto, sin héroe** — es la tercera pata y la que no existía en el enunciado original.
Según el **nivel del asentamiento y de su Mercado**, se pueden asignar escuadrones a una caravana como
escolta permanente, sin que ningún jugador los acompañe. Es distinto de la escolta por ejército (Doc 5.13.3),
que exige a un jugador marchando con ella: aquí la caravana se defiende sola.

Lo que hay que decidir al abordarlo:

- **Cuántos escuadrones por nivel**, y si el cupo lo marca el nivel del asentamiento, el del Mercado, o ambos.
- **De quién son esos escuadrones.** Hoy todo escuadrón pertenece a un Jugador (Doc 5.4) y cuenta contra su
  Liderazgo. Una escolta sin héroe rompe eso: o son tropa del asentamiento —una figura nueva— o son de un
  jugador que las cede y las tiene inmovilizadas mientras la caravana viaja.
- **Cómo se relaciona con la defensa base** de una caravana sin escolta (Doc 3.10), que ya existe y que
  precisamente está calibrada para frenar a un jugador solo. Con escoltas reales, esa defensa base pasa a ser
  el caso "sin ninguna de las tres partes".
- **Qué pasa con la escolta si la caravana es capturada**: se pierde con ella, o vuelve a casa.

Una caravana, mientras más carros tenga, dura más en prepararse; aparte es tan rápida como su carro más lento.

Tiene que haber una interfaz de preparación de caravana y de planificación, es decir, que se lance no solo
automáticamente sino que se dejen planificadas para cierta hora.

Las caravanas pequeñas no llaman la atención, por ende no aparecen en el mapa general: tiene que haber un
jugador cerca para que le aparezca.

Las caravanas grandes (muchos carros, mucho contenido) llaman mucho la atención desde que se empiezan a
preparar, al punto de que jugadores de asentamientos hasta cierta distancia pueden ver que se está preparando,
para salir a interceptarla. Así incentivamos el conflicto.

*Lo que ya existe y hay que respetar al implementar esto:* capacidad y velocidad fijas por categoría
(`CARAVANA_CATALOGO`), flota propia construible (`construirCaravanaComercial`), cupo ampliable por política
(`cupo_caravana_extra`), cooldown de creación (`CARAVANA_COOLDOWN`) y asignación automática por scoring
(`ASIGNACION_CARAVANA`) — esta última es el sustituto de Fase 0 de la carga manual, y es justo lo que la
interfaz de preparación vendría a reemplazar.

## 9. Eventos de asentamiento

Los asentamientos tienen eventos propios como, por ejemplo, ser sitiados por bandidos (y los jugadores tienen
cierta cantidad de horas para formular la defensa y jugar la defensa). Pensar en otro tipo de eventos que
mantengan entretenido el juego.

*Base ya disponible:* los campamentos de bandidos existen y atacan caravanas cada tick
(`engine/bandidos.ts`), pero nunca asedian un asentamiento.

## 10. Landmarks

Hay que añadir la creación de landmarks reconocibles en el mapa, de modo que el jugador que lo explora pueda
reconocer por dónde va sin perderse del todo — algo que ayude a reconocer qué cosas están cerca de qué.

## 11. Progresión de Liderazgo del jugador

`Jugador` tiene hoy `liderazgoBase` y nada más. El propio código lo anota: *"el efectivo es base + progresión,
pero la progresión todavía no está diseñada, así que hoy coinciden"*. Un jugador sin registro usa
`LIDERAZGO.base`.

Falta decidir qué hace subir el liderazgo (combatir, ganar, tiempo al mando, cargo militar…) y con qué curva.
La progresión de la **tropa** sí existe: la veteranía sube el poder del mismo escuadrón sin cambiarle nunca la
identidad (Doc 5.8).

## 12. Niebla de guerra — lo que queda

El grueso está hecho (2026-09-04/05): la vista, la memoria por Facción, su proyección, el pintado en el
cliente de jugador y las fronteras ajenas. Registro completo en
`Consideraciones/Niebla_De_Guerra_Definicion.md`; reglas de juego en Doc 5.12.7/5.12.8.

Queda:

- **Paso 4 — visión compartida por alianza**: en vivo, no "último conocido", porque la alianza es cooperación
  explícita.
- **Paso 6 — calibración** del margen de asentamiento (`VISION.margenAsentamiento`) y del tamaño de celda de
  la rejilla de exploración.

## 13. La capital como decisión del jugador

La capital tiene que ser una **decisión consciente de los jugadores**, no algo heredado.

Hoy no lo es. `encontrarCapital` (`engine/mantenimiento.ts`) devuelve **el asentamiento vivo más antiguo de la
Facción** —literalmente `sort((a,b) => a.fundadoEn - b.fundadoEn)[0]`— y el propio código lo marca como
*"placeholder = proxy de capital"*. De ahí salen tres problemas:

1. **No se elige, se hereda.** El primer asentamiento es capital para siempre, aunque acabe siendo un
   villorrio y la Facción tenga su verdadero centro de poder en otra parte. No hay forma de trasladarla.
2. **El Palacio no pinta nada.** Existe el edificio `palacio` (Doc 4.2.1) y la capital lo ignora por completo.
   Lo natural sería que la capital fuera *donde está el Palacio*, o que designarla lo exigiera.
3. **Y sí tiene efecto mecánico real**, así que no es cosmético: el mantenimiento de cada asentamiento escala
   con su distancia a la capital — `factorDistancia = 1 + min(1, dist/400) × (2-1)`, o sea ×1 en la capital y
   hasta **×2 a distancia 400**, topado a partir de ahí. Es el mecanismo anti-snowball de "cohesión"
   (Fase_0_5 §5.1).

Dos cosas que hay que decidir con ello:

- **Cómo se designa y qué cuesta trasladarla.** Si mover la capital es gratis, el jugador la reubica cada vez
  que conquista algo y el factor de distancia deja de morder.
- **El tope a 400 desactiva el anti-snowball.** Más allá de esa distancia no hay penalización adicional: un
  imperio de punta a punta del mapa paga lo mismo que uno moderadamente disperso. Con provincias de radio ~76
  (ver la escala del mundo), 400 son ~5 provincias — o sea que el "radio cómodo" de un reino ya está fijado en
  el código sin que nadie lo decidiera.

## 13b. El jugador como entidad en el mundo

El jugador como partícipe del mundo, no como ente volador superior. Es decir: el jugador tiene una ubicación
en el mundo. Nace en mundo abierto, en un lugar aleatorio, cuando se une por primera vez al juego; luego funda
un asentamiento, y al fundarlo entra en él; luego puede salir al mundo abierto y moverse por él, entrar a otros
asentamientos y demás. Puede ver el mapa superior de lo conocido (ya aplicado) desde cualquier parte abriendo
el mapa (tecla `,` o botón en alguna parte superior o inferior), pero nunca puede ver ningún asentamiento en el
que no esté físicamente dentro. Se le puede prohibir la entrada a jugadores neutrales o enemigos a
asentamientos (el gobernador).

*Estado en código:* `Jugador` solo tiene `id` y `liderazgoBase` — no hay posición. El spawn aleatorio de
onboarding depende de esta entrada.

> **DISEÑO CERRADO (2026-09-06), pendiente de implementar.** Treinta y seis decisiones con el usuario en once rondas, con las reglas ya escritas en `Docs/Game/`,
> representación en el motor, plan de 10 pasos, invariantes y puntos abiertos en
> `Consideraciones/Jugador_Situado_Definicion.md`. Se diseñó **junto con §14**, que es su verbo, y sacó §15 a
> mecánica propia.
>
> Lo esencial: la regla "solo ves donde estás" es de visión **y** de acción; en ciudad ajena solo se ve la
> capa pública; salir es `movilizarEjercito` con 0 escuadrones permitidos y carga elegida; la columna se
> disuelve solo en tu residencia y se aparca a la puerta en cualquier otra; al desconectarse el viajero
> desaparece en el punto donde quedó; y el movimiento es **clic a destino** —miniatura estilo Total War— así
> que Fase 0 y Fase 1 comparten modelo y no hace falta netcode (§7 de ese documento).
>
> **Segunda ronda:** nada se dispara por proximidad, la proximidad abre un **menú**. Tres radios: vista
> 150/80, **inspección 40 con aviso al observado**, encuentro 15. La persecución pasa a ser un estado con
> debuff de derrota. Eso revisa tres reglas de Doc 5.12 (§1.1c de ese documento).
>
> **Tercera, cuarta y quinta ronda:** la línea entre **columna personal** y **ejército** la marca de dónde
> viene la columna, no cuántos van dentro (§1.1f); un ejército solo nace en una plaza y nunca se queda vacío
> en campo abierto; y **el héroe combate**, con el poder de una unidad de élite derivado del catálogo — lo
> que obliga a reescribir Doc 5.1 ("el jugador nunca combate individualmente"). **Sin bloqueantes.**

## 14. Movimiento libre del jugador

> **Se diseñó junto con §13b** — es el verbo de esa mecánica y no tiene sujeto sin ella. Registro completo en
> `Consideraciones/Jugador_Situado_Definicion.md`. La pregunta que esta ficha dejaba abierta ("ver si se puede
> mover esta lógica a que sea 100% cliente") está **contestada: no.** La posición es T3 por el criterio del
> propio `Docs/Arquitectura/9_Reglas_vs_Simulacion.md` —muta cada tick y decide el resultado de otros
> jugadores (intercepciones, encuentros, asedios)—, así que es del servidor. Del cliente son la interpolación
> entre ticks, la previsualización de la ruta y la cámara.

Hay que implementar el movimiento libre de los jugadores en el mapa general. La salida del asentamiento ya
está aplicada (como movilización de ejército, `movilizarEjercito`), pero en el caso de un jugador que sale
solo sin tropas, este debería moverse más rápido que uno que lleva sus tropas y consumir casi nada de trigo al
moverse por el mundo. Usando el clic sobre el mapa en el cliente — aquí estaría la base para poder hacerlo, o
para ver si se puede mover esta lógica a que sea 100% cliente, a analizar.

## 15. Comerciar con una plaza ajena desde su puerta

Salió del diseño de §13b (`Consideraciones/Jugador_Situado_Definicion.md` §1.1d) y se sacó a ficha propia
**por decisión del usuario**: era una de las cuatro opciones del menú de asentamiento —*entrar*, *asediar*,
*comerciar*, *consultar*— y pesa más que las otras tres juntas. Las otras tres son un botón sobre algo que ya
existe; esta es un sistema económico nuevo.

Lo que se pidió: estando en el radio de puerta de un asentamiento que no es tuyo, poder abrir **su interfaz
de comercio** — ver si tiene órdenes de compra o de venta activas, y venderle o comprarle contra ellas.

*Estado en código:* nada. El comercio de hoy es **intra-Facción y desde asentamiento propio**: las órdenes de
mercado (`colocarOrdenMercado`), el trueque (`proponerTrueque`) y las caravanas (`crearCaravana`) exigen
ciudadanía o residencia, y se ejecutan sobre plazas de la propia Facción. Un forastero delante de una puerta
no tiene hoy ninguna vía de comerciar.

Lo que hay que decidir cuando se aborde:

- **Qué puede comprar y vender un extranjero**, y contra qué. ¿Solo las órdenes ya publicadas del mercado, o
  puede publicar las suyas?
- **La comisión.** `COMISION` ya distingue misma Facción de fuera, así que hay dónde apoyarse, pero falta el
  caso "ni ciudadano ni aliado, plantado en la puerta".
- **Con qué se paga y dónde va lo comprado.** El forastero lleva un carro con capacidad (Doc 5.13), así que
  su almacén es ese carro — comprar más de lo que cabe no puede ser posible.
- **Si el Gobernador puede cerrar el mercado sin cerrar la puerta**, que es la palanca diplomática obvia:
  dejar pasar pero no vender, o vender solo a aliados.
- **Qué relación tiene con el trueque entre asentamientos**, que es la vía existente y funciona entre plazas,
  no entre una persona y una plaza.

Mientras esto no exista, la opción *Comerciar* no aparece en el menú de asentamiento: las otras tres sí.
