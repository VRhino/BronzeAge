# Campamentos como puerta de entrada y fundación por Caravana — decisiones y plan

> **Estado (2026-10-04): diseño cerrado, sin código. Consejo pasado; seis rondas de decisiones; calibración
> aceptada (§8); alijos cerrados (D60-D63). Revisado contra los pasos 1-3 del NPC fuera del motor (`b40af3d`, §3.2).
> Desconexión redefinida (D64-D71). Sin puntos abiertos (§7).**
> Cambia cómo se entra al mundo, cómo nace una Facción nueva, de dónde saca recursos un héroe sin plaza y qué pasa
> con un jugador que no está conectado. Sustituye la «fundación a pie» (`Consideraciones/Entrada_Al_Mundo_Definicion.md`
> §6.5, `Jugador_Situado_Definicion.md` §5.9), amplía la refundación del paso 5 de mercenarios (Doc 1.9b) y rehace
> los campamentos de bandidos (Doc 1.9).
>
> Reglas de juego → irán a `Docs/Game/1` (§1.3, §1.8, §1.9, §1.9b), `Docs/Game/2` (Facción), `Docs/Game/3` §3.10 y
> §3.13 y `Docs/Game/5` (§5.13, batalla) cuando se cierre. Aquí solo decisiones, hallazgos y plan.

## 1. El enunciado del usuario (2026-10-02)

> Cuando arranca un servidor pasa todo igual, solo que al final de la creación se añade un paso: la aparición de
> campamentos de mercenarios regados por el mapa, más o menos según su tamaño. Cuando un héroe entra a la partida
> elige en cuál de los existentes aparece; se le muestra cuántos héroes eligieron ese campamento como inicial, para
> ver lo congestionado que está ese sector. Para fundar hay que: 1) ser parte de una Facción; 2) que entre sus
> miembros reúnan materiales para comprar la Caravana de Fundación en el campamento; 3) salir del campamento con
> ella, llegar a donde quieren fundar y llamar al comando `fundar` allí. La Caravana de Fundación, como las demás,
> debe poder ser atacada y escoltada por jugadores de su Facción.

## 2. Decisiones cerradas con el usuario

### 2.1 Primera ronda (2026-10-02)

| # | Decisión |
|---|---|
| D1 | **Campamentos iniciales** al final de la generación del mundo, repartidos **por separación mínima** entre ellos (valor a balancear), cantidad proporcional al área. *(Reformulada por D32.)* |
| D2 | ~~La aparición gradual se conserva.~~ **Derogada por D32.** |
| D3 | `crearHeroe` **elige campamento**: el héroe nace como residente de ese campamento. |
| D4 | La pantalla de elección muestra **cuántos héroes eligieron cada campamento como inicial**. **Solo informativo**: no hay tope. |
| D5 | Un héroe sin Facción puede **crearla** (queda como Rey sin plazas; ya es así en `crearFaccion`) **o ingresar** en una existente. |
| D6 | El ingreso es por **invitación o por solicitud**, con **visto bueno del Rey** (precisado por D31 y D46). |
| D7 | **Fundar a pie desaparece** (endurecida por D19: del todo). |
| D8 | La **expansión desde un asentamiento** (Caravana de Fundación de una plaza, Doc 1.8) **se conserva**. |
| D9 | Comprar la Caravana en un campamento: **solo una Facción SIN asentamientos**, al **75 %**, pagada desde el fondo de **esa Facción en ese campamento** (D39). |
| D10 | La caravana de campamento nace **sin destino**, parada en el campamento. **Se conduce enganchada** a la columna (Doc 5.13.2-3) y **se funda con el comando `fundar`** donde esté: **ya no es automática**. |
| D11 | **Solo el comprador** (titular) la engancha y llama `fundar`. |
| D12 | Los demás héroes de la Facción **escoltan uniéndose a la columna del titular**. |
| D13 | Si el titular muere, sale de la Facción o se desconecta, la caravana **intenta volver a su origen, el campamento** (D40), y **otro ciudadano puede reclamarla** enganchándola donde la alcance (pasa a ser titular). *(Corregida por D40: antes «se queda donde esté».)* |
| D14 | Si **nadie la engancha en un plazo** (propuesta: 48 h), **caduca y devuelve el fondo** a quienes aportaron, según el registro de D34; lo que no se pueda recibir se pierde (D43). |
| D15 | Destruida en camino: **se pierde**, sin cooldown para recomprar. |
| D16 | Punto válido para `fundar`: **fuera de zonas de influencia**, **fuera del radio de exclusión de un campamento** (§8.2), **no sobre agua**. |

### 2.2 Segunda ronda (2026-10-03, tras el consejo)

**Alcance y compatibilidad**

| # | Decisión |
|---|---|
| D17 | **Mundo nuevo.** El actual se borra al terminar la mecánica. Sin migración ni compatibilidad con snapshots anteriores. |
| D18 | **Los clientes pueden romperse.** BronzeAgeClient y Conquest se arreglan después para reflejar el motor. El contrato cambia sin transición. |
| D19 | **Fundar a pie se elimina del todo**: comando, código asociado y su uso en bots/batch. Sin flag ni modo transitorio. |
| D20 | **El plan de worldgen paralelo (`Docs/Arquitectura/11_...`) no existe para este trabajo.** Se diseña sobre el generador actual. |

**Fundación y Facción**

| # | Decisión |
|---|---|
| D30 | **Un solo mecanismo de fundación en el motor** (cierra C1): «fundar con una caravana en un punto». El comando `fundar` del titular lo llama; la caravana de una plaza que llega a su destino lo llama automáticamente. D8 y D10 son dos llamadores del mismo mecanismo. |
| D31 | **Solo el Rey aprueba** un ingreso (cierra C3). **`comprarCasa` se elimina** con todo su código asociado: no hay tercera puerta a la ciudadanía. |
| D32 | **Se elimina la aparición gradual de campamentos de mercenarios** (cierra C4). Solo existen los iniciales, colocados por separación mínima. |
| D34 | **La caravana registra quién aportó qué** al fondo gastado, para devolverlo si caduca (D14). |
| D35 | **Semilla derivada** para todo lo que se coloca al generar el mundo (campamentos, alijos): no consume el RNG compartido (P11). |
| D36 | **La caravana de campamento se puede atacar como cualquier otra.** Su **origen es el campamento** del que sale (corrige el defecto de dueño del §3.1). |

**Arranque económico (cierra P1)**

| # | Decisión |
|---|---|
| D21 | **Bandidos alrededor de los campamentos de mercenarios**, en **niveles de dificultad**: los altos exigen grupos mayores y dan más. Rendimiento por héroe y hora parecido en todos los niveles, con una prima pequeña por agruparse. Siempre sin abrir una mecánica explotable. *(Aparición precisada por D42.)* |
| D22 | **Los bandidos dan solo oro.** |
| D23 | **El mercado del campamento vende piedra y madera** además de lo que ya vende, con tope: **pila compartida del campamento + cupo diario por héroe** (D41). |
| D24 | **Ración de trigo gratis para el residente**: pequeña, pero suficiente para mover tropa a las inmediaciones y atacar bandidos. |
| D25 | **Tropa prestada por el campamento**: en el arranque, **solo a residentes** y **solo `leva_comunal`** (milicia de lanceros, leñadores, granjeros). **No come mientras vive en el campamento.** La deuda se cobra del botín. *(Naturaleza del préstamo: D45.)* |
| D25b | Si la tropa prestada muere, **se repone allí mismo con un coste bajo y asequible** (son unidades muy básicas). |
| D26 | **Rendimientos decrecientes por héroe**: el botín baja con cada campamento de bandidos destruido en las últimas 24 h. |
| D27 | **Botín de destino restringido**: el oro del botín solo puede ir al fondo de refundación o al mercado del campamento, nunca a la economía de una plaza. |
| D28 | **Reaparición de bandidos por demanda**: alrededor de un campamento, en proporción a sus residentes de Facciones sin asentamiento. |
| D29 | **Alijos de exploración**: escondites colocados al generar el mundo, ocultos por la niebla, de un solo uso y sin reaparición. **Mecánica aparte**: se define cuando esté cerrado todo lo demás, antes de la primera línea de código (D48). **Cerrado por D60-D63.** |
| D37 | **Bandidos unificados**: los de los asentamientos (Doc 1.9) pasan a ser **la misma entidad con niveles**, que salen **aleatorios**. |
| D38 | **Ritmo objetivo**: los **2 primeros días** son para aprender el combate del mundo. Pagar la caravana debe costar **unas 8 horas de juego a un grupo de 3 héroes**. Con esa cifra se calibra todo lo de P1 (§8). |

### 2.3 Tercera ronda (2026-10-03)

**Presencia: qué es de un jugador cuando no está** — regla **general** del mundo, no solo de la caravana.

| # | Decisión |
|---|---|
| D33 | **Desconectado** = el jugador cierra o pierde el cliente. Los jugadores **no están en el mundo mientras no están conectados**: al desconectarse, **su héroe y su tropa salen del mundo** desde donde estén y, al reconectar, **reaparecen donde quedaron**. *(Cierra C2/C5.)* |
| D33b | **En una batalla de Unity** (por tiempo), la desconexión es distinta: si reconecta antes de que la batalla caduque, **vuelve a entrar en ella**. Si la **abandona por su propia mano** (mecanismo nuevo, «abandonar batalla», **trabajo conjunto con Conquest**), se le aplica **derrota**. Si la desconexión es **involuntaria**, recibe **el resultado de su bando**: ganador si su equipo gana, perdedor si pierde. |
| D40 | **Las caravanas no son del jugador.** Cuando la gente que las conduce desaparece, **intentan volver a su origen** (Doc 3.10 se mantiene, y vale también para la de campamento: su origen es el campamento). |
| D40b | **Las unidades prestadas dejan de ser del héroe**: las cedidas a la **escolta de una caravana** y las de la **guarnición** de un asentamiento **se quedan donde están prestadas** cuando el héroe no está. |

**Economía del campamento**

| # | Decisión |
|---|---|
| D39 | **El fondo es de la Facción en un campamento** (cierra P4/M1). **Los materiales no se mueven mágicamente**: si están en otro campamento, alguien los lleva **en su carro** hasta donde está el fondo, y **al pagar la caravana se aporta desde el carro**. *(Excepción explícita a Doc 5.13, «el carro no se descarga en ruta»: solo para aportar al fondo.)* |
| D41 | **Tope del mercado = pila compartida + cupo por héroe.** Cada campamento tiene un stock de piedra y madera que se repone por hora y por el que compiten sus residentes; además, cada héroe tiene un cupo diario. La pila compartida hace que la congestión importe (un campamento lleno rinde menos por cabeza) y el cupo corta el acaparamiento. |
| D42 | **Bandidos del campamento**: aparecen **según la necesidad** (residentes de Facciones sin asentamiento), con **nivel aleatorio**, a **una distancia fija igual para todos los campamentos**, y **no demasiados a la vez**. Esa distancia es la que fija **cuánto trigo** necesita un héroe para ir a atacarlos. |
| D43 | **Devolución por caducidad**: lo que no se pueda recibir (almacén lleno, aportante muerto o que ya no es ciudadano) **se pierde**. |
| D44 | **Repostar trigo en cualquier campamento**: venden trigo por oro y, al ser neutrales, **cualquier héroe** puede repostar en ellos. La ración gratis del residente (D24) se mantiene aparte. |
| D44b | **La columna puede comer de la carga de la caravana** durante el viaje de fundación, pero **el valor de fundación queda bloqueado**: solo se come lo que exceda lo que la caravana necesita para fundar. *(Confirmada 2026-10-03.)* |
| D45 | **El préstamo no es reclutamiento.** Las unidades prestadas **nunca forman parte de la tropa del héroe** (de su «ejército» propio, no de una columna), **no ganan experiencia de tropa**, y el héroe solo las usa **para probarlas o cuando no tiene tropa propia**. **Al dejar de residir** en el campamento, el préstamo **deja de estar disponible**. Reclutar en el campamento (Doc 1.9b, paso 3) es otra mecánica, y esa sí da la tropa al héroe. |
| D46 | **Ingreso**: el Rey tiene **una lista de solicitantes** y **acepta o deniega**. Aceptado, entra en la Facción; denegado, no. |
| D47 | **Orden de los trabajos de diseño**: bots-héroe (§7) antes que los alijos (D29). |

### 2.4 Cuarta ronda (2026-10-03)

| # | Decisión |
|---|---|
| D48 | **Toda caravana en uso lleva la Facción que la usa** (`faccionId`). **Solo un ciudadano de esa Facción puede reclamarla (engancharla) y redirigirla**; a cualquier otro **solo le sale la opción de atacarla**. Antes bastaba deducirla del asentamiento de origen; ahora una caravana puede salir de un campamento donde conviven héroes de varias Facciones. *(`Caravana.faccionId` ya existe y `faccionDeCaravana` la prefiere; falta que sea obligatoria y que todo el motor la use en vez del origen — §3.1.)* |
| D49 | **No se puede entrar en una Facción NPC.** Se quita del canon y de la documentación, junto con la «vida dentro de una Facción NPC» (el huésped de `Entrada_Al_Mundo` §5, descartado; borrado de `Docs/Mecanicas a desarrollar.md` §16). La lista de solicitantes (D46) solo existe en Facciones de jugadores. |
| D50 | **El sobrante de la ración gratis vuelve al campamento**, no al almacén personal (cierra la explotación de §4.4). |
| D51 | **La calibración del §8 se acepta** como punto de partida (placeholder), a ajustar con batch. |

### 2.5 Quinta ronda: bots-héroe (2026-10-04)

| # | Decisión |
|---|---|
| D52 | **Todo el funcionamiento NPC (incluida la gobernanza) sale del motor y del tick**, a otro hilo o proceso de ejecución, y **actúa desde fuera como un jugador**. Motivo: ya ocupa mucho del tick con el comportamiento actual. Arquitectura: `Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md`. |
| D53 | **No hay Facciones NPC ya asentadas al arrancar.** Todo nace por el flujo de campamento → caravana → `fundar`, bots y humanos igual. Meterlas falsearía el comportamiento real del servidor. Se derogan Doc 1.3 «el servidor arranca con Facciones NPC ya asentadas» y el comando de admin `crearFaccionNpc`. |
| D54 | **Bots-héroe y humanos conviven en los campamentos y los bots cuentan como residentes** (pila del mercado, demanda de bandidos). De momento: así, retirarlos cuando lleguen los jugadores tiene poca fricción. |
| D55 | **Los bots tienen sesiones** (presencia, D33): unas horas al día en uno o dos bloques, por semilla; fuera de ellas salen del mundo como un humano. **Lógica fuera del motor.** |
| D56 | **Llegada escalonada** de bots a lo largo de los días, como llegarían los jugadores. **Fuera del motor.** |
| D57 | **Tres perfiles**: grupo de amigos (5 que llegan juntos, desde 2026-10-07 —antes 3—; uno crea la Facción, los otros solicitan), solitario, tardío. En una Facción de bots, el Rey bot acepta las solicitudes. |
| D58 | **Batch nuevo**: N bots-héroe llegando escalonados en vez de 100 Facciones fundadas en el tick 0. La línea base y los diarios de batch anteriores quedan cerrados. **Se borra la estructura que no se use en el formato nuevo** (inventario en `Docs/Arquitectura/12` §7). |
| D59 | **Una Facción de bots que pierde su última plaza no se disuelve**: sus héroes vuelven a residir en un campamento y repiten el ciclo, como un jugador. |

### 2.6 Sexta ronda: alijos (2026-10-04)

Cierra D29.

| # | Decisión |
|---|---|
| D60 | **Los alijos son por héroe**: cada héroe abre cada alijo **una sola vez** y no reaparecen. Nadie le quita un alijo a otro, así que quien llega tarde encuentra lo mismo que quien llegó primero y los bots no compiten con los humanos. *(Lectura de «un solo uso» de D29: por héroe, no por mundo.)* |
| D61 | **Solo dan oro**, por el mismo canal restringido que el botín de bandidos (D27): solo puede ir al fondo de refundación o al mercado del campamento. |
| D62 | **Cantidad y valor (placeholder, D51)**: 6 por zona de campamento, unos 30 de oro por héroe en total. Por distancia al campamento más cercano: 3 cerca (100-200, 3 de oro), 2 a media distancia (200-300, 5 de oro) y 1 lejos (más de 300, en los huecos entre campamentos, 11 de oro). Recorrerlos todos con el héroe solo cuesta unas 2 h y unos 6 de trigo por cada 40 min, es decir, unos 15 de oro por hora, por debajo de los bandidos (unos 17). Se colocan con semilla derivada (D35), nunca sobre agua ni dentro del radio de protección, y pueden caer dentro del anillo de bandidos. Un alijo **se ve** cuando entra en la vista del héroe (80) y él no lo ha abierto todavía; **se abre** estando en el sitio. |
| D63 | **Solo los abren héroes de Facciones sin asentamiento** (el mismo criterio que la demanda de bandidos, D42). Las cuentas secundarias suman 30 de oro cada una; es el mismo techo que ya tienen con los bandidos y se acepta sin más reglas. |

### 2.7 Séptima ronda: qué significa desconectarse (2026-10-04)

Corrige D33, D40 y D40b tal como se implementaron en `b40af3d` (§3.2). Principio: **desconectarse quita el
control, no el sitio.** El héroe desconectado no da órdenes ni participa en nada; **a un jugador que no está no se le
puede cazar**; y como el servidor no distingue una desconexión a propósito de una caída de red o de luz, no se castiga
la desconexión: se hace **inútil para huir**.

| # | Decisión |
|---|---|
| D64 | **Dentro de una plaza o del campamento de mercenarios, desconectarse no mueve nada.** El héroe sigue en su ubicación con una marca de desconectado: no da órdenes y **no defiende en persona**. Sus escuadras no se mueven (desaparece el contenedor `fuera` para la tropa de la plaza). La defensa no cambia: la guarnición defiende siempre con la IA; las escuadras libres no defienden; un héroe conectado, dentro y sano entra a defender con su loadout, dentro de su Liderazgo y del tope de héroes por bando (ya es así en `engine/tropa.ts`, `defensaDe`). |
| D65 | **Con la columna en el mapa, el héroe y su columna salen del mundo** (B2), como en `b40af3d`, a los **2:30** de desconectarse. No se le puede cazar mientras no esté. |
| D66 | **En peligro no se sale.** Si al cumplirse los 2:30 una columna hostil **le está persiguiendo** (`perseguir` con él como objetivo), la salida se aplaza: la columna sigue en el mundo con la última orden que llevaba y, si la alcanzan, combate con la IA como una escolta sin héroe. Sale en cuanto deja de estar perseguida, y **como mucho a los 5 minutos de la desconexión** (D83), pase lo que pase. Si se reconecta antes, recupera el control al instante. Así quien huye desconectándose no gana nada y a quien se le cae la red le pasa, como mucho, lo mismo que si se hubiera quedado quieto un momento. |
| D67 | **En un ejército compartido**: se separa con lo suyo y sale, y el mando pasa al **conectado** más antiguo (si está en peligro, D66 vale para todo el ejército). |
| D68 | **La caravana de fundación sin nadie que la lleve vuelve a su origen** (el campamento). Si llega sin que la haya reclamado otro ciudadano de su Facción, **se desarma y devuelve los materiales a cada donante** con el registro de D34 (lo que no pueda recibir se pierde, como en D43). **Sin cooldown**: se puede volver a comprar en cuanto los jugadores vuelvan. |
| D69 | **La columna dentro de la ventana de D66 sigue dando visión** a su Facción: físicamente sigue allí. |
| D70 | **Un Rey desconectado no delega**: las solicitudes de ingreso esperan en la lista y lo automático de la plaza (autoconstrucción, recetas) sigue funcionando. |
| D71 | **La milicia prestada (D45) sale del mundo con el héroe**, como el resto de su columna, y vuelve con él (cierra §7.3). |

### 2.8 Octava ronda: estar dentro del campamento (2026-10-04)

| # | Decisión |
|---|---|
| D72 | **Residir no exige estar allí**: es tener ese campamento como casa. |
| D73 | **El campamento tiene un «dentro»**, como una plaza: una ubicación propia del héroe, que en el cliente es la **vista de asentamiento del campamento** (sus edificios y su layout; caminable en Unity como está previsto para los asentamientos). Dentro no se come y no hay combate. |
| D74 | **El héroe nace DENTRO del campamento que elige** (D3), como residente, sin columna en el mapa. |
| D75 | **Las acciones del campamento se hacen desde dentro o desde fuera con la columna junto al campamento**, a la misma distancia de interacción que ya se usa con los asentamientos. |
| D76 | **Salir de su campamento de residencia**: elige la tropa (la suya y la prestada) y lo que carga en el carro desde su almacén personal; la columna aparece en la puerta. **Entrar en él**: la columna se deshace, la tropa queda en el campamento y el carro se vacía en el almacén personal; **lo que pase del tope se queda en el carro**. |
| D77 | **Cualquier héroe puede entrar en cualquier campamento** (enclave neutral), como en un asentamiento que no es el suyo: entra **con su columna**, y al salir elige solo entre las unidades que trajo y lo que lleva en el carro. |
| D78 | **Protección (M4)**: a menos de 60 de un campamento nadie inicia un combate, ni jugadores ni bandidos. |
| D80 | **El préstamo es gratis, al pedirlo y al reponerlo, y sin deuda** (corrige D25b y el préstamo del §8.1). El residente elige **una, dos o las tres** tropas de leva comunal (milicia de lanceros, leñadores, granjeros), una escuadra de cada. Es la forma fácil de tener tropa al principio y aprender a usarla antes de tener la propia. |
| D81 | **La ración gratis se mide en minutos de marcha de la columna con la que se sale, no en trigo fijo** (corrige la cifra de D24 y el §8.1; 2026-10-04). Con el coste de terreno real (`mapa.costeEnPunto`, ≈2× en bosque o colina), ir y volver del anillo (150-250) con 15 lanceros cuesta de 41 a 650 de trigo según semilla y dirección, casi siempre más de 100: con 60 no cazaba nadie. Ahora: `consumo de la columna por minuto × 45` (15 lanceros + héroe ≈ 124), cada 30 min, sin acumular y hasta el hueco del carro. PLACEHOLDER. |
| D82 | **Dentro de un campamento se ve como dentro de un asentamiento** (2026-10-04; sustituye a una primera versión en que no se veía nada de fuera): el mapa de campaña sigue llegando con la visión compartida de su Facción y aliados; estar dentro solo añade el interior. Quien no tiene Facción ni columna fuera no tiene ojos en el mapa, así que dentro no ve nada vivo. |
| D83 | **El tope del aplazamiento de D66 son 5 minutos** desde la desconexión (antes 3, que con ticks de 1 minuto caía en el mismo tick que los 2:30). |
| D84 | **Los alijos se quedan como están**: cada héroe puede abrir todos los suyos, en cualquier zona (cierra la duda de limitarlos a la zona de su campamento). |
| D79 | **Contador doble (M3)**: cada campamento guarda cuántos lo eligieron como inicial, además de sus residentes actuales; la pantalla de elección muestra los dos. |

### 2.9 Novena ronda: una sola Caravana de Fundación (2026-10-05)

| # | Decisión |
|---|---|
| D85 | **La Caravana de Fundación es una sola** (decisión del usuario): la de un campamento y la de una plaza son la misma entidad y funcionan igual, tal como estaba definida la de campamento (D10-D14, D30, D40, D68). **La que iba con destino deja de existir**: la plaza la lanza sin destino, parada en ella, con un titular que la engancha a su columna y funda donde esté (`fundar`). Los héroes que se unen a la columna la escoltan y son cofundadores (D12). Sin titular vuelve sola a su origen, suelta caduca (`FUNDACION.caducidadCaravanaHoras`), y al desarmarse o caducar **devuelve lo que costó**: al almacén de la plaza, o a cada aportante si salió de un campamento. |
| D86 | **Quien lanza desde una plaza es el titular** y tiene que ser residente presente en ella (como todo lo que sale de casa). **Desarmar a mano**: solo el titular, con la caravana suelta y en la puerta de su origen (si no, el coste viajaría sin recorrer el camino). El cupo del Cap de Fundación se reserva al lanzar contando las caravanas de fundación vivas. |
| D87 | **Fundar encima de un campamento**: la regla es la de D16 —a menos de 100 de cualquier campamento no se funda—, y como `fundar` es el único mecanismo, no hay forma de saltársela. |
| D88 | **Visibilidad de las caravanas por tamaño** (Doc 5.12.7, placeholder en `VISION.caravanaGrande`): una comercial con **3 o más carros con animal** se ve a **300** de cualquier plaza o columna de otra Facción, desde que se prepara y mientras viaja cargada; las pequeñas siguen bajo la niebla normal. |
| D89 | **El escuadrón recién reclutado en un campamento se une a la columna** que su héroe lidera a la puerta si le cabe en el Liderazgo (contando lo que ya lleva fuera); si no, nace en el campamento. Reponer uno que ya tiene no lo mueve. |

## 3. Lo que ya existe y se reutiliza (medido en el código)

- `crearFaccion` (`session/comandos/crearFaccion.ts`): sin condiciones, el creador queda Rey. Cubre D5.
- `fondoDeFaccion`, `aportarARefundacion`, `retirarDeRefundacion`, `comprarCaravanaDeRefundacion`
  (`engine/refundacion.ts`): cubren D9 salvo que hoy la caravana lleva destino y ruta y **funda sola al llegar**
  (`avanzarCaravanasFundacion`), y que hoy se aporta desde el **almacén personal**, no desde el carro (D39).
- `adjuntarCaravana` / `soltarCaravana` (`engine/ejercitos.ts`): enganche en marcha, posición = la columna,
  pérdida si la columna cae o se disuelve (Doc 5.13.2-5.13.4).
- `unirseAEjercito` / `unirseEnCampo`: un héroe se une a la columna de otro.
- `fundarAsentamiento` (`session/comandos/fundarAsentamiento.ts`): funda **donde se está** con la columna del actor
  y la deshace dentro (`cruzarLaPuerta`); `exigirPuertaDeFundacion`.
- `campamentoInicial` (`engine/mercenarios.ts`): uno en el centro. `avanzarAparicionMercenarios` se elimina (D32).
- `avanzarSpawnBandidos` / `avanzarAtaquesBandidos` (`engine/bandidos.ts`) y `atacarCampamentoConColumna`
  (`engine/combate.ts`): uno por asentamiento, poder fijo 30, botín 40 madera + 20 piedra + 15 oro, reaparición
  60 min. Base de D21/D37/D42.
- `leva_comunal` (Doc 6, de arranque): milicia de lanceros, leñadores, granjeros. Es lo que presta D25.
- `enRefugio` (`engine/zones.ts`): precedente para la protección dentro del campamento (M4).
- `MERCENARIOS.mercado` (`constants.ts`): ya hay pila por bien (`topePorBien` 300, reposición cada 3 h), margen
  ×1,3. Es la base de D41; falta el cupo por héroe.

### 3.1 Hallazgos del consejo en el código

- **La caravana de campamento no tiene dueño para el motor.** `adjuntarCaravana` (`ejercitos.ts:621`) lo deduce de
  `origenAsentamientoId`; con el campamento como origen no encuentra plaza, así que nadie puede engancharla.
  (`resolverEncuentros` ya no existe desde `f916d39`: ver §3.2.) Hay que revisar **cada sitio que deduce la
  Facción de una caravana por su origen** y pasar a `faccionDeCaravana` (hoy privada en `expansion.ts`: exportarla).
  D36 y D48 lo exigen. «Volver al origen» (D40) también tiene que saber ir a un campamento.
- ~~**M4 no es un solo guard.**~~ Superado por `f916d39` (§3.2): el tick ya no abre combates entre ejércitos.
- ~~**El batch no tiene bots-héroe que pasen por `crearHeroe`.**~~ El batch ya corre sobre `GameSession` con el runner
  de bots (`10019e4`); falta su fase «sin plaza» (§3.2).
- **`engine/refundacion.ts` tiene cambios sin commit** (el gasto pasa a ir por ciudadanos, no por residentes).
  Sus tests pasan sobre `b40af3d`. Cerrar antes de reabrir.

### 3.2 Qué cambian los pasos 1-3 del NPC fuera del motor (revisado sobre `b40af3d`, 2026-10-04)

- **Presencia (paso 7) está hecha casi entera** (`engine/presencia.ts`, Doc 1.10.6): D33, D40 (las caravanas
  adjuntas vuelven solas a su origen) y D40b (la guarnición y la escolta cedida se quedan). Faltan dos cosas:
  - **D33b** (batalla de Unity): sigue con Conquest: propuesta `CQ-010` (2026-10-06). Hoy quien está en batalla no sale del mundo hasta que termina.
  - **Salir y volver desde el campamento de mercenarios.** `salirDelMundo` solo conoce «dentro de una plaza» (un
    asentamiento) y «en columna». `crearHeroe` hace nacer al héroe en una columna (`columnaDeAparicion`) y no hay
    ubicación «dentro del campamento». El paso 2 la tiene que crear (es la que protege M4), y presencia tiene que
    volver a ella. Además, «volver al origen» (D40) tiene que saber ir a un campamento.
- **M4 es más sencillo.** Desde `f916d39` el tick ya no combate solo: los combates entre jugadores los abren
  comandos (`atacarColumna`, `interceptar`, `atacarCampamento`, `asediarPlaza`, con sus `validar*`) más
  `abrirBatalla` de Unity. El único combate automático que queda es el ataque de bandidos (`avanzarAtaquesBandidos`).
  La protección va en los validadores y en `bandidos.ts`.
- **D48 sigue pendiente**: `adjuntarCaravana` aún deduce la Facción por el origen, y `faccionDeCaravana` sigue
  privada en `expansion.ts`.
- **Las reglas solo-NPC de derrota ya no existen** (`session/derrotas.ts` borrado). D59 queda cubierto por el motor:
  quien pierde la casa pasa a un campamento.
- **Diario de comandos** (`408e5b0`): cada comando nuevo de este trabajo (elegir campamento, solicitar ingreso,
  préstamo, comprar en el mercado del campamento, abrir alijo, reclamar caravana…) tiene que entrar en
  `REGISTRO_DIARIO` y ser determinista, o el repaso al cargar no reproduce la versión.
- **Los bots dependen de este trabajo, no al revés.** El paso 4 del doc 12 (cerebro «sin plaza») es nuestro paso 8 y
  necesita los pasos 1-6. Mientras tanto el batch sigue con `crearFaccionNpc` como andamio, así que **eliminar
  fundar a pie (paso 6) no puede romper ese andamio**: o se retiran juntos, o el andamio funda por el mecanismo
  único (D30).
- **El cerebro cazador de los bots** (`bots/cerebro/militar.ts`) busca el campamento de bandidos de SU plaza
  (`asentamientoId === plaza.id`). Los bandidos unificados (paso 4) lo rompen: hay que actualizarlo en el mismo paso.
- **`crearHeroe`** ya admite `controlador` (lo pone el servidor). Añadir `campamentoId` es compatible.
- **Los bots se mudan con `cambiarResidencia`** entre plazas de su Facción, no con `comprarCasa`, así que eliminar
  `comprarCasa` (paso 5) no los toca. `HeroeProyectado.residenciaId` ya existe y sirve para el contador.

## 4. Revisión propia: contradicciones, puntos ciegos y mejoras

### 4.1 Contradicciones — todas cerradas

- **C1** → D30 (un mecanismo, dos llamadores).
- **C2** → D33 + D40: el héroe sale del mundo; la caravana vuelve a su origen, como dice Doc 3.10.
- **C3** → D31 + D46: solo el Rey, con lista de solicitantes; `comprarCasa` desaparece.
- **C4** → D32: sin aparición gradual.
- **C5** → D13 + D33: un no-titular reclama cuando el titular muere, sale de la Facción o no está en el mundo.

### 4.2 Puntos ciegos

- **P1 — cerrado** por D21-D29, D37, D38, D41, D42, D44, D45. Oro de bandidos (y alijos); piedra y madera del
  mercado; trigo de la ración y del mercado; arranque con tropa prestada.
- **P2 — M4**, alcance en §3.1, radio en §8.2.
- **P3 — M3**, contador doble (§8.3).
- **P4 — cerrado** por D39.
- **P5 — cerrado** por D34 + D43.
- **P6 — M2**, cofundadores (§8.3).
- **P7 — cerrado** por D36.
- **P8 — se resuelve calibrando** con D38 (§8).
- **P9 — radio en §8.2.** La comprobación de D16 va en el motor y vale para los dos llamadores (D30).
- **P10 — cerrado** por D17-D18.
- **P11 — cerrado** por D35.

### 4.3 Mejoras

- **M1.** ~~Fondo por Facción~~ — descartada por D39.
- **M2.** Cofundadores = héroes de la columna del titular (§8.3).
- **M3.** Contador doble (§8.3).
- **M4.** Protección del residente dentro del radio del campamento: nadie inicia combate ahí (§8.2).
- **M5.** Registrar el reparto del fondo gastado — aceptada como D34.

### 4.4 El coste de movimiento (hallazgo del usuario, 2026-10-03)

Una columna en marcha come de su carro: **0,15 trigo por soldado y minuto** (`MILITAR.racionPorSoldadoPorMinuto`),
una décima parte acampada (Doc 5.13). Consecuencias:

- **La distancia de los bandidos fija el trigo** (D42). Agruparse escala el alcance solo: 5 héroes llevan 5 carros.
- **Se reposta en cualquier campamento** comprando trigo (D44), además de la ración gratis del residente (D24).
- **Explotación del sobrante**: el carro devuelve lo que sobra al almacén de origen. Si ese almacén es el personal,
  la ración gratis se acumula hasta pagar el trigo de la caravana. **El sobrante de la ración gratis debe volver al
  campamento**, no al almacén personal (D50).
- **La tropa prestada come en marcha** (D25 solo la exime en el campamento).
- **El viaje de fundación también come**, y puede comer de la caravana por encima de su valor de fundación (D44b).
- **El trigo comprado puede hacer que cazar bandidos dé pérdidas** — ver §8.1: es la restricción que más pesa en la
  calibración.

## 5. Plan (provisional, se reordena al cerrar §7)

**Diseño pendiente antes del código:** §7 y §8 cerrados → bots-héroe (§7) → alijos (D29).

0. ~~Cerrar los cambios sin commit de `engine/refundacion.ts`.~~ **Hecho** (`c10e454`).
1. ~~**Campamentos iniciales** por separación mínima, con semilla derivada; eliminar la aparición gradual.~~ **Hecho**
   (`3b52ae6`): 4-5 campamentos con separación 900.
2. ~~**`crearHeroe` con `campamentoId`**, contador doble y proyección para la pantalla de elección; **protección
   dentro del campamento** (M4).~~ **Hecho** (D72-D79): ubicación `mercenarios`, nacer dentro, `entrarEnCampamento` /
   `salirDelCampamento`, acciones del campamento dentro o en la puerta, protección de 60 en `atacar` y en los bandidos,
   lista de campamentos en la proyección sin héroe. Quedan para después: la visión del mapa alrededor del campamento
   estando dentro, y retirar `columnaDeAparicion`/`puntoDeAparicion` (solo los usa la fixture de tests de fundar a pie;
   se van con el paso 6).
3. ~~**Economía del residente**~~ **Hecho** (`db99a45`, `2b71510` y el del préstamo): pilas de madera, piedra y trigo
   que se reponen solas (el comercio del mundo es cero al empezar) y cupo diario; trigo para repostar en cualquier
   campamento; ración de 60 al salir cada 30 min, que vuelve al campamento y no se guarda; préstamo de una escuadra de
   leva comunal por tropa, hasta las tres, gratis al pedir y al reponer (D80), retirada por el tick al dejar de residir.
   **Falta**: que la pila de trigo (300) se calibre.
4. ~~**Bandidos unificados por niveles** (D21, D22, D26-D28, D37, D42) y **alijos** (D29).~~ **Hecho** (`b32f8a0` y el de
   alijos): niveles al azar, anillo por demanda, botín de oro a `oroDeBotin` (solo mercado del campamento y fondo; lo
   retirado del fondo vuelve como oro de botín), rendimientos decrecientes, alijos por héroe. **Abierto**: con alijos por
   héroe, quien recorre todas las zonas abre 6 × campamentos (120-150 de oro en un mundo de 4-5), no los 30 de D62.
   **Falta**: el bot cazador sigue buscando solo los bandidos de su plaza; los del anillo los cazará el cerebro «sin
   plaza» (paso 8).
5. ~~**Ingreso con lista de solicitantes del Rey** (sustituye `unirseAFaccion`); **eliminar `comprarCasa`**.~~ **Hecho**:
   `solicitarIngreso` / `responderSolicitud` (solo el Rey) con `Faccion.solicitudesIds`; `comprarCasa` borrado del motor, la
   sesión, la autorización y el canon. Quien vive en un campamento entra a vivir en una plaza de su Facción con
   `cambiarResidencia`, que ya no exige residir antes en otra plaza. Sin invitaciones: D46 deja solo la lista del Rey.
   **Falta**: el Rey bot que acepta (paso 8).
6. ~~**Fundación única**~~ **Hecho** (`a274311`, `98e3da3` y el de fundar a pie): caravana de campamento sin destino con
   titular, `fundar` como mecanismo único (también para la caravana de plaza), D16 con exclusión de 100, cofundadores de la
   columna; fondo en el campamento donde se está, desde almacén o carro; vuelta sola, reclamo, desarme y caducidad con
   devolución; fundar a pie fuera del juego (su código queda solo como fixture de tests, `fundarDePrueba`). **No hecho**:
   D44b (la caravana del campamento lleva justo el coste, no hay excedente que comer). **Pendiente del canon**: Doc 1.3
   sigue diciendo que el servidor arranca con Facciones NPC (D53), que va con el NPC fuera del motor.
7. ~~**Presencia**~~ **Hecho** (D64-D71 en `engine/presencia.ts`, el tick y la defensa; canon Doc 1.10.6). **Ojo**: con
   ticks de 1 minuto, 2:30 y el tope de 3 minutos caían en el mismo tick; resuelto subiendo el tope a 5 minutos (D83). D33b sigue
   con Conquest. Lo que era: base hecha en `b40af3d` (§3.2), a corregir con D64-D71: dentro de plaza o campamento solo una
   marca (sin contenedor `fuera`, sin defender en persona); aplazar la salida si le persiguen (máx. 3 min); mando al
   conectado más antiguo; caravana de fundación que vuelve se desarma y devuelve. Lo hace esta línea de trabajo (la
   sesión NPC queda parada hasta que los pasos 0-6 estén en `main`). Piezas: `engine/presencia.ts` (salir/volver,
   sucesión filtrando conectados), `avanzarTick.ts` (`conSalidasDelMundo`: aplazamiento con tope), comandos
   `conectarse`/`desconectarse`, la autorización (hoy mira `heroe.fuera`; con D64 hace falta otra marca), contrato v1
   (`dto.ts`, schema, `Docs/Coordinacion/01`), Doc 1.10.6 y los tests `presenciaEnElMundo` y `tiempoReal`. El runner
   de bots y el hub WS no cambian. D33b en batalla, **con Conquest** (protocolo BA/CQ; propuesta `CQ-010`).
8. **Bots-héroe** que arranquen como los jugadores y calibración en batch con D38.
9. ~~**Canon, contrato, clientes**~~ **Hecho** (2026-10-04). Canon sin Facciones NPC (glosario, Doc 2, 3, 5, 6) —
   `comprarCasa` y `unirseAFaccion` ya estaban fuera—; contrato v1 con los campos de campamento del héroe y
   `EscuadronDto.prestada`; doc 02 con `crearHeroe(campamentoId)`, la pantalla `sinHeroe.campamentos` y §4.2b (comandos
   del campamento); cliente de admin sin los comandos borrados; cliente de jugador 0.8.0 (`BronzeAgeClient@7529161`):
   nacer en un campamento, pantalla Campamento, campamentos y alijos en el mapa, pedir ingreso y fundar con la
   caravana. Verificado en vivo hasta salir y volver a entrar; la compra y el enganche de la caravana, solo con tipos.
   **Pendiente**: los rechazos llegan solo con el código (`mercenarios.invalido`), sin el motivo.

## 6. Revisión por consejo (2026-10-03)

Cinco consejeros (contrarian, primeros principios, expansionista, outsider, ejecutor), revisión cruzada anónima y
síntesis. Veredicto: **no estaba listo**; todos coincidieron en que P1 bloqueaba el ciclo entero (el jugador
elegía campamento y se quedaba sin salida).

- **Coincidencias**: P1 era el núcleo y faltaba en el plan; la protección del campamento iba tarde (paso 6 tras el
  spawn del paso 2); «desconectado» no existía en el motor; M1 y M2 con apoyo unánime.
- **Choques**: cómo resolver P1 (kit inicial frente a contratos con NPC); C1 (asimetría frente a unificar) — se
  impuso «un mecanismo, dos llamadores», adoptado como D30; cómo romper el contrato — irrelevante tras D18.
- **Lo que salió en la revisión cruzada**: el mundo vivo a migrar (resuelto por D17); con 5 jugadores repartidos en
  4+ campamentos nadie junta grupo (D1 contra D9) — sigue siendo un riesgo del playtest (§8.2); los hallazgos de
  código del §3.1.

## 7. Puntos abiertos

1. **Arquitectura de NPC fuera del motor (D52)**: `Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md`.
2. ~~**Alijos (D29)**~~: cerrado por D60-D63.
3. ~~**El préstamo al salir del mundo**~~: cerrado por D71.

## 8. Calibración (2026-10-03, aceptada como punto de partida — D51)

Todo PLACEHOLDER, derivado de constantes que ya existen. Mapa 2000×2000; velocidad ligera 20 u/min; ración
0,15 trigo por soldado y minuto (0,015 acampado); carro 500.

### 8.1 Cuánto cuesta la caravana y de dónde sale el oro

`costoCaravanaFundacion()` = 160 madera + 20 piedra + 100 trigo + 100 oro. Al 75 %: **120 madera, 15 piedra,
75 trigo, 75 oro.** Comprado todo en el mercado del campamento (precio base × 1,3):

| Recurso | Cantidad | Oro |
|---|---|---|
| Madera | 120 × 1,3 | 156 |
| Piedra | 15 × 1,56 | 23 |
| Trigo | 75 × 1,95 | 146 |
| Oro | 75 | 75 |
| **Total** | | **≈ 400 oro** |

D38 (8 h × 3 héroes = 24 héroe-hora) → **≈ 17 oro netos por héroe y hora**, con los gastos ya descontados (trigo de
las salidas, reposición de la tropa prestada, deuda).

**La restricción que más pesa: el trigo de cada salida.** Con un escuadrón prestado de 15 y bandidos a 200 de
distancia, una salida son 20 min de marcha (15 × 0,15 × 20 = **45 trigo**) más el combate acampado (poco). Si ese
trigo se compra a 1,95, cada salida cuesta **≈ 88 oro**: ningún botín razonable lo cubre y cazar bandidos da
pérdidas. Por eso **la ración gratis (D24) tiene que cubrir las salidas de nivel 1** y el trigo comprado (D44)
queda para lo que la exceda: niveles altos, más gente, viaje de fundación.

Propuesta:

- **Ración gratis**: lo que come **la columna con la que sale durante 45 min de marcha** (D81), que se rellena **cada
  30 min** en su campamento. Lo que sobra vuelve al campamento (D50). No acumulable. *(Antes 60 trigo fijos: con el
  coste real del terreno no llegaba al anillo y volvía; ver D81.)*
- **Ciclo de una salida de nivel 1**: 10 min de ida + combate (≤ 15 min en Unity, inmediato con números) + 10 min
  de vuelta ≈ **30 min → 2 salidas por hora**.
- **Botín** (oro, D22), por héroe y salida, con D21 (prima pequeña por agruparse):

  | Nivel | Héroes | Poder | Oro por héroe |
  |---|---|---|---|
  | 1 | 1 | 20 | 9 |
  | 2 | 2-3 | 60 | 10 |
  | 3 | 4-5 | 120 | 11 |

  2 salidas/h × 9 = 18 oro/h, menos deuda y reposiciones ≈ **17 netos**: cuadra con D38.
- **Rendimientos decrecientes (D26)**: botín completo en las **8 primeras** destrucciones de las últimas 24 h; luego
  −15 % por cada una; **a partir de la 14ª, solo experiencia.** Las 8 h de D38 repartidas en 2 días (≈ 4 h/día,
  8 salidas) quedan dentro del tramo completo; el que juega 12 h seguidas no va 3 veces más rápido.
- **Tropa prestada (D25, D45)** — *corregido por D80: gratis, sin deuda, hasta una escuadra de cada tropa de leva*: **15 milicias de lanceros** (poder ≈ 30) contra un nivel 1 de 20: gana casi
  siempre, con bajas. **Reposición: 1 oro por unidad** (lo mismo que un escalón 1 de `RECLUTAMIENTO_ORO_POR_ESCALON`).
  **Deuda**: el préstamo en sí no cuesta; solo las reposiciones, descontadas del botín. Tope de deuda **30 oro**:
  por encima, el campamento no repone hasta que se salde.
- **Mercado (D41)**: pila compartida de **300 madera y 60 piedra** por campamento (reposición cada 3 h, como ya
  hace `MERCENARIOS.mercado`). Cupo por héroe y día: **60 madera y 10 piedra**. Un grupo de 3 junta en un día
  180 madera y 30 piedra: más de lo que pide la caravana, pero no puede vaciar la pila de un campamento lleno.

### 8.2 Radios y separación

- **Radio del campamento (M4, protección)**: **60**. Dentro, nadie inicia combate. Cubre el layout del campamento y
  su entorno inmediato; es menor que la vista de un héroe solo (80): se ve venir a quien espera fuera.
- **Radio de exclusión para fundar (D16)**: **100** = 60 de protección + 30 de `ZONA_INFLUENCIA.radioInicial` +
  10 de margen. La zona de una plaza nueva no puede nacer pisando el campamento.
- **Distancia de los bandidos (D42)**: **anillo de 150 a 250** alrededor del campamento, igual para todos. Fuera de
  la protección, a 10 min de marcha a velocidad ligera (con el trigo de §8.1).
- **Cuántos a la vez (D42)**: **1 por cada 2 residentes de Facciones sin asentamiento**, mínimo 1, máximo 6 por
  campamento. Reaparición por demanda (D28): cuando hay menos de los que tocan, aparece uno cada **10 min**.
- **Separación entre campamentos (D1)**: **≥ 600** = dos anillos de bandidos (2 × 250) + 100 de margen, para que
  los anillos no se pisen. En 2000×2000 salen **≈ 6-9 campamentos.**
  **Riesgo del playtest (consejo)**: con 5 jugadores en 6-9 campamentos, cada uno queda solo y nadie junta grupo.
  Para el playtest, **separación 900 → ≈ 4 campamentos**, y subir hacia 600 cuando haya población.

### 8.3 Contador doble y cofundadores

- **M3**: la pantalla de elección muestra **dos números** por campamento: **eligieron como inicial** (D4, crece
  siempre) y **residentes ahora** (cuántos viven allí en este momento). El primero dice la historia; el segundo, la
  congestión real. Con D41 (la pila compartida), el segundo es el que importa para decidir.
- **M2**: los **héroes que estén en la columna del titular al llamar `fundar`** son los cofundadores, hasta
  `FUNDACION.maxJugadoresFundacionGrupal` (5). Unirse a la columna ya es el consentimiento (D12).

## 9. NPC fuera del motor (D52)

Movido a su propio documento: `Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md` (2026-10-04). Es un cambio de
infraestructura que va más allá de los campamentos; los bots-héroe (D53-D59) se construyen ya sobre él.
