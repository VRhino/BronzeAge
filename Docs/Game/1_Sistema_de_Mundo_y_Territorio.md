# 1. Sistema de Mundo y Territorio


## 1.0a La escala del mundo

El motor maneja **dos espacios separados**: el **mapa del mundo** (2.667×2.000 unidades, el mundo regional a factor 12) y el espacio local de la **Vista de Asentamiento**. No hay equivalencia entre sus unidades (decidido el 2026-10-12): el trazado de una ciudad se mide en el suyo y la zona de influencia en el del mapa, y ninguna regla convierte de uno a otro.

La jerarquía territorial de la ficción, en unidades de mapa:

| | radio | equivale a |
|---|---|---|
| **Provincia** (zona de influencia) | **6-30**, según nivel | lo que controla un asentamiento |
| **Reino** | todas las provincias de una Facción | sin número fijo: una Facción podría llegar a todo el mapa |

**Separación entre plazas.** Dos plazas distan **30 o más** de centro a centro: un minuto de camino a velocidad ligera (30). La zona inicial (radio 6) no solapa ninguna otra zona; las zonas grandes se recortan al crecer (fronteras duras, 1.2) y los recursos accesibles se cuentan sobre la zona recortada.

**Cuántas provincias caben.** El mundo mide 2.667×2.000 y es mayoritariamente tierra firme en el mundo regional (en torno a dos tercios, el resto agua). El número de plazas no es una meta de diseño: lo fija la separación mínima, no una cuota.

**Un reino cómodo mide ~5 provincias**: sale de `MANTENIMIENTO.escalaDistancia` = 80, la distancia a la capital a la que el coste de mantenimiento se duplica (×2; sigue creciendo, sin tope, Doc 4.5).

**La sensación de distancia.** La calibración es de densidad, no de escala: el mundo mantiene su tamaño y las plazas se juntan. Con la tropa ligera a 30 unidades por minuto (0,5 por segundo en tiempo real), la plaza vecina está a 1 minuto, el radio de una zona de nivel 2 a 30 s y cruzar el mapa de lado lleva 89 minutos (111 en diagonal). Con el zoom de juego (15 de ancho) una pantalla se cruza en unos 30 s.

## 1.0b El agua es un OBSTÁCULO

**Ni los ejércitos ni las caravanas pueden moverse sobre agua.** No es terreno caro: es infranqueable. Todo lo que se desplaza por el mapa —caravanas de comercio, caravanas de fundación, ejércitos y el trazado de los caminos comerciales— la rodea o no llega. Nadie camina sobre el agua.

Tres consecuencias:

1. **Un destino puede quedar SIN RUTA.** Una isla, una península cortada. Si no hay camino por tierra, no hay ruta, y quien la pidió decide qué hacer — no se moviliza el ejército, no sale la caravana, no se traza el camino comercial. Nunca se cae a una línea recta: sería precisamente una ruta por el mar.
2. **No se puede fundar sobre agua**: el asentamiento quedaría incomunicado para siempre. El agua y la cima son terreno inhabitable.
3. **La cima es transitable**, solo que cara (coste 12): es terreno difícil, no un medio distinto.

**Los RÍOS son una entidad aparte, no terreno**: se cruzan solo por un vado o por una ciudad ribereña (1.6).

El comercio por mar está fuera de alcance (Doc 3.11): sin barcos, dos costas enfrentadas no comercian.

## 1.1 Generación del mundo (Fase 0)
- Mapa RECTANGULAR (2.667×2.000), espacio de coordenadas continuo (no grid discreto).
- Tamaño base: 2.667×2.000 unidades (el mundo regional a factor 12), PARAMETRIZABLE.
- Terreno con RELIEVE REAL: campo de ELEVACIÓN continuo por ruido fractal, clasificado en bandas agua/costa/llano/colina/montaña/cima; RÍOS como polilíneas que nacen en montaña y descienden por gradiente de máxima pendiente; BIOMA derivado de terreno+fertilidad+humedad — ver `typescript-final:Consideraciones/Fase_0_1_Definicion.md`. El coste de moverse por el mapa depende del tipo de terreno (ver 1.6 y Doc 3.6). Opcionalmente sesgado a una región geográfica real (Grecia continental/Anatolia/Egeo/Nilo/Mesopotamia — "Libre", sin sesgo, es el valor por defecto). Sin mar navegable (eje naval pospuesto, ver `typescript-final:Consideraciones/Roadmap_Escalado.md`).
- Generación PROCEDURAL de recursos por niveles de rareza:
  - Común (alta frecuencia, disperso): Madera (bosques, ver 1.4), Piedra, Trigo (vía fertilidad, ver 1.4).
  - Intermedio (frecuencia media, varios clusters): Cobre.
  - Raro (baja frecuencia, pocos clusters, espaciado mínimo forzado entre ellos): Estaño, Oro.
  - El estaño tiene además **yacimientos de frecuencia intermedia, la mitad que el cobre**, que se suman a los raros: sin ellos casi nadie fabricaba bronce y el nivel 3, que lo pide, no llegaba (`ESTANO_EXTRA`, `worldgen/config.ts`).
- Fases avanzadas (fuera de alcance de Fase 0): mar navegable; posible mapa fijo diseñado a mano en vez de procedural; puntos de interés fijos (ruinas, maravillas); clima.

## 1.2 Fundación de asentamientos
- La posición es LIBRE —sin rejilla ni puntos predefinidos— y se funda donde está el héroe (1.3): el sitio se elige caminando hasta él.
- **Aviso de viabilidad al fundar** (ayuda, no bloqueante): antes de confirmar se dibuja el radio inicial (6 unidades, ver TAMAÑO abajo) coloreado según viabilidad — VERDE si la posición es fundable, hay un bosque LIBRE alcanzable dentro del radio (madera garantizada desde el principio) **y al menos un nodo de piedra**; ÁMBAR si es fundable pero le falta alguna de las dos; ROJO si la posición no es válida (fuera del mapa, solapa una zona de influencia existente, cima o agua). No impide fundar en ámbar: es solo información para decidir mejor. **Sin madera suficiente a 10 unidades o menos** (el techo de la zona en el nivel 1: ninguna parcela de bosque libre o menos de 3 de madera por minuto si se explotase todo el bosque, 5 × la suma de densidades de sus parcelas; cifra placeholder a calibrar) el aviso añade que la plaza **caerá en ruinas**: sin Leñera no hay madera para el mantenimiento y la zona no crece lo bastante para alcanzar otro bosque (en el batch de población, decidido el 2026-10-18, el 29,7 % de las plazas fundadas a ciegas caía el día 1-2). Los bots y los NPC no fundan en un sitio así. También indica qué nodos minerales caen dentro del radio inicial.
  - **Por qué la piedra cuenta** (2026-09-26): sin piedra no hay Cantera, sin Cantera no hay tercer tipo de extracción para el nivel 2 (Doc 4.5), y sin nivel 2 la zona no crece hasta alcanzar piedra más lejos — el asentamiento nace condenado a nivel 1. Los NPC aplican el mismo criterio al fundar y al expandirse.
- Al fundar se genera automáticamente una ZONA DE INFLUENCIA (círculo/polígono) = nodo.
- TAMAÑO: nace con un radio inicial de 6 unidades y **solo crece por obra**, nunca por tiempo: cada edificio completado la agranda 1 unidad, hasta un TECHO que escala con el NIVEL del asentamiento (Doc 4.5): nivel 1 → 10, 2 → 15, 3 → 20, 4 → 25, 5 → 30. Subir de nivel no hace saltar el radio de golpe — solo levanta el techo hasta el que los edificios pueden seguir agrandando la zona.
- Reglas de construcción: solo se construye dentro de la zona del propio asentamiento. Una zona nueva nace únicamente al fundar (1.8).
- FRONTERAS: al chocar dos zonas en expansión se genera un LÍMITE DURO — ninguna zona sigue creciendo en esa dirección. Las zonas nunca se encogen: la frontera solo se mueve cuando crece un radio y se levanta únicamente si la plaza rival cae o se arruina. Las fronteras son "vivas", reflejan el poder relativo de cada bando en cada momento.

## 1.3 Onboarding de nuevos jugadores
- **El héroe nace DENTRO de un campamento de mercenarios (1.9b) que elige**, como residente, sin columna en el mapa. La pantalla de elección muestra todos los campamentos con dos cifras: cuántos lo eligieron al nacer y cuántos residen ahora (solo informativas, sin tope). Desde allí sale al mundo cuando quiere.
- **Fundar (2026-10-04, D7, D19, D30)**: **solo con una Caravana de Fundación**; no se funda a pie. Una Facción sin asentamientos la compra en un campamento de mercenarios con el fondo de sus héroes (1.8, 1.9b); quien ya tiene plaza la lanza desde ella (1.8). Es la misma caravana: se lleva enganchada a la columna y **se funda DONDE SE ESTÁ** (`fundar`): caminar hasta un buen emplazamiento es la primera decisión de la Facción, y es lo que da sentido a explorar antes de asentarse.
- **Nadie nace asentado (D53, 2026-10-04).** El servidor arranca sin Facciones: todas nacen por el mismo camino,
  campamento → caravana → `fundar`. Los **héroes bot** (5.15.6) llegan escalonados a los campamentos como llegarían
  los jugadores, conviven con ellos y cuentan como residentes (D54, D56); un mundo con Facciones ya puestas falsearía
  lo que pasa de verdad en el servidor.

  El peligro de base lo dan los **bandidos** (1.9), que atacan por su cuenta y no son de nadie. El reparto es
  **bandidos la amenaza, los demás héroes —humanos o bots— los vecinos y la guerra**.
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
- MINERALES (cobre, estaño, hierro, oro) — **solo se explotan con un pueblo extractor** (Doc 4.2.2; también toda Cantera después de la primera, la primera es directa) —: CONDICIONADOS AL RELIEVE — cobre, estaño y hierro en colina o montaña, oro solo en montaña (el más exclusivo, es el más raro); piedra permisiva (colina/montaña/llanura fértil/estepa) para no comprometer el recurso común más consumido (`RECURSO_BIOMA_PERMITIDO`, `worldgen/config.ts`). **El mineral de hierro es mucho más abundante que el estaño: sus yacimientos son 1,5 veces más frecuentes que los de cobre.** Cada uno tiene su propio edificio de extracción: cantera para piedra, mina de oro, mina de cobre, mina de estaño, mina de hierro.
- LIVESTOCK = **ganado vacuno**: fauna LIBRE en el mapa, se aprovecha con el Corral. Da el cuero de la Curtiduría. Cría/domesticación pospuesta a fases avanzadas (se trata como recurso consumible/finito).
- CABALLOS: no son livestock. **Se compran con oro** al reclutar caballería y carros (Doc 5.8), y los animales de tiro de las caravanas se compran igual (Doc 3.13). Cuando exista la captura y la cría habrá las dos fuentes.
- MADERA y PIEDRA: materiales base de construcción de edificios e insumo de materiales derivados.
- REGENERACIÓN DE YACIMIENTOS AGOTADOS: un nodo (mineral o livestock) que llega a stock 0 no queda muerto para siempre — vuelve a aparecer con su cantidad inicial completa pasado un cooldown, parametrizable por categoría (`REGENERACION_NODOS`). Livestock (fauna, se recupera por reproducción/migración) regenera en 3 minutos de mundo y los yacimientos minerales (piedra/cobre/estaño/hierro/oro) en 10 — un filón agotado tarda más en "rellenarse" que una manada.
- COMMODITIES DE NOBLEZA (uvas/olivas → vino/aceite): no son alimento básico, requeridas para felicidad de la Nobleza; su déficit arriesga rebelión/estancamiento, no hambruna.

## 1.6 Red de caminos
Los caminos no se construyen: **los hace el uso**. Diseño y decisiones en `typescript-final:Consideraciones/Rutas_Caravana_Avanzadas_Definicion.md`. Cifras placeholder.

**El terreno por el que se viaja** (caravanas, ejércitos y fundación, con el mismo pathfinding):
- **El agua no se cruza** (1.0b).
- **Los ríos tampoco, salvo por un vado.** El tramo alto, cerca del nacimiento, es arroyo y se cruza por cualquier sitio; aguas abajo hay un vado cada ~100 u en el mapa de 2.667 (un vado cada 300 son 10 minutos de rodeo). Una ciudad a orillas del río es también paso: el río se cruza por la ciudad. Los vados salen de la geometría del río, no del azar.
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
- **Sin inmunidad**: ninguna zona protege del todo a una caravana frente a una columna; la zona solo decide el tipo de combate (A o B, Doc 3.10). Los bandidos sí siguen sin actuar dentro de ninguna zona.

**Logro de `logistica_campana`** (Doc 6, Era IV; BA-006 D30): existe un tramo **fuera de toda zona de influencia** recorrido por al menos **10 rutas de 3 Facciones distintas**.

## 1.7 Cap de fundación de asentamientos por Facción (inspirado en Rise of Nations)
- Límite DURO de asentamientos que una Facción puede FUNDAR (no aplica a conquista/anexión, que no tiene límite). Fundar por encima del cap se rechaza.
- ESCALA con el NIVEL DE FACCIÓN: progresión fácil de 1 a 3, luego se complica progresivamente hasta un MÁXIMO DE 7 en etapa tardía. Curva (`CAP_FUNDACION_POR_NIVEL`, placeholder): `[1, 2, 3, 3, 4, 5, 5, 6, 6, 7]` para niveles de Facción 1 a 10.
- Complementa (no sustituye) al sistema de Mantenimiento/Coste de Gobernanza (ver Doc 4).
- Da incentivo mecánico a preferir vasallaje/conquista sobre fundación directa una vez alcanzado el cap.
- **Qué hace subir el nivel de Facción**: la experiencia acumulada (combate digno, conquista, edificio completado, subir de nivel un asentamiento, fundar, cerrar trueques y, mientras la Facción es joven, destruir campamentos de bandidos). Ver Doc 2.2.1 para el criterio completo, la curva de umbrales y el cupo de asentamientos nivel 2/3 que también depende de este nivel.

## 1.8 Caravana de Fundación (la única forma de fundar)

Mecanismo COMPLEMENTARIO al Cap de Fundación (1.7) — ambos coexisten, no se sustituyen. El objetivo es que fundar un asentamiento cueste recursos reales y se sienta orgánico, evitando fundación en cadena sin fricción.

**Es una sola entidad** (2026-10-05): la que compra una Facción sin plazas en un campamento de mercenarios y la que lanza una plaza son la misma y funcionan igual. **Nace sin destino, parada en su origen**, con un **titular** —quien la compra o la lanza— que la engancha a su columna **dentro de su origen**, la plaza o el campamento (Doc 5.13.2), y **funda con `fundar` DONDE SE ESTÁ**. Ya no existe una caravana que viaje sola hacia un punto elegido: caminar hasta un buen emplazamiento, con la columna y su escolta, es la primera decisión de quien funda.

**COSTE** = suma de tres componentes:
1. Los materiales iniciales que recibe todo asentamiento nuevo al fundar (ver 1.3).
2. El coste de construcción de los edificios que nacen automáticamente con la fundación (Centro Urbano, Granja, 3 Viviendas — ver Doc 4.2.1).
3. +50 de madera extra, representando el coste de fabricar la caravana en sí (placeholder).

**De dónde sale:**
- **Desde una plaza**: la lanza (`lanzarCaravanaFundacion`) un residente de ella presente, que pasa a ser su titular. Gates de origen, todos a la vez: la plaza **paga el coste completo** de su almacén; está en **nivel 2** como mínimo (gate estructural, independiente del coste); y el **cooldown de creación** —tras lanzar una Caravana de Fundación o crear una comercial (Doc 3.12), mismo cooldown COMPARTIDO— no ha de estar corriendo: `CARAVANA_COOLDOWN` (10 minutos, parametrizable), para que no se spamee la creación cuando la recién lanzada es destruida (bandidos, 1.9; intercepción, Doc 3.10). El **cupo del Cap de Fundación** se reserva al lanzar: cuentan las plazas de la Facción más sus caravanas de fundación vivas.
- **Desde un campamento de mercenarios** (2026-10-02; rehecho el 2026-10-04, D9-D16, D30, D34, D39, D40, D68; 1.9b): una Facción sin asentamientos, que no tiene origen del que partir, la compra en un campamento al 40 % del coste, con el fondo que aportan sus héroes **en ese campamento** (lo traen en su almacén personal si residen allí, o en el carro). Los gates de la plaza no aplican; sí el Cap de Fundación.

**Fundar** (`fundar`, D30): el titular, con la caravana enganchada a su columna, en un punto **fuera de toda zona de influencia**, no sobre agua y a **20** o más de **cualquier campamento de mercenarios** (`MERCENARIOS.radioExclusionFundar`, D16). Nunca encima de un campamento. Los ciudadanos de su Facción que van en esa columna son **cofundadores** (hasta `FUNDACION.maxJugadoresFundacionGrupal`): unirse a la columna es el consentimiento (D12). La caravana se gasta y la columna entra en la plaza nueva.

**Escoltarla**: la caravana viaja con la columna de su titular y se defiende con ella (Doc 5.13.3): los héroes de la Facción que se le unen (`unirseEnCampo`) la escoltan como a cualquier caravana. Interceptable igual que las demás (Doc 3.10), mismas reglas de combate (tipo A o B según la zona).

**Titular y Facción**: lleva su Facción; solo un ciudadano de ella la engancha, los demás solo pueden atacarla. **Si el titular deja de llevarla** (sale del mundo, deja la Facción, o la lleva otro), se suelta y **vuelve sola** a su origen; otro ciudadano puede **reclamarla** por el camino (Doc 5.13.2: junto a ella, botón Reclamar) y pasa a ser el titular. Si llega sin nadie, **se desarma**.

**Soltarla, caducar y desarmar** — la caravana siempre devuelve lo que costó:
- **Suelta y sin nadie, caduca a las 48 h** (`FUNDACION.caducidadCaravanaHoras`); la que una columna deja aparcada al guarnecer cuenta como suelta. Destruida, se pierde. No hay espera para lanzar o comprar otra (salvo el cooldown de la plaza).
- **Desarmar a mano** (`desarmarCaravanaFundacion`): solo su titular, con la caravana **suelta y en la puerta de su origen** (si no, sería llevarse el coste de un sitio a otro sin recorrer el camino). Cancelar no pierde nada.
- **Qué devuelve**: la de una plaza, **todo el coste a su almacén** (hasta donde quepa). La de un campamento, **a cada aportante lo suyo** (la caravana apunta quién aportó qué): **al fondo de refundación de ese campamento**, a nombre de cada aportante (nada llega a nadie por arte de magia: cada uno lo retira allí en persona); lo de quien ya no es ciudadano se pierde.

## 1.9 Campamentos de bandidos (inspirado en Travian)

Una sola entidad con **tres niveles**, que sale **al azar** al aparecer (2026-10-04, D21, D37). Aparecen por dos vías:

- **Uno por asentamiento, SIEMPRE**: cada asentamiento vivo tiene el suyo, en SU bosque no reclamado MÁS CERCANO (un bosque que NO se solapa con ninguna zona de influencia; se comprueba el CENTRO del bosque contra los polígonos de zona), y queda ASIGNADO a él. Así nunca aparece en la otra punta del mapa sin ningún asentamiento cerca, ni pegado a una zona de influencia. **Cada asentamiento lleva su propio plazo** de reaparición: tras destruirse el suyo, le reaparece uno cuando vence, sea quien sea quien lo destruyó.
- **En el anillo de cada campamento de mercenarios, según la demanda** (D42, D28): a la misma distancia en todos (entre 30 y 50), en tierra firme, fuera de toda zona y de la protección de cualquier campamento. Hay **uno por cada 2 residentes de Facciones sin asentamiento** (o sin Facción), entre 1 y 6; mientras falten, aparece uno cada 10 minutos.

- Mientras el campamento sigue en pie, ATACA CARAVANAS (Doc 3.10) cuando el plan de la caravana cruza su radio de ataque (encuentro exacto) — se resuelve con números, contra la defensa de la caravana. Si gana, la caravana se pierde por completo (nadie la recibe: el bandido no tiene almacén propio). Junto a un campamento de mercenarios no ataca (1.9b).
- **Se ataca con una columna que llegue a él** (Doc 5.12.3), y la batalla se juega en Unity: los héroes de la columna contra las tropas del campamento, manejadas por la IA del juego (Doc 5.15.6). SIN gate de cargo: a diferencia de un asedio, un campamento bandido es una amenaza de mundo abierto, no una acción de guerra entre Facciones. Si pierde, sus héroes quedan heridos (5.16.4) y la columna pierde la mitad del carro, que no se lleva nadie. Si el servidor no tiene conectado ningún servidor de batalla de Conquest, el choque se resuelve con números: los soldados de los héroes sanos contra el poder del campamento.
- **El botín es solo oro** (D22): al destruirlo, **cada héroe de la columna** recibe el oro de su nivel (30, 33 y 36 de oro en los niveles 1, 2 y 3). El oro **sube al carro de la columna** (2026-10-11; lo que no cabe se pierde) y llega a casa como cualquier carga: al entrar en la residencia pasa al almacén personal, que **solo se gasta en el mercado, la taberna o el fondo de refundación de su campamento**: nunca llega a la economía de una plaza. Ya no existe una «bolsa de oro de botín» aparte. Lo que se retira del fondo vuelve como cualquier recurso.
- **Rendimientos decrecientes por héroe** (D26): en las últimas 24 h, botín completo en las 8 primeras destrucciones, luego un 15 % menos por cada una, y desde la 14.ª solo experiencia.
- En el mapa se marca como un diamante rojo.

Cifras, todas PLACEHOLDER (`CAMPAMENTOS_BANDIDOS`):

| Nivel | Sale | Poder | Tropa en Unity | Oro por héroe |
|---|---|---|---|---|
| 1 | 50 % | 20 | 10 milicias de lanceros | 30 |
| 2 | 30 % | 60 | 30 | 33 |
| 3 | 20 % | 120 | 60 | 36 |

- Radio de ataque a caravanas: 7 unidades del mapa.
- Cada plaza tiene su campamento de bandidos en el bosque más cercano **a 10 o más del borde de su zona** (más separados que antes).
- Reaparición del de un asentamiento: 60 minutos tras destruirse.
- **Atacan caravanas**: los campamentos no se mueven; solo atacan las caravanas que pasan cerca de donde están apostados, con resolución numérica. Es un evento PvE que no figura entre los tipos de batalla instanciada (5.15.1b). **Siguen sin actuar dentro de ninguna zona de influencia**: quitar la inmunidad de las caravanas (3.10) es una regla entre Facciones, no contra el PvE.

El campamento no bloquea la explotación del bosque que ocupa: solo amenaza a las caravanas de paso.

## 1.9b Campamentos de mercenarios (decidido el 2026-10-02; los 5 pasos implementados)

Entidad **neutral** del mundo abierto, con miniatura en el mapa. No es de ninguna Facción, no se puede atacar, no crece y no desaparece. **No es un asentamiento**: no tiene zona de influencia, y ninguna zona lo absorbe (las zonas salen de los asentamientos). Es un enclave.

- **Colocación.** Todos existen **desde que se crea el mundo** y no aparecen más (`colocarCampamentosIniciales`): puntos al azar en tierra firme, pegados a un bosque si lo hay a menos de `MERCENARIOS.margenBosque`, a una **distancia mínima** entre ellos (`MERCENARIOS.separacion`: 600, de 8 a 10 campamentos en el mundo regional; cada uno es el centro de un foco de actividad, 1.9c) y a `margenBorde` del borde. El azar sale de una semilla derivada de la del mapa: misma seed, mismos campamentos.
- **Forma.** Taberna, **diez viviendas** y mercado, más **un edificio militar** (barracón, galería de tiro o caballeriza) que se elige al nacer. Hay **3 variantes de aspecto** (el *origen*, para los modelos), sin efecto sobre lo que hace. Aspecto y edificio militar salen de la posición, no del azar de la partida. **Estructura interna** (`layoutCampamento`, 2026-10-05): **reutiliza el trazado de asentamientos** (Doc 1.5) con la **taberna como ancla principal** —el papel del Centro Urbano—: las viviendas se pegan a ella, el mercado nace con sus puestos de nivel 1, el edificio militar con su plaza de armas, y las calles salen de las reglas de siempre. Pero se coloca **una sola vez**: el campamento no crece, no tiene mantenimiento, no se degrada ni sube de nivel. Lo rodea una **empalizada decorativa** —sin efecto de juego: dentro no hay combate— con una puerta de dos celdas al sur. Cada campamento tiene su propia planta, que sale de su id; es función pura del campamento, así que no se guarda nada. Es lo que se ve al estar dentro (D73).
- **Niebla.** Se conoce **como un camino**: si la Facción ha explorado alguna vez el terreno donde está, lo recuerda; no hace falta verlo ahora, porque un campamento no se mueve ni desaparece. El campamento donde reside o donde está dentro lo conoce siempre.
- **Dentro.** El campamento tiene un «dentro», como una plaza: el héroe está en su vista de asentamiento (edificios y layout). Dentro no se come ni hay combate. **Se ve como desde dentro de una plaza**: el mapa de campaña con la visión compartida de su Facción y aliados (quien no tiene Facción ni columna fuera no ve nada vivo). **Cualquier héroe puede entrar en cualquier campamento**, con su columna en la puerta (`entrarEnCampamento`, a `MOVIMIENTO.radioPuerta`):
  - en **el suyo**, la columna se deshace: la tropa queda en el campamento y el carro se vacía en el almacén personal; lo que no cabe se queda en el carro, aparcado en la puerta;
  - en **otro**, entra con su columna, que queda en la puerta intacta.
  Al salir (`salirDelCampamento`), en el suyo elige la tropa y lo que carga desde el almacén personal; en otro, retoma la columna con la que entró. Como al movilizar desde una plaza, puede salir **admitiendo compañía** (`politicaDeUnion`): entonces sale como ejército, **quieto en la puerta y dirigido por su Líder** con sus clics (Doc 5.12.1), y otros ciudadanos de su Facción se le unen en campo —para cazar en grupo o cofundar—. Se entra solo: un ejército con más gente se separa antes. Residir no exige estar allí.
- **Las acciones del campamento** (reclutar, comprar, el fondo de refundación, la intel de su taberna) se hacen dentro o con la columna en su puerta.
- **La taberna vende intel** (2026-10-05, Doc 5.12.10): cualquier héroe **de una Facción** compra ahí **Miradas** e **Informes de plaza**, pagando con el oro de su almacén personal (si reside allí) o del carro de su columna (de visita). El cupo es fijo (1 Mirada abierta por Facción y campamento). Es la vía de información de las Facciones sin plaza; antes la taberna solo era el ancla del trazado.
- **Tropa prestada.** El campamento presta a su residente, **gratis**, tropa de leva comunal: **una escuadra completa de cada tropa que elija** —milicia de lanceros (25), leñadores (30), granjeros (30); una, dos o las tres— (`pedirPrestamo`; el tamaño es el de la escuadra de su tropa, `unidadesPorDefecto`). Es para aprender a usar tropa antes de tener la propia. **No es tropa del héroe**: no gana experiencia ni cuenta para «una escuadra por tropa» (Doc 5.8), y el campamento la retira si deja de residir en él. No come mientras vive en el campamento. Si pierde gente se repone **gratis**, hasta completa, en el campamento o con la columna a su puerta (`reponerPrestamo`).
- **Repostar gratis, para todos** (2026-10-11, `repostarEnCampamento`): cualquier héroe, residente o no, estando en un campamento (dentro o con la columna a la puerta) **llena sus víveres hasta el tope sin pagar y sin gastar stock**. El trigo va **solo a los víveres, nunca al carro**: no se compra trigo para el carro.
- **Ración gratis del residente.** Al salir de su campamento, al residente se le **llenan los víveres** (Doc 5.13), una vez cada 30 minutos de mundo (`MERCENARIOS.racion`); no se acumula. Los víveres no son carga: no se guardan en el almacén personal ni se venden, sirven para moverse.
- **Alijos de exploración** (D29, D60-D63). Escondites de oro colocados al crear el mundo alrededor de cada campamento (`colocarAlijos`, `ALIJOS`): 3 a 20-40 con 3 de oro, 2 a 40-90 con 5 y 1 a 250-300 —en la tierra lejana, junto a las vetas— con 11; siempre con ese campamento como el más cercano, en tierra firme y fuera de su protección. **Son por héroe**: cada uno abre cada alijo una vez y no reaparecen, así que nadie le quita uno a otro. Solo los ven y los abren los héroes de **Facciones sin asentamiento** (o sin Facción): un alijo **se ve** cuando está a la vista de su columna (`VISION.jugadorSolo`, 15) y no lo ha abierto; **se abre** estando en el sitio (`abrirAlijo`). Su oro sube al **carro** de la columna (1.9).
- **Protección.** A menos de `MERCENARIOS.radioProteccion` (10) de un campamento nadie inicia un combate: ni jugadores (`atacar` se rechaza desde allí y contra lo que está allí) ni bandidos.
- **Residencia** (paso 2): cualquier héroe puede residir en él, y es la casa de quien se queda sin asentamiento; se acaba el huérfano (Doc 2.5, 5.15.5). Almacén personal de cada héroe: Doc 2.5.
- **Reclutamiento** (paso 3): reclutar y reponer las tropas de sus edificios, con tecnología propia, pagando oro y gastando la población del campamento (Doc 5.8, 6.5b). Población: viviendas × 10 (100 con las diez), recupera 10 por hora.
- **Mercado** (paso 4): solo vende, con stock limitado, y el oro cobrado se destruye (Doc 3.3b). **Madera, piedra y trigo tienen pila propia** (`MERCENARIOS.mercado.pilas`: 300, 60 y 300), que vuelve a su tope en cada reposición sin depender del comercio del mundo; los demás bienes se reponen con lo que se comercia. **Cupo por héroe y día** en madera (60) y piedra (10). Quien reside compra a su almacén personal; **cualquier otro héroe compra desde la puerta cualquier bien en venta salvo el trigo (que se reposta gratis a los víveres), pagando con el oro de su carro y llevándoselo al carro** de su columna, sin cupo diario.
- **Refundar** (paso 5; rehecho 2026-10-04): una Facción **sin asentamientos** compra en el campamento una **Caravana de Fundación al 40 %** del coste normal; cómo se lleva, se funda, vuelve y caduca, en 1.8. Cada ciudadano aporta lo que quiere al **fondo de su Facción en ese campamento** —desde su almacén personal si reside allí, o desde el carro de su columna en la puerta (`aportarARefundacion`)— y lo suyo se retira mientras no se gaste (el oro, como oro de botín); cuando el fondo cubre el coste, uno de ellos la compra (`comprarCaravanaDeRefundacion`) y es su titular.

## 1.9c Focos de actividad, anillos de tierra y bosques (decidido el 2026-10-12)

**Los focos los crean los campamentos de mercenarios.** Toda Facción sin plaza vive en un campamento y funda andando desde él; con la exclusión a 20, las plazas se apiñan a su alrededor y las de nivel 2 o más lanzan desde ahí sus Caravanas de Fundación: el foco crece hacia fuera y puede llegar a tocar otro. Los campamentos están a **600** unos de otros (1.9b), de 8 a 10 en el mundo regional (en la emulación sobre su máscara de tierra, 10 a 12).

**Tres anillos de tierra**, por distancia al campamento más cercano. El generador reparte los recursos por anillos y para eso coloca primero los campamentos:

| Anillo | Distancia | Qué hay |
|---|---|---|
| **Foco** | hasta 150 | Tierra fértil: bosques medianos y pequeños densos, piedra, fauna, algo de cobre y hierro. **Estaño y oro, muy poco**: una muestra en el borde del foco, de un tercio de la cantidad normal, para que se sepa que existen. |
| **Frontera** | 150-250 | Bosques grandes compartidos, cobre, hierro, piedra, y **más estaño y oro** (aislados). Es donde se expande la segunda generación de plazas. |
| **Tierra lejana** | más de 250 | **Distritos mineros**: la mayoría del estaño y del oro, en vetas en colina o montaña, guardadas por bandidos. Alijos lejanos y fauna de estepa. |

Un foco de 150 son unos 5 minutos a pie. La capacidad del foco es geométrica, no una cuota: a separación 30 caben del orden de 45 plazas una vez descontados pendiente, bosque, agua y recortes de frontera. La tierra firme es el 66 % del rectángulo y la mitad o más queda a más de 250 de todo campamento.

**Estaño y oro, por gradiente** (decidido el 2026-10-13, cierra O1): cuanto más lejos del campamento, más hay y más rico es, para que el jugador sepa que existen cerca y que hay más a medida que se aleja. El reparto es el mismo para los dos metales (`repartoMetalesPorAnillo`): **foco 5 %, frontera 25 %, tierra lejana 70 %**. Con el total de hoy (unos 64 de estaño y 24 de oro) son 3 y 1 nodos en los focos, 16 y 6 en la frontera y 45 y 17 lejos. Los del foco son una **muestra**: van en el borde del foco (de 100 a 150 del campamento) y llevan un tercio de la cantidad normal. Las cuotas por anillo suman exactamente el total; sin supernodos. Lo de la frontera va aislado, con el espaciado de los raros (20 tras la calibración). Lo de la tierra lejana va agrupado en **vetas** de 3 a 6 nodos de estaño (y de 0 a 2 de oro) en un radio de 20-30: una plaza de nivel 2-3 plantada en el centro de una veta cubre 2-3 nodos, lo que justifica una **colonia minera**; varios nodos del mismo tipo dentro de una zona son válidos. Cada veta lleva un campamento de bandidos de nivel 2-3 sin plaza dueña y un alijo lejano. El bronce depende así de expandirse, comerciar o pelear: las caravanas de estaño cruzan tierra de nadie. (Sustituye a la exclusión dura `exclusionEstanoCampamento` = 200.)

**Bosques: pocos grandes y muchos medianos o pequeños.** Los grandes (radio 40-80, del orden de 10-20) son comarcas compartidas entre focos o en su borde, y dan disputa por la madera; los medianos y pequeños (radio 4-20) están dentro de los focos para que cada plaza tenga madera cerca. En el mundo regional el bosque es un campo continuo de densidad: las masas explotables llevan identificador y área.

**Parcelas de explotación** (decidido el 2026-10-13, cierra O2). La Leñera se construye dentro de la ciudad; lo que limita cuántas hay no es un contador por bosque sino el **derecho a explotar una parte** de él. El generador deja en cada masa una rejilla fija de **parcelas** (puntos separados al menos `2·rp`, con `rp` = 4, cada una con su densidad media). Al construir una Leñera se le asigna una parcela libre **cuyo centro caiga dentro de la zona del asentamiento**, y de ella sale su rendimiento. Así el primero en llegar solo puede reclamar lo que su zona cubre y deja el resto de la masa a quien llegue después; los bosques pequeños tienen pocas parcelas. Cada asentamiento puede ocupar como máximo el `X %` de las parcelas de una masa (`cuotaParcelasPorAsentamiento` = 50 %, mínimo 1). Los bosques no se agotan y no hay freno de rendimiento adicional. Las parcelas ocupadas se pueden marcar en el mapa para ver quién explota qué.

## 1.10 El héroe está SITUADO en el mundo

El héroe de cada jugador es un partícipe del mundo, no un ente volador superior. En todo momento está en **uno** de cuatro sitios, nunca en dos y nunca en ninguno:

| Dónde | Qué significa |
|---|---|
| **Dentro de un asentamiento** | Ve su interior completo y puede dar órdenes ahí |
| **En una columna, en el mapa** | Se mueve, ve lo que su columna alcanza, interactúa con lo que se cruza (Doc 5.12) |
| **Dentro de un campamento de mercenarios** | Donde nace y donde reside si no tiene asentamiento; sin columna (1.9b) |
| **Desconectado** | Fuera del mundo, con su sitio guardado |

### 1.10.1 Solo ves donde estás, y solo actúas donde estás

**Nunca se ve el interior de un asentamiento en el que no se está físicamente.** Ni siquiera uno propio, ni siquiera siendo su Gobernador. Almacén, guarnición, colas de construcción y trazado urbano son cosas que se miran desde dentro; desde fuera, de una plaza propia solo viajan sus cargos y políticas activas y, con una columna a la puerta, su almacén.

Y la regla es también de ACCIÓN: **construir, reclutar, comerciar, activar políticas y designar cargos exige estar dentro** de la plaza en cuestión (Doc 2.5), salvo lo que se hace en la puerta (mostrador de comercio, reponer con la columna a la puerta) y lo que el Rey hace sin presencia (designar Gobernador, fijar la puerta). Con acción remota el jugador seguiría siendo un ojo volador, solo que con una venda.

**Lo ya ordenado sigue corriendo solo.** Salir no congela tu ciudad: la auto-construcción avanza, las colas terminan, las caravanas en ruta llegan, la producción produce. Lo que no puedes es darle órdenes NUEVAS mientras no estés.

**Lo que se deja atrás se RECUERDA.** Al salir de una plaza queda la última foto de su interior, fechada — el mismo mecanismo con el que se recuerda una ciudad ajena que se dejó de ver (Doc 5.12.8). El jugador juzga si fiarse de un dato de hace tres horas.

> **Esto NO toca la niebla de guerra.** Tus plazas siguen vigilando su radio para toda tu Facción aunque no estés dentro de ninguna, y lo avistado y lo recordado del MAPA siguen igual (Doc 5.12.7-5.12.8). Lo que se restringe son los INTERIORES, no saber dónde están las cosas ni qué pasa en el mundo.

### 1.10.2 Salir al mundo

Se sale desde la **residencia**, que es donde el héroe tiene su campamento con todos los escuadrones que no lleva encima (Doc 5.15.2). Al salir elige, en una sola pantalla:

1. **Con qué tropas sale**, bajo su Liderazgo (Doc 5.11) — puede ser una sola, todas las que el Liderazgo permita, o **ninguna**. Puede partir de un loadout guardado (Doc 5.16.5).
2. **Con qué materiales sale**, hasta llenar su carro (Doc 5.13).

Sus **víveres** se llenan solos del almacén al salir, sin bajar de la reserva de comida de la plaza (Doc 5.13).

Y aparece en el mapa de mundo **junto al asentamiento**, sin destino todavía.

### 1.10.3 Entrar en un asentamiento

Se entra estando en la **puerta** —a corta distancia de la plaza— y se ofrece como una acción, no ocurre solo (Doc 5.12.3). Lo que pasa con la columna depende de dónde entres:

| Entras en… | Tu columna |
|---|---|
| **Tu residencia** | Se disuelve: los escuadrones vuelven a tu campamento y el carro al almacén |
| **Cualquier otra plaza** | Se queda **aparcada a la puerta**. Al salir la retomas con lo que llevabas, sin ninguna pantalla de equipamiento |

De ahí salen dos consecuencias sin necesidad de más reglas: si conquistan la plaza ajena mientras estás dentro, tu columna está fuera y la retomas; y si te destruyen la columna aparcada, sales a pie.

### 1.10.4 Dentro de una plaza ajena solo se ve la capa pública

La ficha pública (edificios `activo`, zona) y el mostrador de órdenes activas. **Nunca** el interior ni el trazado: ni almacén exacto, guarnición, colas de construcción ni cargos.

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
