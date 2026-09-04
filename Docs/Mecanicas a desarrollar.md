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

> **DISEÑO CERRADO Y EN EJECUCIÓN (2026-09-04).** El registro de decisiones y el plan de los 6 pasos están
> en `Consideraciones/Niebla_De_Guerra_Definicion.md`; las reglas de juego, en Doc 5.12.7. Esta entrada se
> retirará del índice cuando la mecánica esté hecha.

El jugador debe acabar viendo el mundo en **tres estados** (así lo planteó el usuario):

1. **No lo veo y nunca lo he visto**: esa zona está tapada, terreno incluido.
2. **No lo veo pero lo vi antes**: se guarda lo último que vi, con un filtro oscuro, como si fuera de noche.
3. **Lo estoy viendo**: tal cual. Al dejar de verlo, cae al estado anterior.

Y sale de **una sola regla**: *lo que alcanzas a ver este tick queda grabado*. Ver algo ES conocerlo — el
contacto comercial no es una vía aparte, es otra forma de verlo un instante.

**Hecho (Paso 1, 2026-09-04): la VISTA**, o sea el estado 3. `proyectarParaJugador` proyecta lo ajeno que se
ve ahora mismo, siempre redactado: `ejercitosAvistados` (desde 2026-09-03) y `asentamientosAvistados` con la
ficha del rival. Los dos radios viven en `VISION` (`ejercito: 150`, `margenAsentamiento: 60`) y la mecánica de
ejércitos desbloqueó el primero, que era el número que llevaba tiempo faltando.

**Hecho (Paso 2, 2026-09-04): la MEMORIA se graba.** `memoriaPorFaccion` en el estado guarda, por **Facción**
y no por jugador, qué terreno ha llegado a ver (rejilla de celdas de 25, `engine/exploracion.ts`) y la última
ficha de cada plaza ajena que vio, con el instante (`engine/memoria.ts`). Se escribe al final del tick, con
los ejércitos ya movidos, y **solo crece**. Migración de snapshot v6 -> v7: una partida vieja empieza a
recordar desde el primer tick que corra con la mecánica.

**Hecho (Paso 3, 2026-09-04): la memoria se proyecta.** `ProyeccionJugador` gana `asentamientosConocidos`
—la última foto de cada plaza que se vio y ya no se ve, con su `conocidoEn`— y `exploracion`, la máscara de
celdas con la geometría necesaria para descifrarla. Lo que se ve en vivo gana a lo recordado: cada plaza sale
en una lista o en la otra, nunca en las dos.

**Hecho (Paso 5, 2026-09-04): se pinta.** El cliente de jugador (`BronzeAgeClient`) tapa el terreno no
explorado y pinta lo recordado con filtro oscuro; el de administración no aplica ninguna máscara, porque es
herramienta de operación y no un jugador. El servidor manda dos máscaras —`celdas` y `visibles`— y de ellas
salen los tres estados sin que el cliente sepa una sola regla del juego.

**Lo que queda**: la **visión compartida por alianza** (Paso 4) — en vivo, no "último conocido", porque la
alianza es cooperación explícita— y la **calibración** del margen y del tamaño de celda (Paso 6).

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
