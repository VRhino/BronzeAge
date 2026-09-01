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
    11. [JUGADOR] progreso jugador y tropa 
    12. [VISIBILIDAD] niebla de guerra (proyeccion por audiencia, "ultimo conocido")

## 2. Movimiento de ejercitos por el mapa
Los ejércitos también se mueven por el mapa para atacar como las caravanas, con un símbolo q los identifique por ejemplo un rombo, uno por cada jugador q va en el ejército, uno detrás de otro medio superpuestos y cada rombo del color de su faccion.

## 3. Rutas Caravanas
el pathfinder de las rutas para las caravanas debe buscar evitar bosques(rodearlos) o rios(no los puede atravesar).
Cuando hay muchos caminos q pasan cerca en el mapa general debido a rutas de caravana se debería juntar para formar caminos unificados y si estos caminos para ir de A a C, está B justo en camino o cruza una zona de influencia de B, debe pasar por la ciudad B de camino a C y dejar una pequeña comisión, cuando una caravana está cruzando una una ciudad neutral o aliada no puede ser atacada

cuadno se forma un manino se evalua la proximidad con otros camnios, cada otra caravana que use ese camino le agrega 1 punto, mientras mas puntos mas grande se ven en el mapa real y atrae con mas fuerza a oras rutas para que se desvien asi sea un poco de su camino, creando caminos principales.

## 6. Murallas
mecanica de murallas: que rodee todo el espacio interno de la ciudad ocupando celdas, cada celda de muralla tiene un coste, es decir q escala mientras más celdas tenga, deja granjas y corrales fuera.

la murallas tienen mejora, el coste es de madera y piedra, al principio no se pueden apostar soldados arriba, pero con niveles mas altos si. al subir de nivel, la muralla, gana torres, puertas

> **Diseño cerrado: `Consideraciones/Murallas_Definicion.md`** (2026-08-31, revisado por consejo).
> Especificación completa — entidad `Recinto` persistida, trazo del anillo por dilatación, puertas
> geométricas y congeladas, torres, tres niveles (empalizada → piedra → adarve), coste por celda con obra
> progresiva, arrabal extramuros y ampliación de recinto. **CERO código escrito todavía.**
>
> La razón de ser está en §0: la muralla es una ventaja defensiva abrumadora, y **menos puertas benefician al
> defensor** (embudo). De ahí el eje estratégico: amurallar pronto = fortaleza barata con casi todo el
> crecimiento futuro extramuros; amurallar tarde = metrópoli cara con más frente que cubrir. Por eso entran
> ya en la primera pasada el multiplicador defensivo y el upkeep por celda: la ventaja tiene que ser difícil
> de obtener Y de mantener.
>
> Depende de §7 (calles como celdas): sin la geometría de celdas de la Etapa 6 esta mecánica no se puede
> escribir. El `EdificioTipo 'muralla'` mínimo que existe hoy (1 celda, 2000 piedra, gate de nivel 4)
> desaparece y lo sustituye el recinto.

## 7. Revamp caminos
Revamp la mecánica de colocación de edificios para tomar en cuenta los caminos como celdas y no aristas. Dado q cuando pase a de 2d a 3d el ser aristas causa problemas.

Para el espacio relativo de el asentamiento queda igual solo q se representa diferente lo q antes era 1 celda ahora es 2x2 y lo demás crece en relacion

> **Diseño cerrado y plan de ejecución: `Consideraciones/Vista_Asentamiento_Trazado_Urbano.md` → "Etapa 6"**
> (§E6.1–E6.15), 2026-08-30. Especificación completa + las cinco decisiones cerradas con el usuario + el plan
> por pasos con checkbox. **Paso 0 (línea base instrumentada en el batch) hecho; el resto pendiente.**
>
> Al medir el modelo actual antes de diseñar, el problema resultó más grande que el motivo declarado: el 51%
> de la red de calles es de **ancho cero** (aristas que corren por el muro compartido de dos edificios) y solo
> el 33% de los edificios tiene frente de calle real. Las celdas arreglan el 3D de paso; lo que arreglan de
> fondo es que la calle pase a **costar suelo**, que es la única presión capaz de producir manzanas de verdad.
>
> Es además la pieza que condiciona a las otras dos mecánicas de construcción: §6 (murallas) necesita su
> geometría de celdas, y §4 (políticas de ubicación) puntúa sobre el mismo trazado.

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

Lo que falta es una **decisión de diseño de juego**, no de arquitectura:

- **Patrón: se muestra el ÚLTIMO ESTADO CONOCIDO, no el actual** (niebla de guerra tipo RTS). Decisión ya
  tomada (2026-08-24). Evita filtrar telemetría en vivo — ves el asentamiento rival tal como estaba la última
  vez que tuviste contacto, no como está ahora.
- **Entidad `ConocimientoJugador`**: qué sabe cada jugador y desde cuándo (por asentamiento / entidad).
- **Tres fuentes de visibilidad de lo ajeno**, cada una con un parámetro sin fijar:
  1. **Espacial** — zona de influencia propia + un **radio de visualización** alrededor de tus asentamientos
     y ejércitos. *Falta el número del radio.*
  2. **Contacto** — al proponer un trueque con otro asentamiento pasas a "conocerlo". *Falta decidir si ese
     conocimiento decae con el tiempo o se congela indefinidamente en el último snapshot.*
  3. **Alianza** — un aliado ve lo que ves tú, en vivo (no "último conocido"): la alianza es cooperación
     explícita.

Cuando estos parámetros estén definidos, el backend añade el filtrado a `proyectarParaJugador` (mismo sitio
que Slice 1) y la entidad `ConocimientoJugador` al estado.
