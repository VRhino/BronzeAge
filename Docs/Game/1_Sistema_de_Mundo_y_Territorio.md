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

**Un reino cómodo mide ~5 provincias**: sale de `MANTENIMIENTO.escalaDistancia` = 400, la distancia a la capital a la
que el coste de mantenimiento se duplica (×2; sigue creciendo, sin tope, Doc 4.5). 400 / 76 ≈ 5.

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
  - El estaño tiene además **yacimientos de frecuencia intermedia, la mitad que el cobre**, que se suman a los raros: sin ellos casi nadie fabricaba bronce y el nivel 3, que lo pide, no llegaba (`ESTANO_EXTRA`, `worldgen/config.ts`).
- Spawn de jugadores nuevos: posición aleatoria uniforme, INDEPENDIENTE de la ubicación de recursos.
- Fases avanzadas (fuera de alcance de Fase 0): mar navegable; posible mapa fijo diseñado a mano en vez de procedural; puntos de interés fijos (ruinas, maravillas); clima.

## 1.2 Fundación de asentamientos
- La posición es LIBRE —sin rejilla ni puntos predefinidos— y se funda donde está el héroe (1.3): el sitio se elige caminando hasta él.
- **Aviso de viabilidad al fundar** (ayuda, no bloqueante): antes de confirmar se dibuja el radio inicial (30 unidades, ver TAMAÑO abajo) coloreado según viabilidad — VERDE si la posición es fundable, hay un bosque LIBRE alcanzable dentro del radio (madera garantizada desde el principio) **y al menos un nodo de piedra**; ÁMBAR si es fundable pero le falta alguna de las dos; ROJO si la posición no es válida (fuera del mapa, solapa una zona de influencia existente, cima o agua). No impide fundar en ámbar: es solo información para decidir mejor. También indica qué nodos minerales caen dentro del radio inicial.
  - **Por qué la piedra cuenta** (2026-09-26): sin piedra no hay Cantera, sin Cantera no hay tercer tipo de extracción para el nivel 2 (Doc 4.5), y sin nivel 2 la zona no crece hasta alcanzar piedra más lejos — el asentamiento nace condenado a nivel 1. Los NPC aplican el mismo criterio al fundar y al expandirse.
- Al fundar se genera automáticamente una ZONA DE INFLUENCIA (círculo/polígono) = nodo.
- TAMAÑO: nace con un radio inicial de 30 unidades y crece gradualmente cada minuto hacia un TECHO que escala con el NIVEL del asentamiento (Doc 4.5): nivel 1 → 60, 2 → 90, 3 → 120, 4 → 150, 5 → 180. Subir de nivel no hace saltar el radio de golpe — solo levanta el techo hacia el que la zona ya venía creciendo.
- Reglas de construcción: solo se puede construir dentro de una zona de influencia existente (ampliándola si es del mismo bando) o fuera de cualquier zona (creando una nueva).
- FRONTERAS: al chocar dos zonas en expansión se genera un LÍMITE DURO — ninguna zona sigue creciendo en esa dirección. Solo se rompe si el asentamiento rival cae o su zona se debilita/reduce (guerra u otros medios). Las fronteras son "vivas", reflejan el poder relativo de cada bando en cada momento.

## 1.3 Onboarding de nuevos jugadores
- **El héroe nace DENTRO de un campamento de mercenarios (1.9b) que elige**, como residente, sin columna en el mapa. La pantalla de elección muestra todos los campamentos con dos cifras: cuántos lo eligieron al nacer y cuántos residen ahora (solo informativas, sin tope). Desde allí sale al mundo cuando quiere.
- **Fundar (2026-10-04, D7, D19, D30)**: **solo con una Caravana de Fundación**; no se funda a pie. Una Facción sin asentamientos la compra en un campamento de mercenarios con el fondo de sus héroes (1.8, 1.9b); quien ya tiene plaza la lanza desde ella (1.8). La del campamento se lleva enganchada a la columna y **se funda DONDE SE ESTÁ** (`fundar`): caminar hasta un buen emplazamiento es la primera decisión de la Facción, y es lo que da sentido a explorar antes de asentarse.
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
- FUNDACIÓN GRUPAL: los ciudadanos de la Facción que van en la columna del titular cuando funda son cofundadores, hasta 5 (M2: unirse a su columna es el consentimiento). Todos reciben casa en la plaza nueva.
- MATERIALES INICIALES: la caravana de fundación entrega una reserva inicial de recursos al fundar, suficiente para arrancar la primera construcción. Sin ella el asentamiento quedaría bloqueado para siempre: el edificio que produce madera también cuesta madera. Cifras (`FUNDACION.materialesIniciales`/`POBLACION.pesants.inicial`, placeholder): 50 madera + 20 piedra + 100 trigo + 100 oro, y población inicial de 20 pesants.
- PROTECCIÓN TEMPORAL: tras la fundación hay un período de gracia durante el cual NO se cobra Mantenimiento (ver Doc 4.5), para que la economía pueda arrancar antes de pagarlo. Duración placeholder. Y durante **un día** nadie puede asediarlo: la misma protección que tiene una plaza recién conquistada (Doc 5.12.9), para que reclute con qué defenderse.

## 1.4 Fuentes de recursos por tipo
- CULTIVOS (trigo): NO son un nodo recolectable directo. Dependen de la FERTILIDAD DEL SUELO de la zona (atributo de terreno); requieren construir una Granja para aprovecharse.
- MADERA: proviene de BOSQUES, representados como ZONAS del mapa (no puntos), con densidad variable.
- MINERALES (cobre, estaño, hierro, oro): CONDICIONADOS AL RELIEVE — cobre, estaño y hierro en colina o montaña, oro solo en montaña (el más exclusivo, es el más raro); piedra permisiva (colina/montaña/llanura fértil/estepa) para no comprometer el recurso común más consumido (`RECURSO_BIOMA_PERMITIDO`, `worldgen/config.ts`). **El mineral de hierro es mucho más abundante que el estaño: sus yacimientos son 1,5 veces más frecuentes que los de cobre.** Cada uno tiene su propio edificio de extracción: cantera para piedra, mina de oro, mina de cobre, mina de estaño, mina de hierro.
- LIVESTOCK = **ganado vacuno**: fauna LIBRE en el mapa, se aprovecha con el Corral. Da el cuero de la Curtiduría. Cría/domesticación pospuesta a fases avanzadas (se trata como recurso consumible/finito).
- CABALLOS: no son livestock. **Se compran con oro** al reclutar caballería y carros (Doc 5.8), y los animales de tiro de las caravanas se compran igual (Doc 3.13). Cuando exista la captura y la cría habrá las dos fuentes.
- MADERA y PIEDRA: materiales base de construcción de edificios e insumo de materiales derivados.
- REGENERACIÓN DE YACIMIENTOS AGOTADOS: un nodo (mineral o livestock) que llega a stock 0 no queda muerto para siempre — vuelve a aparecer con su cantidad inicial completa pasado un cooldown, parametrizable por categoría (`REGENERACION_NODOS`). Livestock (fauna, se recupera por reproducción/migración) regenera al DOBLE de rápido que los yacimientos minerales (piedra/cobre/estaño/hierro/oro) — un filón agotado tarda mucho más en "rellenarse" que una manada.
- COMMODITIES DE NOBLEZA (uvas/olivas → vino/aceite): no son alimento básico, requeridas para felicidad de la Nobleza; su déficit arriesga rebelión/estancamiento, no hambruna.

## 1.5 Chokepoints estratégicos (heredado de Iberia)
Puertos de montaña detectados como PUNTOS DE SILLA del campo de elevación (mínimo local a lo largo de la cresta, máximo local en la dirección perpendicular — geometría determinista por seed). El asentamiento cuya zona de influencia CUBRE un chokepoint lo controla; las caravanas comerciales de una Facción rival cuya ruta pasa cerca pagan un PEAJE EN ORO al controlador, cobrado al llegar a destino (ver `Consideraciones/Fase_0_3_Definicion.md`). Solo los puertos de montaña son chokepoints; los vados de río no.

## 1.6 Red de caminos
Los caminos no se construyen: **los hace el uso**. Diseño y decisiones en `Consideraciones/Rutas_Caravana_Avanzadas_Definicion.md`. Cifras placeholder.

**El terreno por el que se viaja** (caravanas, ejércitos y fundación, con el mismo pathfinding):
- **El agua no se cruza** (1.0b).
- **Los ríos tampoco, salvo por un vado.** El tramo alto, cerca del nacimiento, es arroyo y se cruza por cualquier sitio; aguas abajo hay un vado cada ~300 u en el mapa de 2000 (~2 km en el regional). Una ciudad a orillas del río es también paso: el río se cruza por la ciudad. Los vados salen de la geometría del río, no del azar.
- **El bosque frena, no bloquea**: dentro de un bosque el coste de moverse se multiplica por `1 + 2 × densidad`, así que un bosque denso cuesta casi el triple. Las rutas lo rodean cuando el rodeo compensa.

**Los caminos.**
- Una **ruta** es el par origen → destino de una caravana **comercial** (por trueque o lanzada a mano). Cada lanzamiento traza la ruta y la registra en la red: los tramos que recorre pasan a ser camino. Dos rutas que pisan el mismo tramo lo comparten — los caminos se fusionan solos.
- El **peso** de un tramo son las rutas vigentes que lo recorren. Una ruta deja de estar vigente si su par pasa 3 días sin lanzar o desaparece uno de sus asentamientos. Un tramo **nunca desaparece**: sin rutas queda como sendero (infraestructura permanente).
- Se pinta en tres escalones por peso: **sendero** (0-1), **camino** (2-5), **calzada** (6+).
- **Atracción**: al trazar, un tramo con peso cuesta menos (× `max(0,5, 1 − 0,1 × peso)`). Las rutas nuevas se desvían hacia los caminos principales, y como el trazado de un par se recalcula en cada lanzamiento, las rutas viejas migran solas a ellos.
- **Velocidad**: sobre cualquier tramo de la red se avanza al doble (`factorCamino` 0,5), tramo a tramo — no el viaje entero. Mejorable con la política "Rutas Rápidas" (Tesorero, Doc 3.12).
- **Ejércitos: terreno sí, red no.** Marchan más rápido sobre un camino, pero no lo crean ni le suman peso, y no pagan peaje. Las caravanas de fundación tampoco cuentan como ruta.

**Paso por ciudades** (Doc 3.6, 3.10):
- Si el trazado de una caravana comercial cruza la zona de influencia de un asentamiento **ajeno** (de otra Facción), se fuerza el paso por esa ciudad y la caravana deja allí un **peaje en especie del 2 % de cada recurso que lleve**, sin pararse. Varias ciudades en el camino: se pasa por todas, en orden. A la vuelta va vacía y no paga. Una ciudad propia no fuerza nada.
- **Inmunidad**: dentro de una zona de influencia que no sea del atacante, una caravana no puede ser atacada — ni por bandidos (que no tienen zona: cualquier zona protege) ni interceptada por una columna. La zona propia del atacante no es refugio.

**Logro de `logistica_campana`** (Doc 6, Era IV; BA-006 D30): existe un tramo **fuera de toda zona de influencia** recorrido por al menos **10 rutas de 3 Facciones distintas**.

## 1.7 Cap de fundación de asentamientos por Facción (inspirado en Rise of Nations)
- Límite DURO de asentamientos que una Facción puede FUNDAR (no aplica a conquista/anexión, que no tiene límite). Fundar por encima del cap se rechaza.
- ESCALA con el NIVEL DE FACCIÓN: progresión fácil de 1 a 3, luego se complica progresivamente hasta un MÁXIMO DE 7 en etapa tardía. Curva (`CAP_FUNDACION_POR_NIVEL`, placeholder): `[1, 2, 3, 3, 4, 5, 5, 6, 6, 7]` para niveles de Facción 1 a 10.
- Complementa (no sustituye) al sistema de Mantenimiento/Coste de Gobernanza (ver Doc 4).
- Da incentivo mecánico a preferir vasallaje/conquista sobre fundación directa una vez alcanzado el cap.
- **Qué hace subir el nivel de Facción**: la experiencia acumulada (combate digno, conquista, edificio completado, subir de nivel un asentamiento, fundar, cerrar trueques y, mientras la Facción es joven, destruir campamentos de bandidos). Ver Doc 2.2.1 para el criterio completo, la curva de umbrales y el cupo de asentamientos nivel 2/3 que también depende de este nivel.

## 1.8 Caravana de Fundación (mecanismo de expansión más allá del primer asentamiento)

Mecanismo COMPLEMENTARIO al Cap de Fundación (1.7) — ambos coexisten, no se sustituyen. El objetivo es que fundar un asentamiento adicional cueste recursos reales y se sienta orgánico, evitando fundación en cadena sin fricción.

**COSTE** = suma de tres componentes:
1. Los materiales iniciales que recibe todo asentamiento nuevo al fundar (ver 1.3).
2. El coste de construcción de los edificios que nacen automáticamente con la fundación (Centro Urbano, Granja, 3 Viviendas — ver Doc 4.2.1).
3. +50 de madera extra, representando el coste de fabricar la caravana en sí (placeholder).

**Desde un campamento de mercenarios** (2026-10-02; rehecho el 2026-10-04, D9-D16, D30, D34, D39, D40, D68; 1.9b): una Facción sin asentamientos, que no tiene origen del que partir, la compra en un campamento al 75 % del coste, con el fondo que aportan sus héroes **en ese campamento** (lo traen en su almacén personal si residen allí, o en el carro). Los gates de origen de abajo no aplican; sí el Cap de Fundación.
- **Nace sin destino**, parada en el campamento, y su **titular** (quien la compra) la engancha a su columna. Los demás escoltan uniéndose a esa columna.
- **Se funda con `fundar`** donde esté la columna: fuera de toda zona de influencia, no sobre agua y a **100** o más de cualquier campamento (`MERCENARIOS.radioExclusionFundar`). Es el mismo mecanismo con que funda la caravana de una plaza al llegar (D30).
- Lleva su Facción: solo un ciudadano de ella la engancha; los demás solo pueden atacarla, y su origen es el campamento. **Si el titular deja de llevarla** (sale del mundo, deja la Facción, o la lleva otro), se suelta y **vuelve sola** a su campamento; otro ciudadano puede **reclamarla** por el camino y pasa a ser el titular. Si llega sin nadie, **se desarma**.
- **Suelta y sin nadie, caduca a las 48 h** (`MERCENARIOS.caducidadCaravanaHoras`). Al desarmarse o caducar **devuelve a cada aportante lo suyo** (la caravana apunta quién aportó qué): a su almacén personal, el oro como oro de botín; lo que no cabe, o lo de quien ya no es ciudadano, se pierde. Destruida, se pierde. No hay espera para comprar otra.

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

Una sola entidad con **tres niveles**, que sale **al azar** al aparecer (2026-10-04, D21, D37). Aparecen por dos vías:

- **Uno por asentamiento, SIEMPRE**: cada asentamiento vivo tiene el suyo, en SU bosque no reclamado MÁS CERCANO (un bosque que NO se solapa con ninguna zona de influencia; se comprueba el CENTRO del bosque contra los polígonos de zona), y queda ASIGNADO a él. Así nunca aparece en la otra punta del mapa sin ningún asentamiento cerca, ni pegado a una zona de influencia. **Cada asentamiento lleva su propio plazo** de reaparición: tras destruirse el suyo, le reaparece uno cuando vence, sea quien sea quien lo destruyó.
- **En el anillo de cada campamento de mercenarios, según la demanda** (D42, D28): a la misma distancia en todos (entre 150 y 250), en tierra firme, fuera de toda zona y de la protección de cualquier campamento. Hay **uno por cada 2 residentes de Facciones sin asentamiento** (o sin Facción), entre 1 y 6; mientras falten, aparece uno cada 10 minutos.

- Mientras el campamento sigue en pie, ATACA CARAVANAS (Doc 3.10) que pasen dentro de un radio fijo de su posición, cada minuto — se resuelve con números, contra la defensa de la caravana. Si gana, la caravana se pierde por completo (nadie la recibe: el bandido no tiene almacén propio). Junto a un campamento de mercenarios no ataca (1.9b).
- **Se ataca con una columna que llegue a él** (Doc 5.12.3), y la batalla se juega en Unity: los héroes de la columna contra las tropas del campamento, manejadas por la IA del juego (Doc 5.15.6). SIN gate de cargo: a diferencia de un asedio, un campamento bandido es una amenaza de mundo abierto, no una acción de guerra entre Facciones. Si pierde, sus héroes quedan heridos (5.16.4) y la columna pierde la mitad del carro, que no se lleva nadie. Si el servidor no tiene conectado ningún servidor de batalla de Conquest, el choque se resuelve con números: los soldados de los héroes sanos contra el poder del campamento.
- **El botín es solo oro** (D22): al destruirlo, **cada héroe de la columna** recibe el oro de su nivel. Va a su **oro de botín**, aparte del almacén personal, que **solo se gasta en el mercado de un campamento o en el fondo de refundación** (D27): nunca llega a la economía de una plaza. Lo que se retira del fondo vuelve como oro de botín.
- **Rendimientos decrecientes por héroe** (D26): en las últimas 24 h, botín completo en las 8 primeras destrucciones, luego un 15 % menos por cada una, y desde la 14.ª solo experiencia.
- En el mapa se marca como un diamante rojo.

Cifras, todas PLACEHOLDER (`CAMPAMENTOS_BANDIDOS`):

| Nivel | Sale | Poder | Tropa en Unity | Oro por héroe |
|---|---|---|---|---|
| 1 | 50 % | 20 | 10 milicias de lanceros | 9 |
| 2 | 30 % | 60 | 30 | 10 |
| 3 | 20 % | 120 | 60 | 11 |

- Radio de ataque a caravanas: 40 unidades del mapa.
- Reaparición del de un asentamiento: 60 minutos tras destruirse.

El campamento no bloquea la explotación del bosque que ocupa: solo amenaza a las caravanas de paso.

## 1.9b Campamentos de mercenarios (decidido el 2026-10-02; los 5 pasos implementados)

Entidad **neutral** del mundo abierto, con miniatura en el mapa. No es de ninguna Facción, no se puede atacar, no crece y no desaparece. **No es un asentamiento**: no tiene zona de influencia, y ninguna zona lo absorbe (las zonas salen de los asentamientos). Es un enclave.

- **Colocación.** Todos existen **desde que se crea el mundo** y no aparecen más (`colocarCampamentosIniciales`): puntos al azar en tierra firme, pegados a un bosque si lo hay a menos de `MERCENARIOS.margenBosque`, a una **distancia mínima** entre ellos (`MERCENARIOS.separacion`: 900 en el playtest, unos 4 en un mapa de 2000; 600 para más gente, de 6 a 9) y a `margenBorde` del borde. El azar sale de una semilla derivada de la del mapa: misma seed, mismos campamentos.
- **Forma.** Layout fijo, que no cambia: taberna, dos viviendas y mercado, más **un edificio militar** (barracón, galería de tiro o caballeriza) que se elige al nacer. Hay **3 variantes de aspecto** (el *origen*), sin efecto sobre lo que hace. Aspecto y edificio militar salen de la posición, no del azar de la partida.
- **Niebla.** Se conoce **como un camino**: si la Facción ha explorado alguna vez el terreno donde está, lo recuerda; no hace falta verlo ahora, porque un campamento no se mueve ni desaparece. El campamento donde reside o donde está dentro lo conoce siempre.
- **Dentro.** El campamento tiene un «dentro», como una plaza: el héroe está en su vista de asentamiento (edificios y layout). Dentro no se come ni hay combate. **Se ve como desde dentro de una plaza**: el mapa de campaña con la visión compartida de su Facción y aliados (quien no tiene Facción ni columna fuera no ve nada vivo). **Cualquier héroe puede entrar en cualquier campamento**, con su columna en la puerta (`entrarEnCampamento`, a `MOVIMIENTO.radioPuerta`):
  - en **el suyo**, la columna se deshace: la tropa queda en el campamento y el carro se vacía en el almacén personal; lo que no cabe se queda en el carro, aparcado en la puerta;
  - en **otro**, entra con su columna, que queda en la puerta intacta.
  Al salir (`salirDelCampamento`), en el suyo elige la tropa y lo que carga desde el almacén personal; en otro, retoma la columna con la que entró. Como al movilizar desde una plaza, puede salir **admitiendo compañía** (`politicaDeUnion`): entonces sale como ejército, **con su destino fijado al salir** (el de un ejército no se cambia, Doc 5.12.1), y otros ciudadanos de su Facción se le unen en campo —para cazar en grupo o cofundar—. Se entra solo: un ejército con más gente se separa antes. Residir no exige estar allí.
- **Las acciones del campamento** (reclutar, comprar, el fondo de refundación) se hacen dentro o con la columna en su puerta.
- **Tropa prestada.** El campamento presta a su residente, **gratis**, tropa de leva comunal: **una escuadra de 15 de cada tropa que elija** —milicia de lanceros, leñadores, granjeros; una, dos o las tres— (`pedirPrestamo`, `MERCENARIOS.prestamo`). Es para aprender a usar tropa antes de tener la propia. **No es tropa del héroe**: no gana experiencia ni cuenta para «una escuadra por tropa» (Doc 5.8), y el campamento la retira si deja de residir en él. No come mientras vive en el campamento. Si pierde gente se repone **gratis** en el campamento o con la columna a su puerta (`reponerPrestamo`).
- **Ración gratis del residente.** Al salir de su campamento, el residente recibe en el carro el trigo que **su columna come en 45 minutos de marcha** —la tropa con la que sale más él— (`MERCENARIOS.racion`, hasta llenar el carro), una vez cada 30 minutos de mundo; no se acumula. La columna se come la ración la primera. Lo que queda de ella **vuelve al campamento** al entrar y no se puede guardar en el almacén personal: sirve para moverse, no para ahorrar.
- **Alijos de exploración** (D29, D60-D63). Escondites de oro colocados al crear el mundo alrededor de cada campamento (`colocarAlijos`, `ALIJOS`): 3 a 100-200 con 3 de oro, 2 a 200-300 con 5 y 1 a 300-450 —en los huecos entre campamentos— con 11; siempre con ese campamento como el más cercano, en tierra firme y fuera de su protección. **Son por héroe**: cada uno abre cada alijo una vez y no reaparecen, así que nadie le quita uno a otro. Solo los ven y los abren los héroes de **Facciones sin asentamiento** (o sin Facción): un alijo **se ve** cuando está a la vista de su columna (`VISION.jugadorSolo`, 80) y no lo ha abierto; **se abre** estando en el sitio (`abrirAlijo`). Su oro va al **oro de botín** (1.9).
- **Protección.** A menos de `MERCENARIOS.radioProteccion` (60) de un campamento nadie inicia un combate: ni jugadores (`atacar` se rechaza desde allí y contra lo que está allí) ni bandidos.
- **Residencia** (paso 2): cualquier héroe puede residir en él, y es la casa de quien se queda sin asentamiento; se acaba el huérfano (Doc 2.5, 5.15.5). Almacén personal de cada héroe: Doc 2.5.
- **Reclutamiento** (paso 3): reclutar y reponer las tropas de sus edificios, con tecnología propia, pagando oro y gastando la población del campamento (Doc 5.8, 6.5b). Población: viviendas × 50, recupera 10 por hora.
- **Mercado** (paso 4): solo vende, con stock limitado, y el oro cobrado se destruye (Doc 3.3b). **Madera, piedra y trigo tienen pila propia** (`MERCENARIOS.mercado.pilas`: 300, 60 y 300), que vuelve a su tope en cada reposición sin depender del comercio del mundo; los demás bienes se reponen con lo que se comercia. **Cupo por héroe y día** en madera (60) y piedra (10). Quien reside compra a su almacén personal; **cualquier otro héroe solo compra trigo, para repostar, y le va al carro** de su columna.
- **Refundar** (paso 5; rehecho 2026-10-04): una Facción **sin asentamientos** compra en el campamento una **Caravana de Fundación al 75 %** del coste normal; cómo se lleva, se funda, vuelve y caduca, en 1.8. Cada ciudadano aporta lo que quiere al **fondo de su Facción en ese campamento** —desde su almacén personal si reside allí, o desde el carro de su columna en la puerta (`aportarARefundacion`)— y lo suyo se retira mientras no se gaste (el oro, como oro de botín); cuando el fondo cubre el coste, uno de ellos la compra (`comprarCaravanaDeRefundacion`) y es su titular.

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

### 1.10.5 La puerta: el Gobernador y el Rey (el exilio)

Los héroes que llegan a una plaza se agrupan por la relación entre su Facción y la de la plaza: **la propia Facción** (nunca se le cierra la puerta), **aliados** (alianza o vasallaje, en cualquier sentido), **enemigos** (en guerra, Doc 2.4.1) y **neutrales** (el resto). La puerta se cierra **por grupo**, y cada uno es un interruptor independiente: no dejar entrar a neutrales, no dejar entrar a aliados, no dejar entrar a enemigos. Una plaza que nadie ha tocado deja entrar a los suyos y a los aliados, y cierra a neutrales y enemigos.

Lo fija el **Gobernador de la plaza** o **el Rey de su Facción**, plaza por plaza (no hay una puerta de Facción): es un único estado y cualquiera de los dos puede cambiarlo, manda el último. No cuesta nada y no caduca; se levanta dando acceso otra vez al grupo. Es el exilio de la Doc 2.8. Además, el Gobernador puede vetar a jugadores concretos por encima de los grupos (el veto sirve también para uno de la propia Facción).

No es una política de las que expiran (Doc 4.4): una puerta que se abre sola a las dos horas y media no es una puerta.

### 1.10.6 Desconectarse

**Desconectarse quita el control, no el sitio** (2026-10-04, D64-D71). El héroe desconectado no da órdenes ni participa en nada; **a un jugador que no está no se le puede cazar**. Como el servidor no distingue una desconexión a propósito de una caída de red o de luz, no se castiga: se hace **inútil para huir**.

**Desconectado es tener el cliente cerrado.** Abrirlo te trae al mundo; cerrarlo —o perder la conexión— te desconecta. Con el juego abierto en dos sitios, cerrar uno no te desconecta. Apagar el servidor no desconecta a nadie: no es que los jugadores se hayan ido.

**Dentro de una plaza o de un campamento de mercenarios no se mueve nada.** El héroe sigue donde está y sus escuadras también; solo deja de dar órdenes y **no defiende en persona**. La defensa no cambia por eso: la guarnición defiende siempre con la IA, las escuadras libres no defienden, y un héroe conectado, dentro y sano entra a defender con su loadout (Doc 5.15). Al volver, sigue ahí.

**Con la columna en el mapa, sale del mundo con ella**: el héroe y todo lo que carga —sus escuadrones, la tropa prestada por un campamento y su carro— entran y salen del mundo juntos. Al volver aparece con ellos en el último sitio donde estuvo, lo que le da la oportunidad de alcanzar a los suyos si iba en un ejército. **Lo cedido se queda**: la guarnición sigue en su plaza y la escolta cedida a una caravana sigue con ella.

Si iba en un **Ejército**, este **sigue su marcha sin él**, más débil: mecánicamente, desconectarse es separarse (Doc 5.14.2) y desaparecer. Y si el que se desconecta era el **Líder**, el mando pasa al integrante **conectado** con más antigüedad (Doc 5.14.3) — el Líder no puede separarse por voluntad propia, pero sí puede caerse la conexión, y la columna no puede quedarse sin mando.

> **Alcanzar de vuelta a tu ejército no siempre se puede.** Una columna va al ritmo de su escuadrón más lento (Doc 5.12.5), así que solo alcanzas a los tuyos si tus tropas son más rápidas que la más lenta de la columna. El que llevaba la tropa pesada que frenaba a todos no vuelve a alcanzarlos.

**Salir del mundo tarda dos minutos y medio.** Al desconectarse, el héroe sigue en el mundo ese rato —moviéndose como iba— y solo entonces sale. Si vuelve antes, no llega a irse. **En peligro no se sale**: si al cumplirse ese plazo una columna le está **persiguiendo**, la salida se aplaza mientras dure, y como mucho hasta los **5 minutos** desde que se desconectó (`PRESENCIA.topeAplazamientoMs`); si mientras tanto le alcanzan, su columna combate con la IA. **Quien está en una batalla no sale hasta que la batalla termina.** Así quien huye desconectándose no gana nada, y a quien se le cae la red le pasa, como mucho, lo mismo que si se hubiera quedado quieto un momento.

**Si se desconectan TODOS los integrantes de un ejército**, cada uno se lleva lo suyo y las **caravanas adjuntas vuelven solas a su origen** — haciendo el camino, así que son interceptables durante el regreso. No es lo mismo que perderlas: a un ejército DERROTADO se las quita el enemigo (Doc 5.13.2), y a estas no las venció nadie. Si su origen ya no existe, se pierden. La Caravana de Fundación de un campamento vuelve a él y, si llega sin nadie, se desarma y devuelve lo aportado (1.8). Un Rey desconectado no delega: las solicitudes de ingreso esperan.
