# 1. Sistema de Mundo y Territorio


## 1.0a La escala del mundo

El motor maneja **dos espacios**: el mapa del mundo y la Vista de Asentamiento. La equivalencia es:

> **1 unidad de mapa = 40 unidades locales de la Vista de Asentamiento.**

De ahí sale la jerarquía territorial de la ficción:

| | radio | equivale a |
|---|---|---|
| **Ciudad** (casco urbano + afueras) | 121-205 unidades locales = **3-5 de mapa** | el núcleo habitado |
| **Provincia** (zona de influencia) | **30-180 de mapa**, según nivel | lo que controla un asentamiento |
| **Reino** | todas las provincias de una Facción | sin número fijo: una Facción podría llegar a todo el mapa |

**Cuántas provincias caben.** El mundo mide 2000×2000 y el 91% es habitable (7,6% agua y 1,3% cima), o sea
3,64 millones de unidades². Repartido entre las **200 provincias** que Fase 0 quiere, salen 18.207 u² por
provincia: un **radio de ~76**, cerca del tope de zona de influencia de nivel 1 (60).

**Por qué 40.** Una ciudad tiene un TAMAÑO MÍNIMO de ~121 unidades locales que no encoge —al fundar, la Granja
inicial no cabe más cerca—, mientras que la provincia arranca pequeña (radio 30). Con 40, ese peor caso mide
3,0 de mapa dentro de una provincia de 30: el **10,1%**. De ahí para arriba el ratio solo baja (5% en nivel 1,
3% en nivel 5), porque la provincia crece y el mínimo de la ciudad no.

**La regla: una ciudad nunca pasa de una décima de su provincia.** El resto es campo, bosque y minas — lo que
debe haber entre dos ciudades.

**Un reino mide ~5 provincias**: sale de `MANTENIMIENTO.escalaDistancia` = 400, la distancia a la capital a la
que el coste de mantenimiento toca su tope (×2). 400 / 76 ≈ 5.

## 1.0b El agua es un OBSTÁCULO

**Ni los ejércitos ni las caravanas pueden moverse sobre agua.** No es terreno caro: es infranqueable. Todo lo que se desplaza por el mapa —caravanas de comercio, caravanas de fundación, ejércitos y el trazado de los caminos comerciales— la rodea o no llega. Nadie camina sobre el agua.

Tres consecuencias:

1. **Un destino puede quedar SIN RUTA.** Una isla, una península cortada. Si no hay camino por tierra, no hay ruta, y quien la pidió decide qué hacer — no se moviliza el ejército, no sale la caravana, no se traza el camino comercial. Nunca se cae a una línea recta: sería precisamente una ruta por el mar.
2. **No se puede fundar sobre agua**: el asentamiento quedaría incomunicado para siempre. El agua y la cima son terreno inhabitable.
3. **La cima es transitable**, solo que cara (coste 12): es terreno difícil, no un medio distinto.

**Los RÍOS no cortan el paso**: no son terreno sino una entidad aparte, y el coste de movimiento no los tiene en cuenta.

El comercio por mar está fuera de alcance (Doc 3.11): sin barcos, dos costas enfrentadas no comercian.

## 1.1 Generación del mundo (Fase 0)
- Mapa CUADRADO, espacio de coordenadas continuo (no grid discreto).
- Tamaño base: 2000×2000 unidades, PARAMETRIZABLE.
- Terreno con RELIEVE REAL: campo de ELEVACIÓN continuo por ruido fractal, clasificado en bandas agua/costa/llano/colina/montaña/cima; RÍOS como polilíneas que nacen en montaña y descienden por gradiente de máxima pendiente; BIOMA derivado de terreno+fertilidad+humedad — ver `Consideraciones/Fase_0_1_Definicion.md`. El coste de moverse por el mapa depende del tipo de terreno (ver 1.5/1.6 y Doc 3.6). Opcionalmente sesgado a una región geográfica real (Grecia continental/Anatolia/Egeo/Nilo/Mesopotamia — "Libre", sin sesgo, es el valor por defecto). Sin mar navegable (eje naval pospuesto, ver `Roadmap_Escalado.md`).
- Generación PROCEDURAL de recursos por niveles de rareza:
  - Común (alta frecuencia, disperso): Madera (bosques, ver 1.4), Piedra, Trigo (vía fertilidad, ver 1.4).
  - Intermedio (frecuencia media, varios clusters): Cobre.
  - Raro (baja frecuencia, pocos clusters, espaciado mínimo forzado entre ellos): Estaño, Oro.
- Spawn de jugadores nuevos: posición aleatoria uniforme, INDEPENDIENTE de la ubicación de recursos.
- Fases avanzadas (fuera de alcance de Fase 0): mar navegable; posible mapa fijo diseñado a mano en vez de procedural; puntos de interés fijos (ruinas, maravillas); clima.

## 1.2 Fundación de asentamientos
- La posición es LIBRE —sin rejilla ni puntos predefinidos— y se funda donde está el héroe (1.3): el sitio se elige caminando hasta él.
- **Aviso de viabilidad al fundar** (ayuda, no bloqueante): antes de confirmar se dibuja el radio inicial (30 unidades, ver TAMAÑO abajo) coloreado según viabilidad — VERDE si la posición es fundable y hay bosque alcanzable dentro del radio (madera garantizada desde el principio), ÁMBAR si es fundable pero sin madera al alcance (el asentamiento tendrá que arrancar solo con la reserva de materiales iniciales, ver 1.3), ROJO si la posición no es válida (fuera del mapa o solapa una zona de influencia existente). No impide fundar en ámbar: es solo información para decidir mejor. También indica qué nodos minerales caen dentro del radio inicial.
- Al fundar se genera automáticamente una ZONA DE INFLUENCIA (círculo/polígono) = nodo.
- TAMAÑO: nace con un radio inicial de 30 unidades y crece gradualmente cada minuto hacia un TECHO que escala con el NIVEL del asentamiento (Doc 4.5): nivel 1 → 60, 2 → 90, 3 → 120, 4 → 150, 5 → 180. Subir de nivel no hace saltar el radio de golpe — solo levanta el techo hacia el que la zona ya venía creciendo.
- Reglas de construcción: solo se puede construir dentro de una zona de influencia existente (ampliándola si es del mismo bando) o fuera de cualquier zona (creando una nueva).
- FRONTERAS: al chocar dos zonas en expansión se genera un LÍMITE DURO — ninguna zona sigue creciendo en esa dirección. Solo se rompe si el asentamiento rival cae o su zona se debilita/reduce (guerra u otros medios). Las fronteras son "vivas", reflejan el poder relativo de cada bando en cada momento.

## 1.3 Onboarding de nuevos jugadores
- Jugador nuevo aparece en un punto ALEATORIO del mapa con una "caravana de asentamiento" para fundar donde decida.
- **Aparecer ahí es literal (ver 1.10)**: el héroe nace situado en mundo abierto, con su columna, y se mueve por el mapa hasta el sitio donde quiera fundar. **Se funda DONDE SE ESTÁ** — no se elige un punto cualquiera sobre el mapa desde fuera. Caminar hasta un buen emplazamiento es la primera decisión del juego, y es lo que da sentido a explorar antes de asentarse.
- **SE LLEGA A UN MUNDO HABITADO, no a un vacío.** El servidor arranca con Facciones NPC ya asentadas, y son
  **vecinos, no depredadores**: se defienden si las tocan, pero no dan caza a los recién llegados, y ofrecen
  con qué comerciar. Un novato no es aliado de nadie, así que unas Facciones que cazaran a todo lo no aliado
  lo matarían antes de que tuviera con qué defenderse.

  El mundo no es por eso inofensivo: el peligro de base lo dan los **bandidos** (1.9), que atacan por su
  cuenta y no son de nadie. El reparto es **bandidos la amenaza, Facciones NPC los vecinos, otros jugadores la
  guerra**.
- **FUNDAR NO ES EL PRIMER ACTO.** Para fundar una Facción nueva hacen falta **varios ciudadanos** y **haber
  sido ciudadano de alguna antes** — fundar es un **cisma**, gente que ya vivía en algún sitio y se marcha a
  hacer el suyo. Sin esto, mil jugadores que entran son mil Facciones y mil aldeas en el primer minuto; la
  fundación grupal (hasta 5) permite compartir, pero no obliga a nada.

  Las dos exigencias son **parámetros** (`FUNDACION.minFundadoresParaFaccionNueva` y
  `FUNDACION.exigeCiudadaniaPrevia`), y durante las primeras pruebas están **abiertas** —grupo de 1 y
  ciudadanía opcional—: con cinco testers el freno estorba, con mil hace falta.
- **Un héroe sin Facción recuerda lo que explora.** La memoria del mundo es de la Facción (Doc 5.12.8), pero quien todavía no tiene bandera lleva la suya propia, que se funde con la de la Facción al fundar o al entrar en una. Sin esto el primer minuto de juego sería un paseo a ciegas sin registro.
- EDIFICIOS INICIALES: todo asentamiento nace con un Centro Urbano (marcador único, no construible por ningún otro medio), una Granja y 3 Viviendas, ya ACTIVOS sin pasar por la cola de construcción.
- FUNDACIÓN GRUPAL: hasta 5 jugadores pueden organizarse para aparecer juntos en el mismo punto, compartiendo una caravana, fundando el asentamiento entre los 5. Los 5 reciben Ciudadanía de inmediato.
- MATERIALES INICIALES: la caravana de fundación entrega una reserva inicial de recursos al fundar, suficiente para arrancar la primera construcción. Sin ella el asentamiento quedaría bloqueado para siempre: el edificio que produce madera también cuesta madera. Cifras (`FUNDACION.materialesIniciales`/`POBLACION.pesants.inicial`, placeholder): 50 madera + 20 piedra + 100 trigo + 100 oro, y población inicial de 20 pesants.
- PROTECCIÓN TEMPORAL: tras la fundación hay un período de gracia durante el cual NO se cobra Mantenimiento (ver Doc 4.5), para que la economía pueda arrancar antes de pagarlo. Duración placeholder.

## 1.4 Fuentes de recursos por tipo
- CULTIVOS (trigo): NO son un nodo recolectable directo. Dependen de la FERTILIDAD DEL SUELO de la zona (atributo de terreno); requieren construir una Granja para aprovecharse.
- MADERA: proviene de BOSQUES, representados como ZONAS del mapa (no puntos), con densidad variable.
- MINERALES (cobre, estaño, oro): CONDICIONADOS AL RELIEVE — cobre/estaño en colina o montaña, oro solo en montaña (el más exclusivo, es el más raro); piedra permisiva (colina/montaña/llanura fértil/estepa) para no comprometer el recurso común más consumido (`RECURSO_BIOMA_PERMITIDO`, `worldgen/config.ts`). Cada uno tiene su propio edificio de extracción: cantera para piedra, mina de oro, mina de cobre, mina de estaño.
- LIVESTOCK (ovejas, vacas, caballos + especies adicionales por definir): fauna LIBRE en el mapa, debe CAPTURARSE para aprovecharse. Cría/domesticación pospuesta a fases avanzadas (en Fase 0 se trata como recurso consumible/finito). Rendimientos: ovejas = carne+leche; vacas = carne+leche+cuero; caballos = fuente de entrenamiento de tropas montadas (caballería ligera, carros de guerra), no dan recurso de consumo.
- MADERA y PIEDRA: materiales base de construcción de edificios e insumo de materiales derivados.
- REGENERACIÓN DE YACIMIENTOS AGOTADOS: un nodo (mineral o livestock) que llega a stock 0 no queda muerto para siempre — vuelve a aparecer con su cantidad inicial completa pasado un cooldown, parametrizable por categoría (`REGENERACION_NODOS`). Livestock (fauna, se recupera por reproducción/migración) regenera al DOBLE de rápido que los yacimientos minerales (piedra/cobre/estaño/oro) — un filón agotado tarda mucho más en "rellenarse" que una manada.
- COMMODITIES DE NOBLEZA (uvas/olivas → vino/aceite): no son alimento básico, requeridas para felicidad de la Nobleza; su déficit arriesga rebelión/estancamiento, no hambruna.

## 1.5 Chokepoints estratégicos (heredado de Iberia)
Puertos de montaña detectados como PUNTOS DE SILLA del campo de elevación (mínimo local a lo largo de la cresta, máximo local en la dirección perpendicular — geometría determinista por seed). El asentamiento cuya zona de influencia CUBRE un chokepoint lo controla; las caravanas comerciales de una Facción rival cuya ruta pasa cerca pagan un PEAJE EN ORO al controlador, cobrado al llegar a destino (ver `Consideraciones/Fase_0_3_Definicion.md`). Solo los puertos de montaña son chokepoints; los vados de río no.

## 1.6 Caminos comerciales automáticos
- Al ACEPTARSE un trueque entre dos asentamientos (Doc 3.2) se genera AUTOMÁTICAMENTE un camino físico — una polilínea calculada con pathfinding (A* sobre coste de terreno) que rodea relieve costoso en vez de ir en línea recta, el mismo algoritmo que usa cualquier caravana para su propia ruta (ver 1.1, Doc 3.6). El jugador no lo construye.
- El camino AFECTA LA VELOCIDAD de las caravanas que lo siguen (más rápido que campo abierto — ver Doc 3.6).
- MEJORABLE vía Políticas: la política "Rutas Rápidas" (Tesorero, Doc 3.12).
- Si se rompe la relación comercial que originó el camino, el camino queda como infraestructura física PERMANENTE (no se elimina).

## 1.7 Cap de fundación de asentamientos por Facción (inspirado en Rise of Nations)
- Límite DURO de asentamientos que una Facción puede FUNDAR (no aplica a conquista/anexión, que no tiene límite). Fundar por encima del cap se rechaza.
- ESCALA con el NIVEL DE FACCIÓN: progresión fácil de 1 a 3, luego se complica progresivamente hasta un MÁXIMO DE 7 en etapa tardía. Curva (`CAP_FUNDACION_POR_NIVEL`, placeholder): `[1, 2, 3, 3, 4, 5, 5, 6, 6, 7]` para niveles de Facción 1 a 10.
- Complementa (no sustituye) al sistema de Mantenimiento/Coste de Gobernanza (ver Doc 4).
- Da incentivo mecánico a preferir vasallaje/conquista sobre fundación directa una vez alcanzado el cap.
- **Qué hace subir el nivel de Facción**: la experiencia acumulada (combate, edificio completado, conquista, defensa/ataque de caravana). Ver Doc 2.2.1 para el criterio completo, la curva de umbrales y el cupo de asentamientos nivel 2/3 que también depende de este nivel.

## 1.8 Caravana de Fundación (mecanismo de expansión más allá del primer asentamiento)

Mecanismo COMPLEMENTARIO al Cap de Fundación (1.7) — ambos coexisten, no se sustituyen. El objetivo es que fundar un asentamiento adicional cueste recursos reales y se sienta orgánico, evitando fundación en cadena sin fricción.

**COSTE** = suma de tres componentes:
1. Los materiales iniciales que recibe todo asentamiento nuevo al fundar (ver 1.3).
2. El coste de construcción de los edificios que nacen automáticamente con la fundación (Centro Urbano, Granja, 3 Viviendas — ver Doc 4.2.1).
3. +50 de madera extra, representando el coste de fabricar la caravana en sí (placeholder).

**GATES DE ORIGEN (todos necesarios simultáneamente):**
- Solo puede lanzarse desde un asentamiento que PUEDA PAGAR el coste completo.
- Solo puede lanzarse desde un asentamiento en NIVEL 2 como mínimo — gate estructural independiente del coste, que no depende de números ajustables.
- **Cooldown de creación**: tras lanzar una Caravana de Fundación —o crear una comercial (Doc 3.12), mismo cooldown COMPARTIDO entre las dos— el asentamiento de origen no puede crear otra hasta que pase `CARAVANA_COOLDOWN` (10 minutos, parametrizable). Evita el spam de creación cuando la caravana recién lanzada es destruida (bandidos, 1.9; intercepción, Doc 3.10) y el cap y los recursos vuelven a estar disponibles de inmediato. Es regla del motor: gatea igual el lanzamiento de un jugador que el de la gobernanza NPC.

**NATURALEZA:**
- Se lanza por ACCIÓN MANUAL EXPLÍCITA del jugador (elige destino y confirma). Es la caravana donde la intencionalidad del jugador es el punto central del diseño.
- INTERCEPTABLE Y ESCOLTABLE igual que cualquier otra caravana (Doc 3.10) — mismas reglas de combate y umbral de captura del 50%, sin regla especial.

**CASOS:**
- Si el punto de destino elegido deja de estar disponible en tránsito (ej. otra Facción funda ahí primero): en fases con movimiento libre de caravana por el mapa (Fase 1+), el jugador MUEVE la caravana ya en marcha hacia otro punto válido y funda allí — no se pierde el viaje ni el coste.
- CANCELACIÓN: el jugador puede DESARMAR la caravana de fundación en el asentamiento de origen y recuperar el contenido COMPLETO — sin pérdida por cambiar de opinión antes de fundar.

## 1.9 Campamentos de bandidos (inspirado en Travian)

- Aparecen únicamente en BOSQUES (ver 1.4) que NO se solapan con ninguna zona de influencia existente — territorio no reclamado por ninguna Facción. Se comprueba el CENTRO del bosque contra los polígonos de zona, no el círculo completo.
- **UNO por asentamiento, SIEMPRE**: cada asentamiento vivo tiene el suyo, en SU bosque no reclamado MÁS CERCANO, y queda ASIGNADO a él. Así nunca aparece en la otra punta del mapa sin ningún asentamiento cerca, ni pegado a una zona de influencia. Un asentamiento sin campamento propio siempre puede recibir el suyo, sin importar lo cerca que esté de otro ya atendido. Mientras haya asentamientos sin cubrir y se cumpla el plazo de reaparición, cada minuto se cubre como mucho uno.
- Mientras el campamento sigue en pie, ATACA CARAVANAS (Doc 3.10) que pasen dentro de un radio fijo de su posición, cada minuto — se resuelve con números, contra la defensa de la caravana. Si gana, la caravana se pierde por completo (nadie la recibe: el bandido no tiene almacén propio).
- **Se ataca con una columna que llegue a él** (Doc 5.12.3), y la batalla se juega en Unity: los héroes de la columna contra las tropas del campamento, manejadas por la IA del juego (Doc 5.15.6). SIN gate de cargo: a diferencia de un asedio, un campamento bandido es una amenaza de mundo abierto, no una acción de guerra entre Facciones. Al ser DESTRUIDO entrega una RECOMPENSA (loot) fija a quien lo destruye y se agenda el plazo de reaparición. La recompensa va al carro de la columna, hasta donde quepa; lo que no cabe se pierde, como el botín de una caravana (decisión del usuario, 2026-09-15). Si pierde, sus héroes quedan heridos (5.16.4) y la columna pierde la mitad del carro, que no se lleva nadie. Mientras no exista la batalla de Unity, el choque se resuelve con números: los soldados de los héroes sanos contra el poder fijo del campamento.
- REAPARICIÓN: tras destruirse, aparece un campamento nuevo pasado el plazo — cerca del primer asentamiento sin cobertura que encuentre (no necesariamente el mismo que perdió el suyo).
- En el mapa se marca como un diamante rojo.

Cifras, todas PLACEHOLDER (`CAMPAMENTOS_BANDIDOS`):
- Tropas: una escuadra de milicia de lanceros con 15 unidades, sin dueño y manejada por la IA (poder 30, fijo, sin escalado por región). Es lo que combate en Unity contra un héroe, y lo que pesa en el cálculo cuando el campamento ataca una caravana.
- Radio de ataque a caravanas: 40 unidades del mapa.
- Recompensa: 40 madera + 20 piedra + 15 oro.
- Reaparición: 60 minutos tras destruirse.

El campamento no bloquea la explotación del bosque que ocupa: solo amenaza a las caravanas de paso.

## 1.10 El héroe está SITUADO en el mundo

El héroe de cada jugador es un partícipe del mundo, no un ente volador superior. En todo momento está en **uno** de tres sitios, nunca en dos y nunca en ninguno:

| Dónde | Qué significa |
|---|---|
| **Dentro de un asentamiento** | Ve su interior completo y puede dar órdenes ahí |
| **En una columna, en el mapa** | Se mueve, ve lo que su columna alcanza, interactúa con lo que se cruza (Doc 5.12) |
| **Desconectado** | Fuera del mundo, con su sitio guardado |

### 1.10.1 Solo ves donde estás, y solo actúas donde estás

**Nunca se ve el interior de un asentamiento en el que no se está físicamente.** Ni siquiera uno propio, ni siquiera siendo su Gobernador. Almacén, guarnición, colas de construcción, cargos y trazado urbano son cosas que se miran desde dentro.

Y la regla es también de ACCIÓN: **construir, reclutar, comerciar, activar políticas y designar cargos exige estar dentro** de la plaza en cuestión (Doc 2.5). Con acción remota el jugador seguiría siendo un ojo volador, solo que con una venda.

**Lo ya ordenado sigue corriendo solo.** Salir no congela tu ciudad: la auto-construcción avanza, las colas terminan, las caravanas en ruta llegan, la producción produce. Lo que no puedes es darle órdenes NUEVAS mientras no estés.

**Lo que se deja atrás se RECUERDA.** Al salir de una plaza queda la última foto de su interior, fechada — el mismo mecanismo con el que se recuerda una ciudad ajena que se dejó de ver (Doc 5.12.8). El jugador juzga si fiarse de un dato de hace tres horas.

> **Esto NO toca la niebla de guerra.** Tus plazas siguen vigilando su radio para toda tu Facción aunque no estés dentro de ninguna, y lo avistado y lo recordado del MAPA siguen igual (Doc 5.12.7-5.12.8). Lo que se restringe son los INTERIORES, no saber dónde están las cosas ni qué pasa en el mundo.

### 1.10.2 Salir al mundo

Se sale desde la **residencia**, que es donde el héroe tiene su campamento con todos los escuadrones que no lleva encima (Doc 5.15.2). Al salir elige, en una sola pantalla:

1. **Con qué tropas sale**, bajo su Liderazgo (Doc 5.11) — puede ser una sola, todas las que el Liderazgo permita, o **ninguna**. Puede partir de un loadout guardado (Doc 5.16.5).
2. **Con qué materiales sale**, hasta llenar su carro (Doc 5.13).

Y aparece en el mapa de mundo **junto al asentamiento**, sin destino todavía.

### 1.10.3 Entrar en un asentamiento

Se entra estando en la **puerta** —a corta distancia de la plaza— y se ofrece como una acción, no ocurre solo (Doc 5.12.3). Lo que pasa con la columna depende de dónde entres:

| Entras en… | Tu columna |
|---|---|
| **Tu residencia** | Se disuelve: los escuadrones vuelven a tu campamento y el carro al almacén |
| **Cualquier otra plaza** | Se queda **aparcada a la puerta**. Al salir la retomas con lo que llevabas, sin ninguna pantalla de equipamiento |

De ahí salen dos consecuencias sin necesidad de más reglas: si conquistan la plaza ajena mientras estás dentro, tu columna está fuera y la retomas; y si te destruyen la columna aparcada, sales a pie.

### 1.10.4 Dentro de una plaza ajena solo se ve la capa pública

Trazado, edificios visibles y mercado. **Nunca** almacén exacto, guarnición, colas de construcción ni cargos.

Entrar es reconocimiento legítimo —ves si la ciudad es grande, rica y está amurallada— y por eso cerrar la puerta sigue siendo una defensa real sin que abrirla sea suicida.

### 1.10.5 La puerta la controla el Gobernador

El Gobernador fija quién puede entrar en su plaza: **abierta a todos**, **solo a su Facción**, **a su Facción y sus aliados**, o **cerrada**. Puede además vetar a jugadores concretos por encima de esa política.

No es una política de las que expiran (Doc 4.4): una puerta que se abre sola a las dos horas y media no es una puerta.

### 1.10.6 Desconectarse

**Al desconectarse, el héroe desaparece del mundo en el punto donde quedó, y reaparece ahí al volver.** No es una excepción caprichosa: es lo que hace jugable un mundo persistente para una persona sola.

**Y se lleva sus tropas con él.** El héroe y todo lo que carga —sus escuadrones y su carro— entran y salen del mundo juntos. Al volver aparece con ellos en el último sitio donde estuvo, lo que le da la oportunidad de alcanzar a los suyos si iba en un ejército.

Si iba en un **Ejército**, este **sigue su marcha sin él**, más débil: mecánicamente, desconectarse es separarse (Doc 5.14.2) y desaparecer. Y si el que se desconecta era el **Líder**, el mando pasa al integrante con más antigüedad (Doc 5.14.3) — el Líder no puede separarse por voluntad propia, pero sí puede caerse la conexión, y la columna no puede quedarse sin mando.

> **Alcanzar de vuelta a tu ejército no siempre se puede.** Una columna va al ritmo de su escuadrón más lento (Doc 5.12.5), así que solo alcanzas a los tuyos si tus tropas son más rápidas que la más lenta de la columna. El que llevaba la tropa pesada que frenaba a todos no vuelve a alcanzarlos.

**Desaparecer tarda dos minutos y medio.** No es instantáneo: al desconectarse, el héroe sigue en el mundo ese rato —moviéndose como iba— y solo entonces se lo lleva todo consigo. Es lo que impide desconectarse para escapar de un combate que ya se tiene encima, sin castigar por ello a quien sufre un corte de verdad: en 2:30 un perseguidor cubre 30-55 unidades, así que alcanza a quien ya tenía a tiro y no a quien iba lejos.

**Si se desconectan TODOS los integrantes de un ejército**, cada uno se lleva lo suyo y las **caravanas adjuntas vuelven solas a su asentamiento de origen** — haciendo el camino, así que son interceptables durante el regreso. No es lo mismo que perderlas: a un ejército DERROTADO se las quita el enemigo (Doc 5.13.2), y a estas no las venció nadie. Si su origen ya no existe, se pierden.
