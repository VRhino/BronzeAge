# 3. Sistema Económico y de Comercio

## 3.1 Oro
- NO es moneda acuñada: es METAL PRECIOSO EN BRUTO PESADO (sistema tipo siclo/shekel).
- ORIGEN: mina de oro en el mapa, mismo patrón que cantera/mina de cobre (nodo finito, con reemplazo automático al agotarse — ver Doc 4.2); comisiones de comercio (3.5); y **recaudación en el asentamiento según población y clase** (Doc 4.1).
- USO: medio de pago en Mercado (3.3) y en comisiones (3.5); **coste de reclutamiento** salvo la milicia inicial (Doc 5.8); **compra de animales de arrastre para caravanas**, buey incluido (3.13.2); Maravilla (Doc 4.2.1). Pensado también para tecnología, mercenarios y sueldos (Aedas, Doc 6), sistemas que Fase 0 no tiene.
- **El oro obliga a elegir** (`Consideraciones/Economia_Del_Oro_Definicion.md`): es la única moneda que convierte entre ambiciones (ejército / flota comercial / información), y no se puede ir a por todas a la vez. La recaudación por población es la FUENTE que lo hace pagable sin mina; el reclutamiento en oro y los animales en oro son los SUMIDEROS; y la política "Presión Fiscal" (Tesorero, Doc 4.4) cambia más oro por menos crecimiento.
- ENTRE JUGADORES: recurso más de trueque, sin restricción especial.

## 3.2 Trueque de materiales (acuerdos entre Facciones)
- Contrato MARCO abierto en el tiempo: `proponerTrueque` crea el acuerdo (cantidad total pactada por lado, plazo por defecto 200 minutos).
- **PROPUESTA → ACEPTACIÓN**: el acuerdo nace `'propuesto'` y **no obliga a nadie** hasta que el lado receptor (B) contesta con `aceptarTrueque` o `rechazarTrueque`. Ninguna caravana lo mira mientras siga propuesto.
  - **El plazo se cuenta desde el sí**: una propuesta contestada al filo no puede nacer ya sin tiempo material de cumplirse, porque entonces el que acepta de buena fe se comería la penalización por incumplir (Doc 2.7).
  - Una propuesta **sin contestar caduca sin penalizar a nadie** (no hubo promesa que romper); una **rechazada** queda como `'rechazado'` y no se borra: una respuesta es información, y es distinta de un silencio.
  - El **Camino Comercial** del par (Doc 1.6) se traza **al aceptar**, no al proponer: es infraestructura física permanente, y una propuesta unilateral no basta para plantársela a un vecino que no ha dicho ni sí ni no.
  - **Quién contesta cuando no hay jugador**: las plazas NPC contestan en el mismo minuto, con el mismo colchón de excedente que el NPC exige cuando es él quien pide.
  - El NPC contesta también a las propuestas de un jugador. Lo que no hace es proponerle un trueque de SUPERVIVENCIA a un humano: un salvavidas no puede quedarse esperando a que alguien se conecte.
- **Cada envío sale en una caravana propia**: exige tener una caravana PROPIA **'disponible'** en el asentamiento origen (ver 3.12). `asignarCaravanasATrueque` la asigna automáticamente al envío pendiente; si no hay ninguna disponible, el envío espera al siguiente minuto. Para entregar a mano en un trueque, se engancha la caravana a un ejército y se entrega al llegar (Doc 5.13.3); el lanzamiento manual de 3.13.3 no se vincula a trueques.
- Cuando hay menos caravanas disponibles que envíos pendientes en un mismo asentamiento (varios acuerdos compitiendo a la vez), se prioriza por un SCORE ponderado (ver 3.12: urgencia por expiración del acuerdo, urgencia por volumen pendiente, cercanía del destino) — no por orden de llegada.
- La caravana viaja de verdad por el mapa, sobre una RUTA calculada por pathfinding (rodea terreno costoso, ver Doc 1.6/3.6), con la velocidad modulada por el coste del terreno que cruza en cada momento — y entrega al llegar. Al entregar, la caravana vuelve a estar 'disponible' en el origen: es un activo persistente y con coste (3.12), no un objeto de un solo uso.
- Cumplir o incumplir un trueque ajusta el score de reputación de Facción (Doc 2.7).

## 3.3 Órdenes de mercado (comercio abierto)
- Un asentamiento coloca órdenes de compra/venta; cualquier jugador puede dejar lo pedido o comprar lo ofrecido.
- Se pagan con ORO.
- Colocar una orden exige tener un Mercado activo (ver 3.12).
- **EL MOSTRADOR**: una orden es una **oferta en pie EN UNA PLAZA**, no una entrada en una bolsa global. Se cumple **allí**, con alguien que ha ido hasta la plaza con su columna (`comerciarEnPlaza`). No hay emparejamiento automático entre asentamientos: nada cambia de manos sin que alguien recorra el mapa.
  - **La plaza VENDE**: el jugador paga oro DE SU CARRO y la mercancía sube AL CARRO. **La plaza COMPRA**: descarga mercancía DEL CARRO y el oro sube AL CARRO. El viaje completo son cuatro actos —cargar en tu plaza, llegar, comerciar, volver y depositar al entrar en tu residencia (Doc 1.10.3)—.
  - **El oro pesa y ocupa carro** (3.1: es metal precioso pesado, no moneda acuñada). Eso pone un techo físico a cuánto se mueve de una tacada: comprar barato lejos y vender caro en casa cuesta viajes, no un clic.
  - **Manda el Líder de la columna**: el carro es común (Doc 5.13.2), y sin esa condición cualquiera que se uniera en campo podría gastarse el oro de todos.
  - **Sirve lo que puede** en vez de fallar cuando se pide de más —el tope sale a la vez de la orden, del almacén de la plaza, de su oro, del carro y de lo que se lleve encima—, pero **falla si no puede servir nada**.
  - **Las órdenes CADUCAN** (`MERCADO.plazoOrdenMinutos`, 200 minutos, el mismo plazo que un trueque): sin emparejamiento automático nada las cerraría nunca, y una plaza acumularía ofertas eternas a precios viejos.
  - **Visibilidad**: el escaparate de una plaza ajena se ve **al estar en su puerta**, y solo lo que sigue en pie. Un mercado enseña sus ofertas a quien está dentro; una lista global dejaría leer los precios del mundo entero sin moverse.

## 3.4 Precios dinámicos
- Precio de referencia por defecto: precio base escalado por escasez/abundancia GLOBAL (stock objetivo de referencia = 500, con clamp entre ×0.4 y ×3).
- El jugador puede sobreescribir el precio manualmente al colocar una orden.
- NO existe mercado NPC de respaldo.

## 3.5 Comisiones de comercio
- Comisión del 3% dentro de la misma Facción vs. 8% externa, aplicada tanto en trueque como en mercado, modulable por política ("Aranceles/Comercio Abierto") y por reputación de Facción.
- **En el mostrador (3.3), la paga quien toma la orden y se la queda la plaza donde ocurre**: comprando se paga de más, vendiendo se cobra de menos. El oro **se conserva**: la comisión sale de quien comercia, no se acuña. En un trueque, la comisión la cobra el destino al recibir cada entrega (`comisionDeEntrega`).

## 3.6 Categorías de caravana (heredado de Iberia)
1. **Comercial**: la que se usa en Trueque y Mercado, y la única que es un activo PROPIO y persistente (ver 3.12). Es una caravana **compuesta** —lista de carros, cada uno con su animal— y deriva de ahí capacidad y velocidad (3.13).
2. **Militar**: lleva equipo militar antes de un asedio. Capacidad y velocidad propias (`CARAVANA_CATALOGO`).
3. **De construcción**: la Caravana de Fundación (Doc 1.8), un mecanismo aparte del de Trueque/Mercado — no forma parte de la flota propia de 3.12.
4. **De contrabando**: detección reducida; es la más rápida (24).
- **Movimiento con coste de terreno** (ver Doc 1.1/1.5/1.6): la velocidad es la base, y el avance real por minuto se divide entre el coste de terreno en la posición actual (llano≈1, colina/montaña más lento, cima muy caro; el agua no se cruza) — cruzar relieve accidentado tarda más. Cada caravana calcula su propia RUTA por pathfinding al lanzarse (rodea relieve costoso en vez de ir en línea recta), salvo que ya exista un Camino Comercial para ese par de asentamientos (Doc 1.6): entonces reusa su polilínea y recibe además el bonus de velocidad del camino.

## 3.7 Transporte individual espontáneo (heredado de Iberia)
- **El inventario personal es el carro de la columna** (`Ejercito.suministro`, capacidad `LOGISTICA.capacidadCarroPorJugador` por héroe más las caravanas adjuntas, Doc 5.13.2): admite cualquier recurso, incluido el oro. Se carga al salir eligiendo qué llevar (Doc 1.10.2) y se vuelca al almacén al entrar en tu residencia (Doc 1.10.3).
- Es **la única vía por la que la mercancía de una orden de mercado cambia de manos** (3.3): el transporte físico no es opcional.

## 3.8 Bonificación por distancia (heredado de Iberia)
- Aplica como bonus a la comisión de trueque: hasta ×1.5 a partir de 600 unidades de distancia recorrida.
- NO aplica al mostrador (3.3): el bonus está definido sobre la distancia que recorre una CARAVANA de un acuerdo, y en el mostrador el viaje lo hace el jugador.

## 3.9 Dependencia logística real (heredado de Iberia)
No hay un sistema que detecte explícitamente "cortar una ruta" como evento de guerra económica. Si no llegan caravanas, el receptor simplemente no recibe el recurso: es una consecuencia natural del modelo de trueque, no una mecánica dedicada.

## 3.10 Combate de caravanas
- **Interceptar no es un comando**: es lo que puede hacer una columna que se cruza con una caravana enemiga en el mapa, si elige interceptarla (Doc 5.12.3). No hace falta General porque no hace falta orden: hace falta tener una columna ahí, que es más caro y más interesante. El botín viaja en su carro y llega a casa al replegarse.
- La caravana se defiende con **tres capas**: sin nada → la defensa base fija (`defensaBaseCaravana`); con escolta sin héroe (escuadrones cedidos, 3.13.4) → la escolta, manejada por la IA del juego; adjunta a un ejército → el ejército (Doc 5.13.3).
- **UNA CARAVANA SOLA SE DEFIENDE DE UN JUGADOR SOLO.** El héroe combate por sí mismo (Doc 5.1), así que la defensa base deja clara una frontera: **un jugador solo no roba caravanas**. La caravana lleva carreteros y guardias; un hombre a caballo no la para.
  - La cifra **se deriva del poder del héroe**, no se escribe suelta: `defensaBaseCaravana = poderHeroe × 1.7`. Se deriva porque `poderBase` es placeholder (Doc 5.8) y una constante a mano se desincronizaría en cuanto se recalibre el roster — mismo criterio que el coste de Liderazgo (Doc 5.11.1).
  - **Por qué 1.7 y no menos.** Con la varianza de combate (±15% por bando, Doc 5.2.5), para ganar SIEMPRE hace falta superar `1.15 / 0.85 = 1.353`. 1.7 deja margen por encima de ese mínimo sin acercarse al escuadrón más barato: son **25** de defensa, frente a 15 de un héroe y **50** de una milicia completa. Un jugador solo pierde siempre; cualquier escuadrón real gana siempre.
  - Lo que esto cierra: el rol de **salteador unipersonal** —rápido, barato y sin nada que perder—. Depredar caravanas exige tropa.
- Captura exactamente el 50% de la carga si el atacante gana (umbral de captura).
- Una caravana capturada se elimina: no hay reparación (3.13.6).
- **Los bandidos** (Doc 1.9) atacan las caravanas que pasan cerca de su campamento, con la misma resolución numérica.

**ESCOLTA POR EJÉRCITO (Doc 5.13.3).** Una caravana puede ir **adjunta a un ejército** y hacer su entrega normal mientras marcha con él. Deja de defenderse con la defensa base fija y pasa a estar protegida por el poder de combate real del ejército. Se autoequilibra con la regla de velocidad (Doc 5.12.5): el ejército va al ritmo de su integrante más lento, así que escoltar una caravana comercial (16) frena a un ejército ligero (20) y le quita la capacidad de cazar otras caravanas — **no se puede escoltar y depredar a la vez**. Si el ejército es derrotado, la caravana escoltada se pierde con él.

**Pero quedarse sola no es lo mismo que ser capturada.** Si el ejército se deshace sin que nadie lo venciera —porque todos sus integrantes se desconectaron y cada uno se llevó lo suyo (Doc 1.10.6)—, la caravana **vuelve por su cuenta al asentamiento de origen**, haciendo el camino y siendo interceptable durante el regreso. Al derrotado se las quita un enemigo, porque el tren de suministros es objetivo militar; a estas no las venció nadie y los carreteros se vuelven a casa. Si su origen ya no existe, se pierden.

**LAS CARAVANAS NO CRUZAN EL AGUA** (Doc 1.0b): si no hay ruta por tierra entre origen y destino, la caravana **no sale** y la carga se queda en el almacén; el camino comercial tampoco se traza. Aplica igual a las Caravanas de Fundación, que además no pueden fundar en un punto inalcanzable. Comerciar entre dos costas enfrentadas exigiría comercio marítimo, fuera de alcance (3.11).

**LA INTERCEPCIÓN ES EMBOSCADA, NO PERSECUCIÓN**: con las velocidades de tropa de Doc 5.12.5, solo un ejército de infantería **ligera** (20) supera a una caravana comercial tirada por bueyes (16) y puede darle caza. Un ejército medio (16) la iguala y uno pesado (12) no la alcanza jamás; el contrabando (24) y la caravana tirada por caballos (24, 3.13.2) escapan de todo. Interceptar con tropa pesada solo es posible **estando ya apostado en la ruta**, no persiguiendo.

## 3.11 Comercio marítimo — fuera de alcance de Fase 0
Requiere tecnología de barcos + puertos. Fase 0 es 100% terrestre (eje naval pospuesto a fase completa).

## 3.12 Mercado como edificio y flota de caravanas propias

**Mercado (edificio, Doc 4.2.1)**: adición MANUAL de Gobernador/Maestro de Obras (ver Doc 4.2), no auto-construcción. Gatea DOS cosas: colocar órdenes de mercado (3.3) y crear caravanas propias (más abajo). Sus niveles internos administran el cupo de flota (2/4/6) y el de escolta por caravana (1/2/3, 3.13.4) — ver catálogo completo en Doc 4.2.1.

**Flota de caravanas propias**: activo persistente, no efímero. Se crea como un casco vacío y gratis (`crearCaravana`, 3.13.2) que cuenta contra `cupoCaravanas` (nivel de Mercado + política "Ampliación de Flota", +1 aditivo) hasta que se pierda capturada en combate (3.10). **No se puede desmantelar voluntariamente.** Se reutiliza, no se reconstruye en cada viaje: su ciclo de estados está en 3.13.3.

**Cooldown de creación**: tras crear una caravana desde un asentamiento —comercial o de Fundación (Doc 1.8), COMPARTIDO entre las dos— hay que esperar `CARAVANA_COOLDOWN` (10 minutos, parametrizable) antes de poder crear otra desde el mismo asentamiento. Evita que se spamee la creación cuando una caravana recién salida es destruida (bandidos, Doc 1.9; combate de caravanas, 3.10) y el cupo y los recursos vuelven a estar disponibles de inmediato. Es regla del MOTOR, así que gatea igual la creación de un jugador que la de la gobernanza NPC.

**Scoring de asignación** (`asignarCaravanasATrueque`, ver 3.2): cuando hay menos caravanas disponibles que envíos pendientes en un asentamiento, se ordenan por un score 0-100 ponderado:
- Urgencia por expiración del acuerdo (50%): cuánto del plazo ya se consumió.
- Urgencia por volumen pendiente (30%): qué fracción del total pactado sigue sin entregar.
- Cercanía del destino (20%): destinos más cercanos rinden más envíos por caravana disponible.

Pesos y distancia de referencia (600 unidades) placeholder.

**Políticas (Tesorero, Doc 4.4)**:
- "Ampliación de Flota": +1 cupo de caravanas (aditivo).
- "Carga Ampliada": ×1.5 la capacidad de carga de las caravanas propias.
- "Rutas Rápidas": ×1.5 la velocidad de las caravanas propias.

## 3.13 La caravana compuesta

Una caravana `comercial` es un contenedor de **carros, animales y escolta**, y de lo que se le monta deriva su
capacidad y su velocidad. Diseño y representación en el motor en `Consideraciones/Revamp_Caravanas_Definicion.md`.
Todas las cifras (`CARRO_CATALOGO`, `ANIMAL_CATALOGO`, `CARAVANA_PREPARACION`, `CARAVANA_ESCOLTA`) son placeholder.

### 3.13.1 Tres partes

| Parte | Qué aporta | Modelo |
|---|---|---|
| **Carros** | Capacidad de carga; y cuanto más carros, más tarda en prepararse | Lista de carros; cada carro lleva **como mucho un animal** |
| **Animales de carga** | Qué carros pueden moverse y a qué ritmo | Un animal por carro; sin animal, el carro no sale |
| **Escolta** | Defensa propia sin ningún héroe acompañando | Escuadrones cedidos por un héroe, **por viaje** (3.13.4) |

- **Solo viajan los carros con animal.** Un carro sin animal se queda `'disponible'` en el origen — no es lastre en ruta, simplemente no sale en ese envío.
- **Capacidad de viaje** = suma de (`capacidadBase` del carro × `factorCarga` del animal) sobre los carros con animal.
- **Velocidad** = la del animal **más lento** de la caravana.

### 3.13.2 Carros y animales

La caravana comercial nace como un **casco vacío y gratis** (`crearCaravana`) — cuenta contra el cupo del
Mercado y arranca el cooldown de creación, pero no puede viajar hasta que se le montan piezas. Todo el coste
está en las piezas, que se construyen y compran **sobre una caravana concreta** (no hay piezas sueltas).

**Carros:**

| Carro | Dónde se fabrica | Coste | Rol |
|---|---|---|---|
| Básico | Mercado | 20 madera | `capacidadBase` ancla — un carro básico + un buey dan 500 de capacidad a velocidad 16 |
| Reforzado | Carpintería | 40 madera | Más `capacidadBase` (800) |

**Animales** — se compran sobre una caravana y se enganchan a un carro sin tracción:

| Animal | `factorCarga` | Velocidad | Coste | Nota |
|---|---|---|---|---|
| Buey | 1.0 | 16 | **~12 oro** | El ancla |
| Caballo | 0.5 | 24 | 60 oro | A esta velocidad **escapa de casi toda intercepción** (3.10, "emboscada, no persecución") |
| Camello | 0.75 | ~19 | 40 oro | Opción intermedia |

**El buey se paga en ORO, no en madera.** La fundación entrega 100 oro (`FUNDACION.materialesIniciales`) y la
recaudación de oro por población (3.1 / Doc 4.1) lo repone aunque no haya mina, así que un asentamiento nuevo
se paga su primera caravana (20 madera + 12 oro) con lo de fundar. Buey barato para que sea una decisión de
*cuántas* caravanas montar, no un muro. Los animales solo se compran: el livestock del Corral (Doc 1.4) es un
recurso distinto.

**El Mercado NO regala ninguna caravana.** Con los 100 oro de fundación cubriendo el arranque, una caravana
gratis solo amplificaría la fuente de oro: todo asentamiento comerciando desde el primer minuto, más
comisiones, más oro.

### 3.13.3 Preparación y lanzamiento manual

Además del reparto automático (3.13.5), un residente del origen **lanza una caravana a mano**
(`prepararCaravana`): elige **carga** —del almacén del origen, hasta la capacidad de la caravana— y
**destino**. La caravana pasa por un estado `'preparando'` en el origen durante
`prepMinutos = CARAVANA_PREPARACION.kPorCarro × (nº carros − 1)` — una caravana de 1 carro sale al instante,
las grandes tardan. La carga se **reserva del almacén ya** (se descuenta al preparar).

- **`cancelarCaravana`** mientras siga `'preparando'` la devuelve a `'disponible'` y **reingresa la carga
  entera** al almacén — igual que quitar una obra `'en_cola'` (Doc 4.2).
- Al vencer la preparación, pasa a `'en_transito'` sobre la ruta ya calculada (si al preparar no había ruta
  por tierra, el comando se rechaza — el agua es infranqueable, Doc 1.0b).
- Una caravana `'preparando'` **no es interceptable**: está en su ciudad, no en el camino.
- Al llegar a destino, **vuelca la carga en su almacén y paga la comisión de comercio** (3.5), como cualquier
  entrega; luego hace el viaje de vuelta (`'retornando'`). El lanzamiento manual **no se vincula a un trueque**
  — es logística de recursos (mover mercancía a plaza propia o aliada); los trueques los cubre el reparto
  automático o la entrega con ejército (3.2).

Ciclo completo: `disponible → preparando → en_transito → retornando → disponible`. Al volver desanda su ruta:
una caravana nunca se teletransporta.

### 3.13.4 Escolta sin héroe

Distinta de la escolta por ejército (Doc 5.13.3), que exige a un héroe marchando con la caravana.

- Un héroe **residente del asentamiento de origen** cede escuadrones de su campamento a la caravana **al
  lanzarla** (`prepararCaravana` con `escoltaEscuadronIds`). No necesita estar presente ni acompañar el viaje;
  lo que viaja son sus escuadrones (`Caravana.escolta`).
- **Siguen con la caravana hasta que termina el viaje o es destruida** — cesión **por viaje**, no enganche
  permanente. Al terminar vuelven al campamento de su héroe. Si la caravana es destruida o capturada, por el
  motivo que sea, quedan a 0 unidades y vuelven igualmente al campamento de su héroe (Doc 5.15.4).
- Mientras están cedidos no están en el campamento, así que tampoco pueden estar en guarnición, y **cuentan
  contra el Liderazgo del héroe** (Doc 5.11), sumados a lo que lleve consigo y a sus otras escoltas. Ceder
  tropa a una escolta no libera Liderazgo: es coste de oportunidad puro.
- **No consumen ración.** Una escolta no es una campaña; se abstrae el suministro (a diferencia de Doc 5.13).
- **Cupo por nivel interno de Mercado**: 1 / 2 / 3 escuadrones por caravana (`CARAVANA_ESCOLTA`).
- **Combate**: la escolta combate manejada por la IA del juego. Contra un héroe que intercepta la caravana
  (3.10) es una batalla de Unity; contra bandidos (Doc 1.9), sin ningún héroe humano en juego, se resuelve con
  números (Doc 5.15.6). Sus bajas son permanentes y no queda herida: los escuadrones no tienen ese estado (Doc
  5.16).

### 3.13.5 Reparto automático vs preparación manual

- `asignarCaravanasATrueque` (3.2) reparte para trueque y NPC las caravanas **no reservadas**.
- Una caravana con `reservadaManual = true` sale del pool automático **sea cual sea su tamaño**. Una de 1
  carro se puede reservar; una de 5 puede seguir en automático. La señal es explícita, no se infiere de la
  composición.
- El **NPC no compone** caravanas multi-carro ni asigna escolta: es cosa del jugador.
- La caravana es **persistente y se reconfigura**: entre viajes el jugador le añade carros y animales, y
  **mueve carros entre dos caravanas suyas** (`moverCarroCaravana`) — libre, sin coste ni tiempo, mientras
  ambas estén `'disponible'` en el mismo asentamiento. Un carro no se quita suelto: se mueve a otra caravana.

### 3.13.6 Captura

Si el atacante gana, **la caravana se elimina**: carros, animales y carga se pierden, y la parte capturada de
la carga (3.10) va al carro del atacante. El dueño reconstruye contra su cupo con el `CARAVANA_COOLDOWN` de
siempre. Su escolta sin héroe queda a 0 unidades y vuelve al campamento de su héroe (3.13.4).

### 3.13.7 Caravana aparcada

Un ejército con caravanas adjuntas que **entra en una plaza de su Facción** (Doc 5.12.4) no las pierde: quedan
**aparcadas** en esa plaza. Una caravana aparcada:

- **Sigue siendo de su origen.** No la reparte el comercio automático de la plaza anfitriona, no cuenta contra
  su cupo de Mercado, y los bandidos no la tocan (está en una plaza amiga).
- **Intercambia con el almacén de la anfitriona.** Un residente de su origen presente en la plaza puede cargar
  recursos del almacén al carro y descargar del carro al almacén.
- **Solo sale de dos formas:** enganchada a un ejército de la Facción (cualquiera, no solo el que la trajo), o
  **enviada a su origen** — si va vacía aparece allí al instante, si lleva carga recorre el mapa de vuelta y
  la vuelca en el almacén del origen al llegar (una caravana con carga nunca se teletransporta).
