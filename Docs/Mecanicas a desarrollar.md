+ Feature nuevas(indice)
    7. [CONSTRUCCION] Revamp caminos
    6. [CONSTRUCCION] murallas
    2. [GUERRA] movimiento de ejercitos por el mapa
    8. [CARAVANAS] mecanica caravanas.
    1. [MOTOR] mecanicas de taxes en los asentamientos, que es la generacion de oro en base a la poblacion y tipo de poblacion que vive en el asentamiento
    3. [CARAVANAS] Rutas Caravanas.
    4. [POLITICAS]politicas de ubicacion de construccion.
    5. [TRUEQUE]trueque compuesto de varios materiales
    9. [ASENTAMIENTO] Eventos de asentamiento
    10. [WORLDGEN] creacion de landmarks reconocibles (3d)

## 2. Movimiento de ejercitos por el mapa
Los ejércitos también se mueven por el mapa para atacar como las caravanas, con un símbolo q los identifique por ejemplo un rombo, uno por cada jugador q va en el ejército, uno detrás de otro medio superpuestos y cada rombo del color de su faccion.

## 3. Rutas Caravanas
el pathfinder de las rutas para las caravanas debe buscar evitar bosques(rodearlos) o rios(no los puede atravesar).
Cuando hay muchos caminos q pasan cerca en el mapa general debido a rutas de caravana se debería juntar para formar caminos unificados y si estos caminos para ir de A a C, está B justo en camino o cruza una zona de influencia de B, debe pasar por la ciudad B de camino a C y dejar una pequeña comisión, cuando una caravana está cruzando una una ciudad neutral o aliada no puede ser atacada

cuadno se forma un manino se evalua la proximidad con otros camnios, cada otra caravana que use ese camino le agrega 1 punto, mientras mas puntos mas grande se ven en el mapa real y atrae con mas fuerza a oras rutas para que se desvien asi sea un poco de su camino, creando caminos principales.

## 6. Murallas
mecanica de murallas: que rodee todo el espacio interno de la ciudad ocupando celdas, cada celda de muralla tiene un coste, es decir q escala mientras más celdas tenga, deja granjas y corrales fuera.

la murallas tienen mejora, el coste es de madera y piedra, al principio no se pueden apostar soldados arriba, pero con niveles mas altos si. al subir de nivel, la muralla, gana torres, puertas

## 7. Revamp caminos
Revamp la mecánica de colocación de edificios para tomar en cuenta los caminos como celdas y no aristas. Dado q cuando pase a de 2d a 3d el ser aristas causa problemas.

Para el espacio relativo de el asentamiento queda igual solo q se representa diferente lo q antes era 1 celda ahora es 2x2 y lo demás crece en relacion

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