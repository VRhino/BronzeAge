# Escalas del mundo en 3D — definición

> **Estado (2026-10-30, tercera ronda): escalas cerradas, se pasa a los siguientes pasos (§6).** El autor eligió en la
> mesa de escalas 3D los valores de §3 y decidió:
> - rejilla de 4 u con suavizado;
> - pendientes del movimiento medidas con la exageración de altura (§4.3);
> - la cámara de juego (§5);
> - el mundo 3D se prototipa primero en el cliente de jugador (`BronzeAgeClient`) y luego se lleva a Unity (§5).
>
> **La mesa se sigue afinando**: es la referencia visual, y este documento se actualiza con cada acuerdo. Nada está
> aplicado todavía en el generador, el contrato, el servidor ni los clientes.
>
> Mesa: https://claude.ai/artifact/9PfPuAfEr3nmtuK5jt5Mwg (fuente en `Consideraciones/artefactos/mesa_escalas_3d/`, §8).
> Página 2D anterior, con el análisis de escalas: https://claude.ai/artifact/GKqGYMohRFTMihKpPkaeGG

## 0. Razón de ser

Probando el juego, el autor vio que el movimiento ya está coordinado y se siente bien. También la vista del jugador, la
marcha de las tropas respecto a esa vista, el tamaño de las zonas de influencia y la separación entre plazas. Lo que no
encaja con esas medidas es el terreno: montañas, valles, bosques y árboles. Además, la rejilla de movimiento se nota
demasiado grande. Las ciudades, los pueblos y los campamentos como maquetas agrandadas están bien.

Hasta aquí el mundo se había modelado solo con referencias 2D (mapas, perfiles). La mesa 3D (three.js) pone el terreno
del generador bajo la cámara del juego, con todo lo que ya funciona a su medida, y deja probar tamaños.

## 1. Tres escalas y una regla

| Escala | Qué es | Estado |
|---|---|---|
| **Ancla, en u** | Cámara 15 × 8,4 u; tropa ligera 30 u/min, que cruza la pantalla en 30 s; zonas de 6 a 30 u; plazas a 30 u; visión 15 / 30 u (`Calibracion_Distancias_Definicion.md` §1-§2) | Fija: se siente bien |
| **Ficción ×100** | El mapa de 2.667 × 2.000 u se lee como 3.200 × 2.400 km (el Mediterráneo oriental): 1 u ≈ 1,2 km, una pantalla ≈ 18 × 10 km y un minuto de juego ≈ 36 km, un día de marcha | Adoptada el 2026-10-30 («el mapa de 32 × 24 km en realidad es más un ×100») |
| **Unity ×12, maqueta** | 1 u = 12 m de Unity (§2.1 de la calibración). Plazas, tropas y árboles son maquetas agrandadas: su tamaño no sale de la ficción | Fija |

**Regla para el terreno: tamaño en la ficción ÷ 1,2 km = u.** La altura se exagera, como en cualquier mapa de campaña. Con
la ficción tal cual, una montaña de 2.000 m serían 20 m de Unity, casi plana.

## 2. Lo que enseñó la mesa del generador actual (seed 42)

Ventana de 320 × 240 u entre la cordillera norte y el campamento‑0:

- **El llano es una mesa.** Tiene menos de 1 m de desnivel en una pantalla, y los árboles son lo único con volumen.
- **La cordillera tiene altura, pero no forma.** El desnivel por pantalla llega a 164 m (mediana de la ventana: 8 m), pero
  es una rampa lisa que dura varias pantallas. Mide cerca de 100 u de ancho, que es la anchura de una cordillera real en
  la ficción ×100. Lo que le falta son formas del tamaño de una pantalla: crestas, valles y lomas de 2 a 40 u.
- **La rejilla de 8 u es media pantalla.** El quiebro de 45° de una ruta octil sigue ahí con rejilla de 4 o 2 u. Lo quita
  el suavizado; la rejilla fina sirve para que quepan valles estrechos.
- En la vista libre se ve un borde recto con dientes en la cordillera. Viene del generador y hay que revisarlo cuando se
  rehaga el relieve.

## 3. Valores elegidos por el autor (2026-10-30)

La primera ronda fue relieve ×2, altura ×3, copa 0,4 u, maquetas ×1,6 y rejilla de 2 u con suavizado. Con la pendiente
medida sobre lo que se ve (§4.3), el 61 % del macizo quedaba cerrado al paso. En la segunda ronda el autor bajó a altura
×1,5 y tropa ×3,2. En la tercera, la que vale, quedó así:

| Mando de la mesa | Valor | Qué significa |
|---|---|---|
| Terreno | **Propuesta ×100** | Boceto procedural con formas a escala de pantalla (receta en §3.1); no sale del generador |
| Tamaño del relieve | **×2,2** | Escala tamaño **y** altura a la vez (homotecia: las laderas no cambian). Crestas cada ~92 u, valles de 11-44 u de ladera a ladera, colinas de ~31 u y lomas de 7-22 u |
| Altura | **×2** | Exageración de todas las alturas. También cuenta para las pendientes del movimiento (§4.3) |
| Copa de árbol | **0,4 u** (≈ 5 m de Unity) | Árbol como maqueta. Un bosque denso lleva unos 3 árboles por u² (el número de árboles sale de la densidad del bosque y del tamaño de copa) |
| Plazas (maquetas) | **×1,6** | Escala de casas, pueblos, avanzadas y campamentos |
| Héroes y tropa | **×3** (soldado de 4,9 m de Unity) | Mando propio, separado de las plazas como los árboles (decisión del autor). El héroe va a caballo al frente de su columna |
| Rejilla | **4 u, con suavizado** | Ver §4.3 y §6 |
| Cámara | **30°**, libre | Gira en cualquier ángulo alrededor del héroe, a distancia fija y siguiéndole siempre (§5) |

Medido sobre la ventana con esos valores (`relieve ×2,2`, `altura ×2`):

| | Valor |
|---|---|
| Cumbres sobre el llano | hasta ~440 m de Unity (36 u); el 99 % del terreno, por debajo de 340 m |
| Desnivel en una pantalla del macizo | 107 m de mediana, 219 m en el percentil 90 |
| Pendiente en el macizo | 31° de mediana, 59° en el percentil 90 |
| Macizo cerrado al paso (más de 35°) | 43 %: las rutas van por los valles y los collados |

### 3.1 Receta del boceto (para que el generador pueda reproducirlo)

Todo en u y metros de Unity; `S` es «Tamaño del relieve» (2,2), y los ruidos son simplex 2D con semillas fijas.

- **Geografía fija**: un macizo al norte (máscara suave de `y < 55` a `y > 140`, con el borde ondulado), piedemonte de
  colinas entre 120 y 195, y un llano al sur.
- **Llano**: base de 468 m, ondulación amplia de ±4 m (60 u) y lomas de ±1,8·S m con longitud de onda 6·S u.
- **Colinas**: hasta 18·S m, longitud de onda 14·S u.
- **Macizo**: `S × (18 + 95·c)` m, donde `c` es un ruido de crestas (*ridged*, 4 octavas) de longitud de onda 42·S u.
- **Valles**: perfil gaussiano alrededor de los ríos, de semiancho `(4 + 6·macizo)·S` u, con suelo a `466 + 8·macizo·S` m.
- **Ríos**: dos, de ~0,9 u de ancho, que no se cruzan salvo por **vados** (6 en la ventana).
- **Bosque**: un ruido de 24·S u más bosquetes de 3,5·S u; desaparece por encima de las cumbres bajas.

## 4. Reglas que la mesa respeta

### 4.1 Plazas, zonas y ciudad

- Zona por nivel (`ZONA_INFLUENCIA`): inicial 6 u; niveles 1 a 5: 10 / 15 / 20 / 25 / 30 u. La ciudad mide el 10 % del
  radio de su zona (`AVANZADAS.fraccionCiudadDeLaZona`): 1,5 u en el nivel 2 y 3 u en el 5. Con maquetas ×1,6 la ciudad
  del nivel 2 queda igual que la que eligió el autor.
- Las plazas se ponen cada 30 u (retícula hexagonal), en tierra llana o de colinas.

### 4.2 Pueblos y avanzadas: siempre dentro de la zona de su plaza

**Regla del autor (2026-10-30): los pueblos y las avanzadas son partes de la plaza que se muestran en el mundo y van
siempre dentro de su zona de influencia.** Coincide con `Economia_Construccion.md` §14 (el nodo o el punto del pueblo
dentro de la zona real de la madre; la avanzada, dentro de la zona y sin pisar su ciudad).

- Cupos por nivel de la madre: pueblos 1 / 3 / 4 / 5 / 6 (`PUEBLOS_EXTRACTORES.cupoPorNivel`); avanzadas, una por nivel
  desde el 2 (`AVANZADAS.cupoPorNivelDeLaMadre`).
- Separación de 3 u entre pueblos y de cualquier plaza (`separacionMinima` de la aldea).
- La mesa aproxima la zona real recortada con un reparto por cercanía: un pueblo es de la ciudad más cercana y está a
  menos del radio. Dibuja además el camino de carro de cada pueblo y avanzada hasta su muralla.

### 4.3 Movimiento

- Coste por terreno (`COSTE_MOVIMIENTO` del generador): llano ×1, colina ×1,8, montaña ×3,5 y cima ×12, multiplicado por
  `1 + min(2; 0,45·pendiente)`. El agua y las pendientes de más de 0,7 (35°, `pendienteMaximaTransitable`) no se cruzan.
- La ruta funciona como `crates/rutas`:
  1. engancha el origen y el destino a hasta 4 nodos cercanos con tramo recto libre;
  2. cruza la rejilla con A* (aristas en 8 direcciones, coste integrado por Simpson);
  3. sale al destino exacto.
- **Suavizado por coste**: la ruta se salta vértices solo si la recta está libre y **no cuesta más** que el tramo que
  sustituye. Así se quitan los quiebros sin cortar por una montaña cara.
- **Las pendientes se miden con la exageración de altura** (decisión del autor, 2026-10-30): lo que se ve es lo que se
  anda, y nadie camina por laderas raras. Vale para el paso (35°), el coste, y dónde caben plazas, pueblos y árboles.
  Para el generador esto quiere decir que las alturas que escribe son ya las que se ven: no hay un factor visual aparte.
- La rejilla es de **4 u** con suavizado (decisión del autor, 2026-10-30). Con 2 u la ruta apenas cambia y el grafo
  cuadruplica.
- La columna va adonde se hace clic, a 30 u/min en llano, dividida por el coste del terreno que pisa.

## 5. Decisión: primero en el cliente de jugador, después en Unity

El mundo se representará como en la mesa:

- la **cámara de juego**, libre en ángulo pero a distancia fija del héroe y siempre siguiéndole (decisión del autor,
  2026-10-30). La distancia es la que da 15 u de ancho a la altura del héroe; arranca a 30° de inclinación y no se acerca
  ni se aleja. En la montaña se levanta si la ladera queda por encima de ella;
- el terreno con formas a escala de pantalla y la altura exagerada;
- árboles, plazas, héroes y tropa como maquetas;
- la columna que va adonde se hace clic.

**El cliente final sigue siendo Unity.** El cliente de jugador (`BronzeAgeClient`, TypeScript, frente de la fase 7 en
`Docs/Coordinacion/08_Consumo_del_Contrato.md`) es el prototipo inicial (decisión del autor, 2026-10-30). Hacerlo
funcionar primero ahí, con three.js como en la mesa, ayuda a cerrar la representación antes de llevarla a Unity.
`Reconstruccion_Definicion.md` §2.1 lleva una nota con esto.

## 6. Qué falta para llegar ahí (orden propuesto)

1. **Seguir afinando la mesa** con lo que quede abierto en §7.
2. **Suavizado por coste en el servidor** (hecho, 2026-10-30). No depende del terreno nuevo y quita el zigzag ya.
   Medido en el mundo de la semilla 42, con 1.000 rutas entre campamentos:
   - los vértices bajan de 203.483 a 18.291, 11 veces menos;
   - la marcha cuesta un 2,3 % menos;
   - cada ruta tarda 3,0 ms en vez de 2,8.

   El artefacto gana la capa `coste` (hash `c6639fa6…`; el resto, idéntico) y la generación pasa de unos 68 s a 77 s.
   Regla en `Docs/Mechanics/Movimiento_Ejercitos.md` §4.3.
   - **Capa `coste` en el artefacto.** El generador escribe, cada 4 u, el coste del terreno con la misma función que
     cuesta las aristas del grafo (u16 en milésimas, 1000 = llano). Es opcional: sin ella, el servidor no suaviza.
   - **`crates/rutas`.** Después del A*, la ruta se estira desde el origen. Desde cada vértice salta al más lejano
     posible siempre que se cumplan las tres condiciones:
     - la recta está libre (`transitable`);
     - no se salta ninguna arista de vado;
     - la recta no cuesta más que el tramo que sustituye, medidos los dos con la capa `coste`.

     Los tramos nuevos llevan ese coste en el plan, y las aristas que quedan, el suyo.
   - **Quién suaviza:** las columnas (marchar) y las persecuciones. Las caravanas y los aedas no suavizan, porque van
     por la red de caminos y la red se forma con sus aristas.
3. **Generador: rehacer el relieve con la receta de §3.1** sobre la geografía regional (macizos, cuencas y costas
   actuales), con formas de 2 a 80 u dentro de cada macizo, ríos con vados y bosque. Se valida con la mesa: la ventana
   se saca con `generador/scripts/datos-mesa-3d.ts`.
4. **Contrato de mundo: resolución.**
   - El grafo pasa de 8 u a **4 u**: 334.167 nodos y unos 1,3 M aristas, 4 veces los de hoy (83.834). Hay que medir el
     tamaño del artefacto, la carga y el A* del servidor.
   - La altura: hoy los rásteres van a 4 u y la mesa usa 0,5 u. En todo el mapa serían 5.334 × 4.000 = 21 M muestras,
     43 MB en u16 sin comprimir; hay que decidir qué resolución lleva el artefacto y qué detalle añade el cliente.
5. **Cliente de jugador (prototipo) y después Unity** (§5):
   - three.js con la cámara de juego;
   - terreno por teselas;
   - árboles instanciados por trozos (la mesa corta en 140.000);
   - maquetas instanciadas por material.
6. **Metales en zonas montañosas** (pendiente desde el análisis del generador): depende de qué es «montaña» en el relieve
   nuevo.

## 7. Abierto

- **Ríos y vados** del terreno nuevo. Con el terreno real, la mesa dibuja los ríos pero no cortan el paso, porque la
  ventana no trae sus vados.
- **Cuántos árboles** dibuja el cliente y con qué detalle.
- **Tamaño de héroes y tropa** frente a las plazas y los árboles, ahora con su propio mando (×3,2).
- **Resolución de la altura** en el artefacto (§6.4).

## 8. Fuente de la mesa

- `Consideraciones/artefactos/mesa_escalas_3d/plantilla.html`: la página (three.js 0.147 desde jsDelivr), con
  `__DATOS__` donde van los datos.
- `Consideraciones/artefactos/mesa_escalas_3d/datos.json`: la ventana de la seed 42 (x0 = 1276, y0 = 340).
- `Consideraciones/artefactos/mesa_escalas_3d/montar.mjs`: escribe la página entera
  (`node montar.mjs [salida.html]`). La página montada no se versiona.
- `generador/scripts/datos-mesa-3d.ts`: regenera `datos.json` desde una seed y su artefacto
  (`npx tsx scripts/datos-mesa-3d.ts 42 mundo-<hash>.bin datos.json [x0 y0]`, unos 25 s).
