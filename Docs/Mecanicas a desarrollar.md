+ Feature nuevas(indice)
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

> **Movimiento de ejércitos por el mapa — HECHO (2026-09-04), retirado de este índice.** Las reglas viven en
> `Docs/Game/5_Sistema_Militar_y_Combate.md` §5.11-5.13 y en el glosario; el registro de decisiones, el plan
> de ejecución y las mediciones, en `Consideraciones/Movimiento_Ejercitos_Definicion.md`.
>
> Se llevó por delante dos entradas más de esta lista: la **escolta de caravanas** (Doc 3.10, que llevaba
> años marcada como "no modelada") y la **fuente espacial** de la niebla de guerra (§12), que estaba bloqueada
> por faltar el radio de visión.

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
