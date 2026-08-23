#### Notas:

separa el generador de mundo del motor general del juego
# General
[resuelto]tendriamos 4 partes separadas en el codigo incluso en carpetas aparte con 0 acoplamiento (motor/interfaz/generador de mundo/arnes de pruebas[aqui creo q no se puede evitar el acoplamiento])

## Generador de mundos
- [resuelto] modificar la tasa de aparicion de bosques: es super necesario 
- [resuelto]que el generador de mundo sea mas completo ya no solo genereun mapa plano sin relieve ni rios a uno con relieve rios y biomas y que la aparicion de los nodos de recursos sea un poco mas relista, los metales en montañas como por ejemplo
- [resuelto]agregar rios, e identificar que ventajas trae tenerlos, fertilidad y demas
+ agregar nodos de comoditis, fertilidad de uvas(para vino), oliva(para aceite), arcilla(para pottery) 
- [resuelto] logica de caminos, moviemiento en el mapa con la elevacion y demas factores, chokepoints y demas

## Engine
+ [RESUELTO]Revisar si los mercados se construyen. 
+ [RESUELTO]Liberar la creación de facciones para q se puedan crear x
+ Pathfinder caravanas, priorizar en la creación de la ruta atravesar asentamientos aliados y neutrales, si están en la ruta son desviarse demasiado, esto crea puntos seguros de camino.
+ modificar el auto construir para que tome en cuenta:
    + lineas de produccion: ubicacion y proximidad entre extractores y transformadores, y almacenes
    + ubicacion de construccion de viviendas, lo mas agrupadas posibles con 2 opciones: barrios(agrupados por clusters)(politica del maestro de obras) o siempre lo mas cerca del centro urbano posible(por defecto) 
+ agregar las primeras versions de cria
+ Crear una ia para que juegue dentro del juego
+ Cambiar la mecánica de la simulación a una versión por tiempo y no por tics para jugar

## Interfaz:
+ [resuelto]agregar una nueva pestaña que sea comercio y dejar toda la info mas la acciones de comercio en esa pestaña

## Prueba multijugador
+ separar el motor a un backend y una interfaz simplificada para un jugador y la interfaz completa para el GM

# Dudas a futuro:
+ como afecta el ataque y saqueo de un nodo en sus construcciones: sons destruibles en el ataque, o solo reciben daño y se reparan como en total war.
+ la idea es que timing de los asentamientos es que sea lento que le de tiempo a los jugadores de luchar, crear comercio alianzas y jugar el juego en general mientras se este crece solo lentamente influenciado por las decisiones de los jugadores. los nodos de metales se pueden agotar y luego llenar cada cierto tiempo para generar una escala de tiempo


# Notas simulacion:
------- revision 1
+ [resuelto]no se muestra la produccion de los recursos intermedios
+ [resuelto]en la pestaña de jugador se debe mostrar los escuadrones reclutados por el y estado actual de dichos escuadrones
+ [resuelto]en la interfaz mover el registro general de sitio a un punto mas visible
+ [resuelto]hacer un control mas completo de las caravanas, no pueden aparecer en asentamientos al azar si salen del asentamiento A al asentamiento B, y llega al asentamiento B, no puede aparecer magicamente en el asentamiento A, tiene q moverse hasta el asentamiento A de vuelta ya sea con carga o sin ella, asi como si se construyo en el asentamiento A, no puede aparecer magicamente en asentamiento B, tiene que ir hasta el para trae cosas.
+ [resuelto]en al pestaña comercio mostrar mas informacion de las caravanas activas y en transito, punto de partida, destino, carga, escolta, % del viaje completado.
+ [resuleto]revisar si cuando una caravana termina un viaje y entrega materiales se actualiza correctamente en el trueque activo.
+ [resuelto]las caravanas no se destruyen al llegar a su destino son reutilizables.
------- revision 2
+ Asentamientos;
    + [resuelto]poder renombra asentamientos
    + [resuelto]tener un segmento en la interfaz de asentamiento q sea almacen donde se puedan calibrar diferentes cosas
        + [enRevision]reserva de recursos(es decir cuantos recursos reservar para cosas manuales, que el automatico no pueda tomar de esa reserva), seria un slider por producto de 0 a 999
        + [resuelto]solo se muestre activo si se tiene un tesorero
+ Caravanas:
    + [resuelto]las caravanas no son identificables, vamos a cambiarles lo q las identifica en el mapa de un punto blanco, a otra forma pero del color del asentamiento padre, para poder identificarla.
    + [resuelto] en la interfaz de comercio donde se muestra el origen, destino y demas datos mostrar las coordenadas exactas donde se esta pensando.
-------- revision 3
+ [resuelto]revisar la aparicion de los campamentos rebeldes(en una simulacion solo salio 1 cuando habian 3 asentamientos activos)
+ [resuelto]revisar cada cuanto tiempo aparecen los campamentos de bandidos.
+ [resuelto]nodos de recursos agotados se regeneran, los de livestock el doble de rapido q los de metales
+ Auto-construir:
    + [resuelto]hasta no poseer al menos 1 de las materias primas que usa un edificio de transformacion no tiene sentido que esta se construya, es decir hasta que no tenga, cobre o estaño no tiene sentido que se construya una fundicion por ejemplo y asi para el resto de edifcios de transformacion
    + [resuelto]lineas de produccion: ubicacion y proximidad entre extractores y transformadores, y almacenes
    + [resuelto]opcion de exportar mapa existe pero no tiene opcion en la interfaz
------- revision 4
+ [resuelto]vista asentamiento (para ver con mas detalle la ubicacion de construccion), hay que separar la zona del asentamiento y el mapa general, son ubicaciones logicas diferentes, el mapa general es que estamos usando hasta ahora,
el mapa de asentamiento es una espacio logico que se genera tomando en cuenta el mapa general, en el punto donde se fundo el asentamiento y generando un espacio con el centro urbano al medio
+ [resuelto]Ubicacion de las construcciones por defecto:
    + las minas(oro, cobre, estaño), corrales y cantera aparecen en su nodo.
    + leñeras aparecen dentro del bosque que extraen.  
    + las granjas siempre se construyen en el borde del espacio exterior del asentamiento, a las afueras de la "ciudad", pero dentro del espacio del asentamiento, 
    + las viviendas lo mas cerca posible al centro urbano agrupadas juntas.
    + almacenes: lo mas cerca posible al centro urbano
    + edificios de transformacion(armeria, fundicion, curtiduria) cerca del centro urbano.
    + edificios militares (galeria de tiro, barracon, carpinteria) cerca del centro urbano
    + palacio: punto mas elevado dentro de la zona del asentamiento
    + mecado: cerca de los edificios de transformacion
-------------------
+ [REVISION] mecanica de anclas y satelites en el auto construir
+ Correciones:
    + [NPC/CORNER-CASE]hay veces cuando un asentamiento es fundado por npc este no tiene acceso a la madera aun cuando el script de fundacion deberia prioriza q si o si tenga madera en distancia de zona de influencia, porq cuando va a construir la leñera le sale el mensaje de que no tiene bosque en su zona de influencia
    + [COMERCIO]mirar en profundidad como funcionan las comisiones de comercio
    + [ANCLA]el funcionamiento del ancla cuando no se puede cumplir a n0, ir ampliando para simpre construi cerca. mientras no haya otra ancla de ese tipo.
    + [ANCLA]los edifcios se contruyen siempre en la misma orientacion
    + [ANCLA]tomar en cuenta la orientacion encuanto a ancla satelite, siempre intentando tener la maro cantidad de celdas adyacentes al ancla
    + [ANCLA]las anclas tienen q salir en direcciones sin reclamar del asenteamiento, es decir, el centro urbano al centro, la siguiente toma una de las direcciones (N,S,E,O,NE,NO,SE,SO) y se construye en esa direccion, la siguiente ancla de tipo diferente toma otra direccion q no haya sido tomada y asi sucesivamente, aplica solo a anclas de diferentes tipos, y para aunmentar aun mas la diferencia entre ciudades, el eje de partida q define q es sur, norte, este y oeste, lo movemos algunos grados de forma aleatoria, por ejempo 10 grados o 45 o 33 grados asi es mad dificl encontrar ciudades iguales.
    + [ANCLA] el credimiento de las viviendas tiene que tratar de conseguir crecer para rellenar los espacios en todas direcciones para que el centro urbano este simpre lo mas rodeado posible 
    + [BALANCE]cooldown de creacion de caravanas creado pero no parametrizable
    + [RESUELTO]revisar produccion de armaduras, no funciona correctamente
    + [FUNDACION]error cuadno se funda un segundo asentamiento no se elige quienes van en la caravana de fundacion para asignarlos al nuevo y quitarlos de anterior
    + [DEGRADACION-IMPORTANTE]los asentamientos no se estan degradando hasta volver a ruinas, simpelemente se estan destruyendo al llegar a 0, cuando mantenimiento al llegar a 0 deberia bajar de nivel primero y luego destruir
+ Interfaz:
    + [RESUELTO]mostar en pestaña comercio, segmento con todos los trueques activos con todos la info correspondiente, tiempo de vida, los involucrados, caravanas asignadas
    + [RESUELTO]mejora en interfaz de trueque, cuando se elige un asentamiento en los combos, muestre de bajo una info box con los materiales que posee disponibles dicho asentamiento para comerciar
    + [RESUELTO] cambiar la vista de asentamiento de un scroll infinito, a pestañas para separa la info general, edificios, militar(escuadrones y demas),
    + [RESUELTO]mejora en la interfaz de vista asentamiento, cuando pasa el cursor sobre un edificio, un tooltip con informacion sobre el mismo, nivel, nombre, produccion y consumo
    + [RESUELTO] en el espacio de escuadrones del asentamiento mostrar abajo de la lista el total de soldados y nIvel de poder
    + [RESUELTO] pestaña de mantenimiento mostra tambien aparte de coste por tick y disponible la produccion por tick
    + [RESUELTO] mostrar en interfaz de asentamiento, trueques activos, % de completacion y caravanas asignadas
    + [RESUELTO] en la interfaz de asentamientos, dentro del cuadro de seleccion de asentamientO(donde sale el nombre, agregar un icono de warning en rojo cuando no se esta cumpliendo el mantenimiento)
    + [PRODUCCION] la interfaz no muestra correctamente el valor de consumo/total en la pestaña produccion cuando la produccion esta a 0 por ejemplo si consume 3 y la producciion esta a 0 el cosumo deberia ser 0/3 0 porq no se produce nada y 3 lo que consume esa receta al 100% de capacidad(sumatoria de los edificios)
+ Add Up:
    + [ZONA/interfaz] las zonas de influencia de la misma faccion no se fusionan en un solo poligono en el mapa de asentamiento
    + [GUERRA] Los ejércitos también se mueven por el mapa para atacar como las caravanas, con un símbolo q los identifique por ejemplo un rombo, uno por cada jugador q va en el ejército, uno detrás de otro medio superpuestos y cada rombo del color de su faccion.
    + [ASENTAMIENTO]cuando se sube de nivel de asentamiento se agregan edificios de decoracion solos a la ciudad como plazas y esas cosas
    + [RUTAS]el pathfinder de las rutas para las caravanas debe buscar evitar bosques(rodearlos) o rios(no los puede atravesar) y montañas
    + [POLITICAS]politicas de ubicacion de construccion.
    + [TRUEQUE]trueque compuesto de varios materiales
    + [REGISTRO-MOTOR]cuando motor de un mensaje de fondos insufiecientes para reealiza cualquier accion, q muestre lo que falta














