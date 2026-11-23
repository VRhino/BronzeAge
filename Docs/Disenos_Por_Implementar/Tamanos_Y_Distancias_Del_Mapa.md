# Tamaños y distancias del mapa

> **Diseño acordado el 2026-11-23 con el autor. Sin implementar** (vive en `Docs/Disenos_Por_Implementar/` hasta que se implemente; ver su README). Pide **mundo nuevo** (cambia el generador) y **partida nueva** (cambia la huella de las reglas). No hay bloque `parametros` hasta implementarlo. Entrada 79 de `Docs/Mecanicas a desarrollar.md`.

## 1. Qué es y de dónde sale

El 2026-11-23 el autor probó en Conquest, en una escena hecha para eso («Distancias de plaza», `Assets/Scenes/BronzeAge/DistanciasDePlaza.unity`), las distancias del motor contra el tamaño real de las maquetas del mapa. Con las reglas de hoy las cosas se pisan; con los valores de este documento no se pisa nada. Todo lo de aquí lo decidió él sobre esa escena y sobre las mediciones del mundo `a98e65c7` (semilla 42).

La causa de fondo es una sola: **el motor mide todo desde el centro de la plaza y no sabe cuánto ocupa su maqueta.**

- El tamaño de la plaza lo decide el cliente y va atado a la zona: Conquest y el prototipo la dibujan a `zona × 0,08 × factor de nivel` (0,62 en el nivel 1; 0,8 en el 2; 1 del 3 al 5), más la muralla. El motor cree que la ciudad mide `zona × fraccionCiudadDeLaZona` (0,05) y que todo lo demás mide 0,2 u (`TAMANOS_MAPA`, de relleno).
- Las distancias se fijaron antes de que las maquetas tuvieran tamaño. Con las reglas de hoy la aldea a 3 u queda bajo la plaza (los campos desde el nivel 2, el caserío desde el 3) y la columna «a la puerta», a 2 u del centro, queda dentro de la maqueta desde el nivel 3.
- Hay dos clases de distancia y hoy se mezclan. Lo que **se coloca** (aldea, nodo, orilla, avanzada) necesita una **reserva fija desde el centro**, pensada para la plaza de nivel 5, para que la plaza crezca sin mover nada. Lo que **se mueve** (columna, ataque, entrega) se mide **desde el borde** de la maqueta de ese momento.
- La `separacionMinima` de la aldea hacía tres trabajos: aldea a plaza, aldea a extractor y aldea a aldea.
- Los nodos no tenían ninguna regla respecto a la plaza: un nodo pegado a la plaza acaba debajo de ella cuando crece.
- Reservar espacio cuesta tierra fundable, y lo que más quita son los nodos (§7).

## 2. Valores

| Regla | Dato | Hoy | Nuevo |
|---|---|---|---|
| Zona al fundar | `ZONA_INFLUENCIA.radioInicial` | 12 u | 24 u |
| Techo de la zona por nivel | `ZONA_INFLUENCIA.radioMaximoPorNivel` | 20 / 30 / 40 / 50 / 60 u | 30 / 50 / 70 / 80 / 90 u |
| Entre plazas | `FUNDACION.separacionPlazas` | 20 u | 60 u |
| Junto a mercenarios | `MERCENARIOS.radioExclusionFundar` | 20 u | 60 u |
| Orilla del río de la plaza | `ORILLA_RIO.radio.plaza` | 4,8 u | 15 u |
| Orilla del río de pueblo, aldea y avanzada | `ORILLA_RIO.radio.pueblo`, `.aldea`, `.avanzada` | 0,75 / 1,8 / 0,65 u | su tamaño real: 2,04 / 3,58 / 1,44 u |
| Aldea a plaza | `PUEBLOS_EXTRACTORES.aldea.separacionMinima` | 3 u | 15 u |
| Aldea a otro pueblo (extractor o aldea) | dato nuevo | 3 u, el mismo dato | 7,5 u |
| Avanzada a su plaza madre | `zona × AVANZADAS.fraccionCiudadDeLaZona + tamaño / 2` | 4,6 u con zona 90 | 15 u del centro |
| Plaza a nodo, al fundar | regla nueva | no existe | 12 u del centro al nodo |
| Puerta | `MOVIMIENTO.radioPuerta` | 2 u del centro | todo el contorno, a 1 u del borde |
| Tamaño de la plaza | lo calcula el cliente con la zona | crece con cada edificio | fijo por nivel (§3) |
| Nodos de piedra, cobre e hierro | generador, `PARAMETROS_RECURSOS.totalBase` | 2.489 / 240 / 360 | 1.106 / 107 / 160 |
| Entre nodos de distinto tipo | generador, `PARAMETROS_RECURSOS.separacionOtroTipo` | 45 (3,75 u) | 72 (6 u) |

No cambian: los 2 u de zona por edificio (`crecimientoPorEdificioCompletado`), el cupo de pueblos 2 / 3 / 4 / 5 / 6, la regla de los anillos de aldea (`Nucleo::anillo_de_aldea`, que sigue a los techos nuevos: las dos primeras hasta 30 u, la tercera de 30 a 50, y así), las cantidades por nodo, la separación entre nodos del mismo tipo, el estaño y el oro.

Son cinco bloques. Los cuatro primeros valen sin el quinto, que es el caro.

## 3. Bloque A: el motor es dueño de los tamaños

- `TAMANOS_MAPA` (`crates/asentamiento/src/datos.rs`, `TamanosMapa`) deja de ser relleno y lleva el tamaño real de cada cosa: la plaza por nivel, con y sin muralla; el pueblo extractor; la aldea por nivel, con su caserío y sus campos; la avanzada por etapa; los campamentos.
- La plaza deja de medir una fracción de su zona: `AVANZADAS.fraccionCiudadDeLaZona` desaparece. La zona puede cambiar sin que cambie la maqueta, y la maqueta crece solo al subir de nivel o al levantar la muralla.
- Los tamaños siguen viajando en `/v1/datos` y los clientes dibujan a esa medida, sin fórmula propia.

Tamaños de partida, en **radio** (hoy el bloque guarda diámetros). Son los que salen en Conquest con los valores del autor (lo fijo a ×2, las plazas a ×0,5 sobre lo fijo) y la zona en su techo:

| Plaza | Radio sin muralla | Radio con muralla |
|---|---|---|
| Nivel 1 | 1,49 u | no tiene |
| Nivel 2 | 3,20 u | 3,42 u |
| Nivel 3 | 5,60 u | 6,27 u |
| Nivel 4 | 6,40 u | 7,09 u |
| Nivel 5 | 7,20 u | 8,05 u |

| Elemento | Radio |
|---|---|
| Pueblo extractor | 2,04 u |
| Aldea con sus campos, niveles 1 / 2 / 3 | 2,30 / 3,00 / 3,58 u |
| Caserío de la aldea de labor, niveles 1 / 2 / 3 | 0,86 / 1,00 / 1,27 u (la ganadera, 0,93 u en los tres) |
| Avanzada: atalaya / fuerte / ciudadela | 0,68 / 1,28 / 1,44 u |
| Campamento de mercenarios | 2,20 u |

«Con muralla» es lo que el cliente ya recibe como `tieneMuralla` (empalizada en el nivel 2). La muralla de Conquest tiene dos variantes por nivel que difieren unas centésimas; la tabla lleva la mayor.

## 4. Bloque B: colocación con reservas fijas desde el centro

1. **Zona, plazas, mercenarios y orilla de la plaza:** los valores de la tabla de §2 (`crates/territorio/src/fundacion.rs`, `crates/nucleo/src/estado.rs`).
2. **La separación de aldea pasa a ser dos datos.** Hoy `sitio_de_aldea_sin_ruta` (`crates/nucleo/src/nucleo/pueblos.rs`) compara un solo dato contra toda plaza y todo pueblo. Pasa a comparar la plaza con el dato de plaza (15 u) y los pueblos, extractores o aldeas de cualquier Facción, con el dato de pueblo (7,5 u: dos aldeas de nivel 3 miden 3,58 u cada una). El muestreo de `sitio_de_aldea` arranca en el dato de plaza. El rechazo sigue siendo `pueblo.demasiado_cerca`. **El extractor sigue sin mirar ninguno** (`candidatos_de_pueblo`: nodo libre, en la zona y fuera de la orilla), así que dos extractores en nodos vecinos valen siempre.
3. **Regla nueva al fundar: ningún nodo a menos de 12 u del centro**, de cualquier tipo. Un motivo más en `crates/territorio/src/fundacion.rs` junto a `JuntoAlRio`, `CercaDePlaza` y `CercaDeCampamento`, con su código (`fundacion.cerca_de_nodo`) en el catálogo de rechazos. Con 12 u, entre la muralla de la plaza de nivel 5 y el pueblo de ese nodo quedan 1,91 u.
4. **La avanzada no se coloca a menos de 15 u del centro de su madre** (`avanzada.dentro_del_asentamiento`, `crates/asentamiento/src/avanzadas.rs`), la misma reserva que la aldea. `avanzada.pisa_otro_elemento` compara ya con los tamaños reales del bloque A.
5. **La orilla del río de pueblo, aldea y avanzada es su tamaño real**, del bloque A o con las cifras de §2. La de la plaza es su reserva (15 u) y sigue siendo un dato aparte. La de bandidos (0,8 u) no se trató.

## 5. Bloque C: la puerta es el contorno

Hoy la puerta de una plaza es su centro: `Nucleo::puerta_de` (`crates/nucleo/src/nucleo/salidas.rs`) devuelve la posición del asentamiento, `MarcharA` a un asentamiento va a esa posición (`crates/nucleo/src/sesiones/ordenes.rs`), la salida nace en ella y «a la puerta» es estar a `radioPuerta` del centro: 27 usos de `radio_puerta()` en 17 ficheros de `crates/nucleo/src/nucleo/`.

- **«A la puerta» es estar a 1 u o menos del borde**, por cualquier punto del contorno: distancia al centro ≤ radio del lugar (bloque A) + `radioPuerta`, con `radioPuerta` = 1. Una sola función con el lugar y la posición sustituye a las comparaciones sueltas. Vale igual para plazas, pueblos, aldeas, avanzadas y campamentos, cada uno con su tamaño.
- **Marchar a un asentamiento acaba en el punto del contorno por donde llega la ruta**, no en el centro.
- **Al salir, la columna nace en el borde**, en el rumbo de su destino.
- **Atacar una plaza** mide su `radioEncuentro` (1 u) desde el borde.
- Nada que se mueve se queda parado dentro de una maqueta.

## 6. Bloque D: menos nodos en el generador

En `generador/src/recursos.ts` (`PARAMETROS_RECURSOS`): `totalBase` de piedra, cobre e hierro entre 2,25 (2.489 → 1.106, 240 → 107, 360 → 160) y `separacionOtroTipo` de 45 a 72 (de 3,75 a 6 u). No se tocan `separacionMismoTipo`, la capa lejana, el estaño ni el oro (`PARAMETROS_METALES`), ni las cantidades por nodo. Precedente: el 2026-11-06 la piedra ya se dividió entre 2,25 al doblar las zonas.

Medido regenerando los nodos del mundo `a98e65c7` con las reglas nuevas (orilla 15, mercenarios 60, plazas a 60, nodo a 12 u); «foco» es la tierra a 150 u o menos de un campamento:

| | Nodos | Fundable, mundo | Fundable, focos | Plazas que caben | Nodos que ve una plaza nueva del foco |
|---|---|---|---|---|---|
| Reglas y nodos de hoy | 4.986 | 98 % | 95 % | 8.695 a 20 u | 4 de mediana; el 2 % ninguno |
| Reglas nuevas, nodos de hoy | 4.986 | 52 % | 13 % | | |
| Reglas nuevas, nodos entre 2,25 | 2.267 | 69,8 % | 38,7 % | 982 a 60 u, 144 en los ocho focos | 3 de mediana; el 6 % ninguno |
| Lo mismo, con 6 u entre tipos distintos | 2.267 | 69,7 % | 38,7 % | 982 | igual |

Los 6 u entre tipos distintos quitan las parejas donde dos pueblos se montarían (hoy hay 4 a menos de 4,1 u) y casi no cuestan tierra. Lo que mueve la tierra fundable es el número de nodos y los 12 u de la regla: dividir entre 3 deja el 11 % de los sitios del foco sin ningún nodo, y entre 4, el 20 %.

## 7. Bloque E: las columnas rodean las plazas y las caravanas entran en ellas

Hoy el A* solo mira el terreno, por decisión del autor del 2026-10-27: «los elementos del mapa no estorban» (`Movimiento_Ejercitos.md` §4.3, `crates/rutas`). Esto lo cambia **solo para las plazas**, y de forma distinta según lo que se mueva (decisiones del autor del 2026-11-23, tras implementar C). Pueblos, aldeas, avanzadas y campamentos siguen sin estorbar a nadie: se cruzan, porque por ellos se unen los caminos.

**Columnas y ejércitos: rodean todas las plazas, también las propias.**

> **Implementado en Rust (2026-11-23):** ver `Movimiento_Ejercitos.md` §4.3. Un tramo que se mete en una plaza cuesta `MOVIMIENTO.factorPasoPorPlaza` (50) veces lo suyo al trazar la ruta.

- La huella de la plaza es **paso muy caro, no cerrado**. La ruta la rodea y solo la cruza por dentro si no hay otro camino (decisión del autor: «como último recurso»). Una plaza de nivel 5 ocupa unas 13 celdas.
- La ruta que sale de una plaza o acaba en ella no paga ese coste, porque empieza y termina en su contorno (bloque C).
- Una caravana enganchada a una columna (`adjunta`) va con ella y sigue sus reglas.

**Caravanas comerciales: atraviesan las plazas de su camino, con parada.**

- **El camino no rodea las plazas.** Cada plaza cuya huella atraviese el camino de una caravana es una **escala**. Hoy el trazado ya lleva la ruta al centro de cada ciudad ajena de cuya zona cruza, para el peaje (`Comercio_Caravanas.md` §7.2).
- **En la escala la caravana sale del mapa y entra en la ciudad.** Al llegar al contorno desaparece del mundo y de la rejilla de encuentros, como una caravana `esperando` de hoy (`Comercio_Caravanas.md` §5.8.4): ni se ve ni se ataca. Al acabar la espera sale por el contorno del otro lado y sigue su camino.
- **En una plaza ajena paga el peaje de hoy dentro** (§7.3): el 2 % del valor de lo que lleva, en oro de la capital de su Facción hacia la de la ciudad. El autor lo llama «acuñar». Deja de ser un cobro instantáneo en el punto de paso. **En una plaza propia entra y no paga.**
- **La escala dura** `tiempoDescargaMinutosPorCarro` × los carros con animal de la caravana, **con un mínimo de 2 min**. Si en esa plaza tiene que dejar o recoger (una parada de su encargo, o la de una ruta compuesta: entrada 80), la parada dura lo de hoy (§5.8.5), con el mismo mínimo y sin sumarle nada.
- **Mínimo de 2 min en todas las reglas que ya existen** (decisión del autor del 2026-11-23): ninguna estancia de una caravana dentro de una plaza dura menos de 2 min. Hoy una parada que no mueve nada tarda 0 y el lanzamiento a mano de un solo carro sale en el acto (`kPorCarro × (carros − 1)`). El mínimo vale para cada parada de un encargo (descarga más carga) y para la preparación del lanzamiento a mano, también con un solo carro: ninguna caravana sale antes de 2 min (confirmado por el autor el 2026-11-23).

> **Implementado en Rust (2026-11-23):** el mínimo de 2 min (`ENCARGO.paradaMinimaMinutos`, `CARAVANA_PREPARACION.minimoMinutos`) y el carro de acarreo, que entra en su plaza y no se ve mientras descarga. Ver `Comercio_Caravanas.md` §5.6, §5.8.5 y §12. Falta la escala de las caravanas comerciales en las plazas que cruzan.

**Carro de acarreo (pueblo o aldea → su plaza): entra, no rodea.**

- Su camino va solo dentro de la zona de su plaza y solo la une con ella.
- Mientras espera a llenarse está dentro de su pueblo o aldea y no se ve en el mapa. Al salir se ve en el mundo hasta el borde de su plaza; allí entra y desaparece, gasta el tiempo de descarga (el mismo de las caravanas, con el mínimo de 2 min) y sale al mundo de vuelta hasta su pueblo o aldea, donde vuelve a desaparecer al entrar.

**Lo que cambia por dentro.**

- El trazado de las columnas suma el coste de las huellas de plaza. El de las caravanas no lo suma, pero parte el plan en tramos por cada escala, con un evento de entrada y otro de salida.
- La caché de caminos (`crates/nucleo/src/nucleo/viaje.rs`, `cache.epoca`) caduca cuando una plaza se funda, sube de nivel, levanta su muralla o cae.
- El peaje deja de ser un evento en el punto de paso y se cobra al entrar en la escala.

Sin este bloque, una columna de paso cruza la maqueta por encima, aunque con el bloque C nunca se queda parada dentro. Por eso va el último.

## 8. Orden y lo que arrastra

1. A, B y D juntos: son el mundo nuevo y la partida nueva.
2. C.
3. E.

Al implementar: `Docs/Mechanics/` (`Economia_Construccion.md`, `Movimiento_Ejercitos.md`, `Campamentos_Mercenarios.md`, `Generacion_Del_Mundo.md`), el canon, `Docs/Coordinacion/07_Catalogo_de_Rechazos.md` y las versiones de los crates que cambien.

Lo que toca a los demás:

- **Conquest:** dibuja la plaza al tamaño que diga el motor en vez de calcularla con la zona; la columna espera fuera de la maqueta y la ruta y la marca de destino acaban en el contorno. La escena «Distancias de plaza» queda como banco de pruebas.
- **BronzeAgeClient:** lo mismo para su mapa 3D (`FRACCION_CIUDAD` y `radioDePlaza` en `src/mundo3d`).
- **BotsBrain:** sus plazas se fundan con las separaciones nuevas y la regla de nodo, y sus aldeas con las dos separaciones; leen los rechazos.

## 9. Por concretar al implementar

- Los nombres de los datos nuevos y si `TAMANOS_MAPA` sigue guardando diámetros.
- Qué pasa con una orden de marchar a un punto que cae dentro de la huella de una plaza.
- Con menos nodos y las mismas cantidades por nodo hay menos piedra y metal en el mundo: revisar en `Docs/Mecanicas a balancear.md`.
- La orilla del río de los bandidos.

## 10. Cómo se midió

Los tamaños salen del código de Conquest (`MaquetasDelPrototipo.RadioDePlaza`, `MaquetasDelMapa`, `ParcelasDeAldea.Radios`) y de la escena de pruebas. La tierra fundable y los nodos, de unos análisis sueltos sobre el artefacto de mundo que regeneran los nodos cambiando `PARAMETROS_RECURSOS` en memoria; no están versionados.
