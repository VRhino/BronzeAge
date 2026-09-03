+ Feature nuevas(indice)
    2. [GUERRA] movimiento de ejercitos por el mapa
    8. [CARAVANAS] mecanica caravanas.
    1. [MOTOR] mecanicas de taxes en los asentamientos, que es la generacion de oro en base a la poblacion y tipo de poblacion que vive en el asentamiento
    3. [CARAVANAS] Rutas Caravanas.
    4. [POLITICAS]politicas de ubicacion de construccion.
    5. [TRUEQUE]trueque compuesto de varios materiales
    9. [ASENTAMIENTO] Eventos de asentamiento
    10. [WORLDGEN] creacion de landmarks reconocibles (3d)
    11. [JUGADOR] progreso jugador y tropa 
    12. [VISIBILIDAD] niebla de guerra (proyeccion por audiencia, "ultimo conocido")
    13. [POLITICA] la capital como decision del jugador

## 2. Movimiento de ejercitos por el mapa
Los ejércitos también se mueven por el mapa para atacar como las caravanas, con un símbolo q los identifique por ejemplo un rombo, uno por cada jugador q va en el ejército, uno detrás de otro medio superpuestos y cada rombo del color de su faccion.

Las tropas son del JUGADOR, no del asentamiento — están apostadas ahí. En marcha no comen del granero: el
ejército lleva su propio carro de suministros, y sin comida hay deserción y baja moral. El jugador sale solo
(eligiendo qué tropas se lleva) o como parte de un ejército de varios jugadores que se mueve como una sola
entidad. En ambos casos lo limita su **liderazgo**: cada tipo de tropa tiene un coste, y solo puede sacar lo
que quepa en su valor.

Se le pueden **adjuntar caravanas** para cargar más trigo del que suman los carros de sus jugadores, y esas
caravanas pueden ir cargadas de mercancía — lo que resuelve de paso la **escolta de caravanas**, que estaba
pendiente. Cada tropa tiene **velocidad propia** y el ejército va al ritmo de la más lenta, así que escoltar
frena y no se puede escoltar y depredar a la vez. Una marcha se puede **cancelar**, lo que dispara la vuelta.

> **Diseño cerrado (2026-09-01/02, 17 decisiones con el usuario + revisión por consejo, CERO código escrito).**
> Sin puntos bloqueando el arranque.
>
> **Las reglas son canon y viven en `Docs/Game/`**: `5_Sistema_Militar_y_Combate.md` §5.11 (Liderazgo),
> §5.12 (Ejércitos y movimiento por el mapa) y §5.13 (Suministro en campaña), más las entradas
> *Escuadrón/Tropa/Unidad*, *Ejército*, *Guarnición*, *Huérfano* y *Liderazgo* del glosario. Doc 5.4 recoge
> que los escuadrones son del Jugador, Doc 5.10 aclara que el movimiento en el mapa SÍ es Fase 0, y Doc 3.10
> recoge la escolta de caravanas, que esta mecánica resuelve.
>
> **El cierre de decisiones y el plan de ejecución** (13 pasos con checkbox) están en
> `Consideraciones/Movimiento_Ejercitos_Definicion.md`, junto con la **revisión por consejo** (§9): cinco
> asesores + revisión cruzada, misma metodología que murallas. Encontró 9 hallazgos, 5 de los cuales
> cambiaron reglas del juego (mulas de suministro, intercepción imposible por velocidad plana, guerra suma
> negativa, cancelar marcha, jugador huérfano) y 4 el orden del plan.
>
> **Decisión abierta que salió al preguntar por qué se usa RNG** (§1.4 del doc de ejecución, regla en Doc
> 5.2.5): la varianza de combate de ±15% nunca estuvo escrita en ningún documento, solo en el código, y
> significa que un atacante necesita un 35% más de poder para tener la victoria asegurada. Esta mecánica
> multiplica el coste de una mala tirada, y la niebla de guerra (§12) es una fuente de incertidumbre mejor
> —reducible jugando bien— así que se replantea bajarla o retirarla. No bloquea el arranque.
>
> De la revisión salió además un dato que NO es de esta mecánica: **el ejército no es el problema del trigo**.
> Un asentamiento nivel 1 a tope de población come 30/tick y una Granja nivel 1 produce 15 — nace en déficit
> estructural, y en nivel 3 harían falta ~13 Granjas nivel 4. Es el cuello de botella de nivel 3 ya conocido.
> Ver §9.4.
>
> La mecánica trae dos cosas que no estaban en el enunciado: la entidad **`Jugador`** (hoy inexistente en el
> motor; `engine/combate.ts:143` ya la daba por pendiente "si llega a necesitar un propósito propio" — el
> liderazgo es ese propósito) y el **liderazgo** como primer eje de progresión personal, lo que abre
> parcialmente §11.
>
> El movimiento en sí sale casi gratis: `calcularRuta` + `avanzarPosicionEnRuta` ya mueven caravanas, y
> `resolverCombate` ya recibe tropas planas sin saber de asentamientos. Lo caro es la propiedad de la tropa —
> se va de verdad del asentamiento, y eso hace que "tu ciudad queda desnuda" se cumpla por construcción.
>
> Depende de §12 (niebla de guerra) solo para ver ejércitos AJENOS; los propios no la necesitan, así que no
> bloquea.

## 3. Rutas Caravanas
el pathfinder de las rutas para las caravanas debe buscar evitar bosques(rodearlos) o rios(no los puede atravesar).
Cuando hay muchos caminos q pasan cerca en el mapa general debido a rutas de caravana se debería juntar para formar caminos unificados y si estos caminos para ir de A a C, está B justo en camino o cruza una zona de influencia de B, debe pasar por la ciudad B de camino a C y dejar una pequeña comisión, cuando una caravana está cruzando una una ciudad neutral o aliada no puede ser atacada

cuadno se forma un manino se evalua la proximidad con otros camnios, cada otra caravana que use ese camino le agrega 1 punto, mientras mas puntos mas grande se ven en el mapa real y atrae con mas fuerza a oras rutas para que se desvien asi sea un poco de su camino, creando caminos principales.

## 8. Revamp caravanas
Cuando construyes tu primer mercado te cuesta 50 de oro y te da un carro de caravana de básica y un animal de arrastre

Se pueden comprar animales de arrastre (cuestan oro) o obtener vía cria, están bueyes lentos pero llevan mucha carga y son más económicos, caballos, son más rápidos pero menos carga y los camellos, más lentos q los caballos pero más rápidos q los bueyes y no se mueren en los desiertos.

Los carros hay de varios tipos, el más básico se fabrica en el mercado por 20 de madera, y en la carpintería se pueden fabricar mejores.

El mercado mantiene el cap de caravanas activas, una caravana puede tener muchos carros asignados, y cada carro dependiendo de cada uno (tipo) puede tener un animal asignado muchos.

Una caravana mientras más carros tenga, dura más en prepararse, aparte es tan rápida como su carro más lento.

Tiene q haber una interfaz de preparación de caravana, y planificación de caravana, es decir q se lance no solo automáticamente sino q se deje planificadas para cierta hora.

Las caravanas pequeñas no llaman atención por ende no aparecen en el mapa general, tiene un jugador estar cerca para q le aparezca,

Las caravanas grandes(muchos carros, mucho contenido)  llaman mucho la atención, desde q se empiezan a preparar, al punto q jugadores de asentamientos hasta cierta distancia pueden ver q se está preparando para salir a interceptarla, así incentivamos al conflicto.

## 9. Eventos de asentamiento
La asentamientos tienen eventos propios como por ejemplo ser sitiados por bandidos ( y los jugadores tienen cierta cantidad de horas para fórmular la defensa y jugar la defensa), pensar en otro tipo de eventos q mantengam entretenido en juego

## 10. landmarks
hay que añadir la creacion de landmarks reconocibles en el mapa, de modo que el jugador que lo explora pueda reconocer por donde va sin perderse del todo algo que atua a recocer que cosas estan cerca de que cosas

## 12. Niebla de guerra ("último conocido")

Viene del plan de arquitectura (era "C4 Slice 2"). La parte de infraestructura ya está: la proyección por
audiencia (`GET /jugador/partidas/:gameId`, `proyectarParaJugador` en `session/proyecciones/jugador.ts`) — hoy
en "Slice 1", deliberadamente conservador: el jugador ve su Facción completa y de las rivales **solo los
metadatos ya públicos en la ficción** (nombre, nivel, reputación, Rey/Embajador, relaciones diplomáticas).
Cero telemetría en vivo de un rival.

**La fuente espacial ya está implementada para EJÉRCITOS** (2026-09-03, Doc 5.12.7): `proyectarParaJugador`
proyecta en `ejercitosAvistados` los ejércitos ajenos que caen en tu zona de influencia o dentro de
`LOGISTICA.radioVisionEjercito` (150) de uno de los tuyos, redactados a posición, Facción y nº de
participantes. La mecánica de ejércitos desbloqueó ese radio, que era justo el número que faltaba.

Lo que queda es **decisión de diseño de juego**, no de arquitectura:

- **Patrón: se muestra el ÚLTIMO ESTADO CONOCIDO, no el actual** (niebla de guerra tipo RTS). Decisión ya
  tomada (2026-08-24). Evita filtrar telemetría en vivo — ves el asentamiento rival tal como estaba la última
  vez que tuviste contacto, no como está ahora. **Es lo que hoy NO hace la proyección de ejércitos**: sin
  memoria, un ejército rival aparece y desaparece del mapa según entra y sale de tu vista.
- **Entidad `ConocimientoJugador`**: qué sabe cada jugador y desde cuándo (por asentamiento / entidad). Es la
  pieza que falta para el punto anterior.
- **Las tres fuentes de visibilidad de lo ajeno**:
  1. **Espacial** — hecha para ejércitos (arriba). *Falta aplicarla a los ASENTAMIENTOS rivales, que hoy no
     se proyectan en absoluto, y decidir si el radio de un asentamiento es su zona de influencia o algo mayor.*
  2. **Contacto** — al proponer un trueque con otro asentamiento pasas a "conocerlo". *Falta decidir si ese
     conocimiento decae con el tiempo o se congela indefinidamente en el último snapshot.*
  3. **Alianza** — un aliado ve lo que ves tú, en vivo (no "último conocido"): la alianza es cooperación
     explícita.

Cuando estos parámetros estén definidos, el backend extiende el filtrado en `proyectarParaJugador` (mismo
sitio) y añade la entidad `ConocimientoJugador` al estado.

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
