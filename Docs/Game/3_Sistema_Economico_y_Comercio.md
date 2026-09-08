# 3. Sistema Económico y de Comercio

> Auditado contra el código real (trade.ts, market.ts, combate.ts, constants.ts). Cada sección indica su estado: ✅ implementado tal cual el diseño, 🔷 implementado con simplificación intencional de Fase 0, 🔶 implementado parcialmente, ❌ no implementado.

## 3.1 Oro — ✅ implementado
- NO es moneda acuñada: es METAL PRECIOSO EN BRUTO PESADO (sistema tipo siclo/shekel).
- ORIGEN: mina de oro en el mapa, mismo patrón que cantera/mina de cobre (nodo finito, con reemplazo automático al agotarse — ver Doc 4.2); comisiones de comercio (3.5); y **recaudación en el asentamiento según población y clase** (Doc 4.1, bloque "economía del oro").
- USO: medio de pago en Mercado (3.3) y en comisiones (3.5); **coste de reclutamiento** salvo la milicia inicial (Doc 5.7/5.8); **compra de animales de arrastre para caravanas**, buey incluido (3.13.2); Maravilla (Doc 4.2.1); **intel/mapas en la Taberna** (ficha aparte, sin implementar). Compra de tecnología/mercenarios/sueldos: esos sistemas (Aedas, mercenarios) no están implementados todavía en Fase 0.
- **BLOQUE "ECONOMÍA DEL ORO" (2026-09-08 — diseño cerrado, calibración en curso; `Consideraciones/Economia_Del_Oro_Definicion.md`):** el oro es la única moneda que convierte entre ambiciones (ejército / flota comercial / intel). El sentido del bloque es que el oro te obligue a elegir: no se puede ir a por las tres a la vez. La recaudación por población es la FUENTE que lo hace pagable sin mina; el reclutamiento en oro y los animales en oro son los SINKS (la intel de taberna, el sink recurrente, va aparte); y la política "Presión Fiscal" (Tesorero, Doc 4.4) es el knob de más-oro-a-cambio-de-menos-crecimiento.
- ENTRE JUGADORES: recurso más de trueque, sin restricción especial.

## 3.2 Trueque de materiales (acuerdos entre Facciones) — 🔷 implementado con simplificación intencional
- Contrato MARCO abierto en el tiempo: `proponerTrueque` crea el acuerdo (cantidad total pactada por lado, plazo por defecto 200 ticks).
- **PROPUESTA → ACEPTACIÓN (2026-09-07, decisión del usuario)**: el acuerdo nace `'propuesto'` y **no obliga a nadie** hasta que el lado receptor (B) contesta con `aceptarTrueque` o `rechazarTrueque`. Ninguna caravana lo mira mientras siga propuesto. Hasta esa fecha se aceptaba solo al proponerse — era una simplificación declarada de Fase 0, con el motivo escrito en el código ("requiere un jugador interactivo real, fuera de alcance del prototipo"), y ese motivo caducó con el jugador situado.
  - **El plazo se cuenta de nuevo desde el sí**: una propuesta contestada al filo no puede nacer ya sin tiempo material de cumplirse, porque entonces el que acepta de buena fe se comería la penalización por incumplir (Doc 2.7).
  - Una propuesta **sin contestar caduca sin penalizar a nadie** (no hubo promesa que romper); una **rechazada** queda como `'rechazado'` y no se borra: una respuesta es información, y es distinta de un silencio.
  - El **Camino Comercial** del par (Doc 1.6) se traza **al aceptar**, no al proponer: es infraestructura física permanente, y una propuesta unilateral no basta para plantársela a un vecino que no ha dicho ni sí ni no.
  - **Quién contesta cuando no hay jugador**: las plazas NPC lo hacen en el mismo tick (`responderPropuestasNpc`), con el mismo colchón de excedente que el NPC exige cuando es él quien pide — así el trueque entre dos NPC sale igual que antes de que la aceptación existiera. El laboratorio (`simulacionAutoComercio`) representa a las dos plazas y acepta por la receptora, pero **pasando por la misma función** que usaría un jugador.
  - Consecuencia: el NPC **ya puede contestar a la propuesta de un jugador**, algo que antes se prohibía a sí mismo justamente porque pactar sin consentimiento le habría comprometido recursos. Lo que sigue sin hacer es proponerle un trueque de SUPERVIVENCIA a un humano, y ahora por otro motivo: un salvavidas no puede quedarse esperando a que alguien se conecte.
- **SIMPLIFICACIÓN DE FASE 0, revisada (ampliación de comercio, a petición del usuario)**: el diseño objetivo dice que un jugador debe TOMAR una caravana y MOVERLA físicamente — Fase 0 ya no genera caravanas de la nada como antes. Ahora exige tener una caravana PROPIA construida y **'disponible'** en el asentamiento origen (ver 3.12); `asignarCaravanasATrueque` la asigna automáticamente al envío pendiente, sustituyendo por ahora la elección manual del jugador ("solo simulación": en el diseño objetivo el jugador elige la caravana, la carga y la escolta a mano). Si no hay ninguna disponible, el envío simplemente espera al siguiente tick. **Plan: pasar a asignación/carga manual en Fase 1+**, tal como está descrito el diseño objetivo.
- Cuando hay menos caravanas disponibles que envíos pendientes en un mismo asentamiento (varios acuerdos compitiendo a la vez), se prioriza por un SCORE ponderado (ver 3.12: urgencia por expiración del acuerdo, urgencia por volumen pendiente, cercanía del destino) — no por orden de llegada.
- La caravana sí viaja de verdad por el mapa: desde Fase 0.3, sobre una RUTA calculada por pathfinding (rodea terreno costoso, ver Doc 1.6/3.6) en vez de una línea recta, con velocidad ×2 respecto a la original (ver 3.6) modulada por el coste del terreno que cruza en cada momento — y entrega proporcionalmente al llegar. Al entregar, la caravana propia vuelve a estar 'disponible' en el origen — no desaparece, es un activo persistente y con costo (3.12), no un objeto de un solo uso.
- Cumplir o incumplir un trueque ajusta el score de reputación de Facción (Doc 2.7) — esto sí está conectado.

## 3.3 Órdenes de mercado (comercio abierto) — ✅ implementado
- Un asentamiento coloca órdenes de compra/venta; cualquier jugador puede dejar lo pedido o comprar lo ofrecido.
- Se pagan con ORO.
- **Gate nuevo (ampliación de comercio, a petición del usuario)**: colocar una orden ahora exige tener un Mercado activo (ver 3.12) — antes cualquier asentamiento podía hacerlo desde el tick 0, sin edificio.
- **EL MOSTRADOR (2026-09-07, decisión del usuario)**: una orden es una **oferta en pie EN UNA PLAZA**, no una entrada en una bolsa global. Se cumple **allí**, con alguien que ha ido hasta la plaza con su columna (`comerciarEnPlaza`). El emparejamiento automático entre cualquier par de asentamientos —que liquidaba al instante y sin que nada recorriera el mapa— **se retiró**: era la última simplificación viva del sistema, y la propia sección la marcaba como deuda.
  - **La plaza VENDE**: el jugador paga oro DE SU CARRO y la mercancía sube AL CARRO. **La plaza COMPRA**: descarga mercancía DEL CARRO y el oro sube AL CARRO. El viaje completo son cuatro actos —cargar en tu plaza, llegar, comerciar, volver y depositar—; el último ya lo hacía entrar en tu residencia (Doc 1.10.3).
  - **El oro pesa y ocupa carro** (3.1: es metal precioso pesado, no moneda acuñada). Eso pone un techo físico a cuánto se mueve de una tacada: comprar barato lejos y vender caro en casa cuesta viajes, no un clic.
  - **Manda el Líder de la columna**: el carro es común (Doc 5.13.2), y sin esa condición cualquiera que se uniera en campo podría gastarse el oro de todos.
  - **Sirve lo que puede** en vez de fallar cuando se pide de más —el tope sale a la vez de la orden, del almacén de la plaza, de su oro, del carro y de lo que se lleve encima—, pero **falla si no puede servir nada**.
  - **Las órdenes CADUCAN** (`MERCADO.plazoOrdenMinutos`, 200 minutos, el mismo plazo que un trueque). No es un extra: sin emparejamiento automático nada las cerraría nunca, y una plaza acumularía ofertas eternas a precios de hace cien ticks.
  - **Visibilidad**: el escaparate de una plaza ajena se ve **al estar en su puerta**, y solo lo que sigue en pie. Un mercado enseña sus ofertas a quien está dentro; una lista global dejaría leer los precios del mundo entero sin moverse.

## 3.4 Precios dinámicos — ✅ implementado
- Precio de referencia por defecto: precio base escalado por escasez/abundancia GLOBAL (stock objetivo de referencia = 500, con clamp entre ×0.4 y ×3).
- El jugador puede sobreescribir el precio manualmente al colocar una orden.
- NO existe mercado NPC de respaldo — correcto según diseño.

## 3.5 Comisiones de comercio — ✅ implementado (parcial, como marca el propio diseño)
- Comisión del 3% dentro de la misma Facción vs. 8% externa, aplicada tanto en trueque como en mercado, modulable por política ("Aranceles/Comercio Abierto") y por reputación de Facción.
- **En el mostrador (3.3), la paga quien toma la orden y se la queda la plaza donde ocurre (2026-09-07)**: comprando se paga de más, vendiendo se cobra de menos. Aquí el oro **se conserva** — el emparejamiento automático que esto sustituye acuñaba la comisión de la nada y se la regalaba al vendedor. La comisión de trueque (`comisionDeEntrega`) sigue como estaba.
- PENDIENTE (sin cambios): en qué se usa la riqueza acumulada; nivel intermedio de comisión para Facciones aliadas/vasallas de la misma Liga.

## 3.6 Categorías de caravana (heredado de Iberia) — 🔶 parcial: catálogo existe, solo 1-2 de 4 se usan
1. **Comercial**: ✅ implementada y en uso — única categoría que el motor instancia realmente para Trueque/Mercado, y la única que es un activo PROPIO y persistente (ver 3.12), en vez de efímera. **El revamp de 3.13 (Pasos 1-2 implementados) la hace una caravana COMPUESTA** —lista de carros, cada uno con su animal— y deriva de ahí capacidad y velocidad; ya **no** tiene entrada en `CARAVANA_CATALOGO`. Las otras tres categorías no cambian y siguen leyéndolo.
2. **Militar**: catálogo definido (capacidad/velocidad propias) pero el motor NUNCA la dispara ni le da comportamiento distinto.
3. **De construcción**: sí tiene uso real (Caravana de Fundación, Doc 1.8), pero es un mecanismo aparte del de Trueque/Mercado — no forma parte de la flota propia de 3.12.
4. **De contrabando**: solo datos, sin uso real.
- **Velocidad ×2 en las 4 categorías (ampliación de comercio, a petición del usuario)**: el diagnóstico mostró que a la velocidad original un trueque de tamaño moderado a distancia media podía necesitar más ticks de viaje (varios envíos en serie, ver 3.2) que el plazo por defecto del acuerdo (200 ticks) — expiraba antes de poder completarse pase lo que pase. Se dobló la velocidad de las 4 categorías a la vez para no dejar el catálogo inconsistente entre sí, aunque solo Comercial (y Construcción, Doc 1.8) tienen uso real hoy.
- **Movimiento con coste de terreno (Fase 0.3, ver Doc 1.1/1.5/1.6)**: la "velocidad" del catálogo sigue siendo la base, pero el avance real por tick ahora se divide entre el coste de terreno en la posición actual (llano≈1, colina/montaña más lento, agua/cima muy caro) — cruzar relieve accidentado de verdad tarda más ticks. Cada caravana calcula su propia RUTA por pathfinding al lanzarse (rodea relieve costoso en vez de ir en línea recta), salvo que ya exista un Camino Comercial para ese par de asentamientos (Doc 1.6), en cuyo caso reusa su polilínea y recibe además el bonus de velocidad del camino. Caravanas de partidas guardadas antes de Fase 0.3 (sin ruta) siguen moviéndose en línea recta sin coste de terreno — compatibilidad hacia atrás sin migración. Ver `engine/movimiento.ts`, `world/rutas.ts`.
- PENDIENTE: conectar Militar/Contrabando a sus disparadores correspondientes (equipo militar antes de asedio, mecánica de detección reducida).

## 3.7 Transporte individual espontáneo (heredado de Iberia) — 🔶 el inventario existe; el transporte libre, no
- **El inventario personal SÍ existe** desde el jugador situado: el carro de la columna (`Ejercito.suministro`, capacidad `LOGISTICA.capacidadCarroPorJugador` × participantes más las caravanas adjuntas, Doc 5.13.2) admite cualquier recurso, incluido el oro. Se carga al salir eligiendo qué llevar (Doc 1.10.2) y se vuelca al almacén al entrar en tu residencia (Doc 1.10.3).
- Desde 2026-09-07 es además **la única vía por la que la mercancía de una orden de mercado cambia de manos** (3.3): el transporte físico dejó de ser opcional.
- Lo que sigue sin existir es el intercambio **directo entre dos jugadores** fuera de una plaza — cara a cara en mitad del mapa, sin mercado ni acuerdo de por medio.

## 3.8 Bonificación por distancia (heredado de Iberia) — ✅ implementado (parcial, solo trueque)
- Aplica como bonus a la comisión de trueque: hasta ×1.5 a partir de 600 unidades de distancia recorrida.
- NO aplica a órdenes de mercado. Ya no por la razón vieja —"se liquidan al instante sin viaje"—, que dejó de ser cierta en 2026-09-07: ahora el viaje existe, pero lo hace **el jugador**, y el bonus está definido sobre la distancia que recorre una CARAVANA de un acuerdo. Aplicarlo al mostrador es una decisión de diseño abierta, no un hueco de implementación.

## 3.9 Dependencia logística real (heredado de Iberia) — 🔶 implícita, no es mecánica dedicada
No hay un sistema que detecte explícitamente "cortar una ruta" como evento de guerra económica. En la práctica, si no llegan caravanas, el receptor simplemente no recibe el recurso — es una consecuencia natural del modelo de trueque, no una mecánica de intercepción/guerra económica dedicada todavía (para eso hace falta que el combate de caravanas —3.10— interactúe activamente con acuerdos en curso, lo cual no está conectado aún).

## 3.10 Combate de caravanas — ✅ implementado (simplificado)
- **Ya NO es un comando** (2026-09-04, Paso 11 del movimiento de ejércitos): interceptar dejó de declararse desde un asentamiento y pasó a ser lo que le ocurre a un ejército que se cruza con una caravana enemiga en el mapa (Doc 5.12.3). No hace falta General porque no hace falta orden: hace falta tener una columna ahí, que es más caro y más interesante. El botín viaja en su carro y llega a casa al replegarse.
- Calcula poder del atacante contra una defensa base de caravana FIJA — que sigue siendo el valor para una caravana SIN escolta. Con escolta ya no aplica: se resuelve contra el poder real del ejército que la acompaña (5.13.3), que era justo lo que este apartado daba por no modelado. **Con el revamp (3.13, sin implementar) hay TRES capas**: sin nada → `defensaBaseCaravana`; con escolta sin héroe (escuadrones cedidos, 3.13.4) → poder de esos escuadrones; adjunta a un ejército → poder del ejército. La defensa base pasa a ser el caso "cero de las tres partes".
- **UNA CARAVANA SOLA SE DEFIENDE DE UN JUGADOR SOLO** (a petición del usuario, 2026-09-06). Desde que el héroe combate por sí mismo (Doc 5.1), la defensa base tiene que dejar clara una frontera: **un jugador solo no roba caravanas**. La caravana lleva carreteros y guardias; un hombre a caballo no la para.
  - La cifra **se deriva del poder del héroe**, no se escribe suelta: `defensaBaseCaravana = poderHeroe × 1.7`. Se deriva porque `poderBase` sigue siendo placeholder (Doc 5.8) y una constante a mano se desincronizaría en cuanto se recalibre el roster — mismo criterio que el coste de Liderazgo (5.11.1).
  - **Por qué 1.7 y no menos.** Con la varianza de combate (±15% por bando, 5.2.5), para ganar SIEMPRE hace falta superar `1.15 / 0.85 = 1.353`. 1.7 deja margen por encima de ese mínimo sin acercarse al escuadrón más barato: hoy son **25** de defensa, frente a 15 de un héroe y **50** de una milicia completa. Un jugador solo pierde siempre; cualquier escuadrón real gana siempre.
  - Lo que esto cierra: el rol de **salteador unipersonal** —rápido, barato y sin nada que perder— que el héroe combatiente habría abierto de par en par. Depredar caravanas sigue exigiendo tropa.
- Captura exactamente el 50% de la carga si el atacante gana (umbral de captura, coincide con el diseño).
- NO hay reparación NPC tras derrota — la caravana simplemente se elimina si es capturada (el diseño original preveía reparación; simplificado en Fase 0).
- **Amenaza NPC nueva — IMPLEMENTADO (a petición del usuario, ver Doc 1.9)**: campamentos de bandidos en bosques no reclamados atacan caravanas que pasen cerca, con la misma resolución de combate asimétrico que la intercepción entre Facciones, pero contra un bando NPC — ver Doc 1.9 para spawn, recompensa y reaparición, `engine/bandidos.ts` para la implementación.

**ESCOLTA DE CARAVANAS — resuelta por la mecánica de ejércitos (2026-09-02, ver Doc 5.13.3).** La escolta que este apartado daba por no modelada ya tiene forma: una caravana puede ir **adjunta a un ejército** y hacer su entrega normal mientras marcha con él. Deja de defenderse con la defensa base fija y pasa a estar protegida por el poder de combate real del ejército. Se autoequilibra con la regla de velocidad (Doc 5.12.5): el ejército va al ritmo de su integrante más lento, así que escoltar una caravana comercial (16) frena a un ejército ligero (20) y le quita la capacidad de cazar otras caravanas — **no se puede escoltar y depredar a la vez**. Si el ejército es derrotado, la caravana escoltada se pierde con él.

**Pero quedarse sola no es lo mismo que ser capturada** (a petición del usuario, 2026-09-06). Si el ejército se deshace sin que nadie lo venciera —porque todos sus integrantes se desconectaron y cada uno se llevó lo suyo (Doc 1.10.6)—, la caravana **vuelve por su cuenta al asentamiento de origen**, haciendo el camino y siendo interceptable durante el regreso. Al derrotado se las quita un enemigo, porque el tren de suministros es objetivo militar; a estas no las venció nadie y los carreteros se vuelven a casa. Si su origen ya no existe, se pierden.

**LAS CARAVANAS TAMPOCO CRUZAN EL AGUA** (2026-09-02, ver Doc 1.0b): si no hay ruta por tierra entre origen y destino, la caravana **no sale** y la carga se queda en el almacén; el camino comercial tampoco se traza. Aplica igual a las Caravanas de Fundación, que además no pueden fundar en un punto inalcanzable. Comerciar entre dos costas enfrentadas exigiría comercio marítimo, que sigue fuera de alcance (3.11).

**LA INTERCEPCIÓN ES EMBOSCADA, NO PERSECUCIÓN** (hallazgo de la revisión por consejo, 2026-09-02): con las velocidades de tropa de Doc 5.12.5, solo un ejército de infantería **ligera** (20) supera a una caravana comercial (16) y puede darle caza. Un ejército medio (16) la iguala y uno pesado (12) no la alcanza jamás; el contrabando (24) escapa de todo. Interceptar con tropa pesada solo es posible **estando ya apostado en la ruta**, no persiguiendo.

## 3.11 Comercio marítimo — ❌ fuera de alcance (correcto, según diseño)
Requiere tecnología de barcos + puertos. Nada implementado — consistente con que Fase 0 es 100% terrestre (eje naval pospuesto a fase completa).

## 3.12 Mercado como edificio y flota de caravanas propias — ✅ implementado (ampliación de comercio, a petición del usuario)

**Contexto:** hasta esta ampliación, "Mercado" era solo el nombre de la mecánica de órdenes (3.3) — no existía como edificio en ningún catálogo, pese a que el diseño original (Doc 0/2) ya describía al Tesorero como responsable de "trueque + Mercado" como dos cosas separadas. Al mismo tiempo, las caravanas de Trueque se creaban de la nada en cada envío, sin ningún concepto de cuántas tenía un asentamiento, activas o no. Ambos huecos se cierran juntos.

**Mercado (edificio, Doc 4.2.1)**: adición MANUAL de Gobernador/Maestro de Obras (ver Doc 4.2, cambio de base a petición del usuario — la política "Construir Mercado" que existía antes se retiró), mismo patrón que Barracón/Galería de tiro/Palacio — no auto-construcción. Gatea DOS cosas: colocar órdenes de mercado (3.3) y construir caravanas propias (más abajo). Con niveles internos que administran el cupo de flota — ver catálogo completo en Doc 4.2.1.

**Flota de caravanas propias**: `construirCaravanaComercial` — activo persistente, no efímero, que cuesta 50 madera y cuenta contra `cupoCaravanas` (nivel de Mercado + política "Ampliación de Flota", +1 aditivo) hasta que se pierda capturada en combate (3.10). **No se puede desmantelar voluntariamente** (decisión confirmada con el usuario). Nace 'disponible' en el asentamiento; al ser asignada a un envío pasa a 'en_transito'; al entregar, vuelve a 'disponible' en el origen — se reutiliza, no se reconstruye en cada viaje.

**Cooldown de creación (2026-08-20, a petición del usuario)**: tras crear una caravana desde un asentamiento —comercial o de Fundación (Doc 1.8), COMPARTIDO entre las dos— hay que esperar `CARAVANA_COOLDOWN.ticksCooldown` (`constants.ts`, 10 ticks, parametrizable) antes de poder crear otra desde el mismo asentamiento. Evita que se spamee la creación cuando una caravana recién salida es destruida (bandidos, Doc 1.9; combate de caravanas, 3.10) y el cupo/recursos vuelven a estar disponibles de inmediato — sin esto, un asentamiento con recursos de sobra podía reconstruir una caravana cada tick indefinidamente. Regla del MOTOR (`puedeCrearCaravana`/`ticksCooldownCaravanaRestantes`, `engine/asentamientoQuery.ts`; aplicada dentro de `construirCaravanaComercial` y `lanzarCaravanaFundacion`), no un heurístico de la interfaz ni del NPC — así que gatea igual la creación manual y la de la gobernanza NPC. El asentamiento guarda `ultimaCaravanaCreadaEnTick`; ausente = nunca creó ninguna, cooldown no aplica.

**Scoring de asignación** (`asignarCaravanasATrueque`, ver 3.2): cuando hay menos caravanas disponibles que envíos pendientes en un asentamiento, se ordenan por un score 0-100 ponderado:
- Urgencia por expiración del acuerdo (50%): cuánto del plazo ya se consumió.
- Urgencia por volumen pendiente (30%): qué fracción del total pactado sigue sin entregar.
- Cercanía del destino (20%): destinos más cercanos rinden más envíos por caravana disponible.

Pesos y la distancia de referencia (600 unidades) son PLACEHOLDER, confirmados con el usuario, sin calibrar por simulación todavía.

**Políticas nuevas (Tesorero, Doc 4.4)**:
- "Ampliación de Flota": +1 cupo de caravanas (aditivo).
- "Carga Ampliada": ×1.5 la capacidad de carga de las caravanas propias.
- "Rutas Rápidas": ×1.5 la velocidad de las caravanas propias.

**Verificado en el navegador** de punta a punta: colocar una orden o construir una caravana sin Mercado se rechaza; tras construir el Mercado (adición manual a la cola de Gobernador/Maestro de Obras, ver arriba), construir una caravana consume 50 madera y la deja 'disponible'; un trueque activo la asigna automáticamente (`asignarCaravanasATrueque`) y la hace viajar a la velocidad ×2 (tiempo de viaje observado coincide exactamente con distancia/velocidad); al entregar, vuelve a 'disponible' en vez de desaparecer. Sin errores de consola.

## 3.13 Revamp de caravanas — la caravana compuesta — ✅ implementado (2026-09-08); calibración de placeholders continua

> Sustituye el modelo de 3.12 —una caravana `comercial` es un activo único de capacidad y velocidad fijas
> (500/16)— por una caravana **compuesta**: carros, animales y escolta. Decisiones, representación en el motor,
> plan de 5 pasos e invariantes en `Consideraciones/Revamp_Caravanas_Definicion.md`.
>
> **Hecho:** el modelo (`Caravana.carros`, `CARRO_CATALOGO`/`ANIMAL_CATALOGO`, `capacidadCaravana`/
> `velocidadCaravana`, migración de snapshot v11→v12; el viejo `CARAVANA_CATALOGO.comercial` fijo se borró),
> el casco vacío gratis (`crearCaravana`), las piezas (`agregarCarroCaravana`, `comprarAnimalCaravana`,
> `reservarCaravana`, `moverCarroCaravana`), el lanzamiento manual con preparación (`prepararCaravana`,
> `cancelarCaravana`) y la **escolta sin héroe** (§3.13.4). El batch NPC quedó **bit-idéntico** en los cuatro
> pasos. Todas las cifras (`CARRO_CATALOGO`, `ANIMAL_CATALOGO`, `CARAVANA_PREPARACION`, `CARAVANA_ESCOLTA`)
> son placeholder — calibración por simulación, continua.
>
> **Lo de 3.12 que NO cambia:** el Mercado como gate y como cupo de flota (2/4/6 + política), el activo
> persistente con coste que no se desmantela, el `CARAVANA_COOLDOWN` de creación, la vuelta a `'disponible'`
> en el origen desandando la ruta (nunca hay teletransporte), y el reparto automático (`asignarCaravanasATrueque`)
> como sustituto de Fase 0 de la carga manual. El revamp añade piezas y una vía manual; no tira nada de eso.

### 3.13.1 Tres partes

Una caravana `comercial` deja de tener capacidad y velocidad propias: las **deriva** de lo que se le monta.

| Parte | Qué aporta | Modelo |
|---|---|---|
| **Carros** | Capacidad de carga; y cuanto más carros, más tarda en prepararse | Lista de carros; cada carro lleva **como mucho un animal** |
| **Animales de carga** | Qué carros pueden moverse y a qué ritmo | Un animal por carro; sin animal, el carro no sale |
| **Escolta** | Defensa propia sin ningún jugador acompañando | Escuadrones cedidos por un jugador, **por viaje** (3.13.4) |

- **Solo viajan los carros con animal.** Un carro sin animal se queda `'disponible'` en el origen — no es lastre en ruta, simplemente no sale en ese envío.
- **Capacidad de viaje** = suma de (`capacidadBase` del carro × `factorCarga` del animal) sobre los carros con animal.
- **Velocidad** = la del animal **más lento** de la caravana. Los carros no capean velocidad todavía (todos los tipos actuales manejan igual); "tan rápida como su carro más lento" queda anotado para cuando el catálogo de carros crezca.

### 3.13.2 Carros y animales

> **Bloque "economía del oro" (2026-09-08 — `Consideraciones/Economia_Del_Oro_Definicion.md`, calibración en
> curso):** el buey pasa de 30 madera a ~12 oro. Se probó reintroducir la caravana #0 gratis y se descartó por
> medición. Ver los dos últimos párrafos de esta sección.

La caravana comercial nace como un **casco vacío y gratis** (`crearCaravana`) — cuenta contra el cupo del
Mercado y arranca el cooldown de creación, pero no puede viajar hasta que se le montan piezas. Todo el coste
está en las piezas, que se construyen y compran **sobre una caravana concreta** (el pool no vive suelto).

**Carros:**

| Carro | Dónde se fabrica | Coste | Rol |
|---|---|---|---|
| Básico | Mercado | 20 madera | `capacidadBase` ancla — un carro básico + un buey reproduce los 500/16 de hoy |
| Reforzado | Carpintería | 40 madera | Solo **más `capacidadBase`** (800). El catálogo se ampliará más adelante (resistencia a captura, penalización de velocidad) |

**Animales** — se compran sobre una caravana y se enganchan a un carro sin tracción:

| Animal | `factorCarga` | Velocidad | Coste | Nota |
|---|---|---|---|---|
| Buey | 1.0 | 16 | **~12 oro** (era 30 madera) | El ancla. Pasa a oro con el bloque "economía del oro" (ver más abajo) |
| Caballo | 0.5 | 24 | 60 oro | A esta velocidad **escapa de casi toda intercepción** (3.10 §"emboscada, no persecución") |
| Camello | 0.75 | ~19 | 40 oro | Opción intermedia. La **inmunidad al desierto queda diferida** (3.13.7): no existe bioma árido de primera clase |

**El buey se paga en ORO (~12), no en madera** (bloque "economía del oro", 2026-09-08 — diseño cerrado,
calibración en curso; `Consideraciones/Economia_Del_Oro_Definicion.md`). El texto anterior lo pagaba en madera
para esquivar el deadlock "sin caravana no hay comercio, sin comercio no hay oro, sin oro no hay caravana" —
razonamiento que dependía de que el oro solo entrara por mina o comercio. El bloque lo corta porque **la
fundación ya entrega 100 oro** (`FUNDACION.materialesIniciales`) y la recaudación de oro por población (3.1 /
Doc 4.1) lo repone aunque no haya mina: un asentamiento nuevo se paga su primera caravana (20 madera + 12 oro)
con lo de fundar. Buey barato (~12) para que sea una decisión de *cuántas* caravanas montar, no un muro. Cifras
placeholder a calibrar. La **cría** de animales queda diferida (3.13.7): por ahora solo compra. El livestock
del Corral (Doc 1.4) es un recurso distinto.

**El Mercado NO regala ninguna caravana** (sin cambios respecto a la decisión original de 3.13.2). El bloque
"economía del oro" probó reintroducir una caravana #0 gratis como on-ramp del buey en oro, la midió en batch y
la quitó: era también un amplificador de la fuente (todo asentamiento comerciando desde el tick 1 → más
comisiones → más oro), y con los 100 oro de fundación cubriendo el arranque, no compensaba.

### 3.13.3 Preparación y lanzamiento manual — ✅ implementado

Además del reparto automático (3.13.5), un residente del origen **lanza una caravana a mano**
(`prepararCaravana`): elige **carga** —del almacén del origen, hasta la capacidad de la caravana— y
**destino**. La caravana pasa por un estado `'preparando'` en el origen durante
`prepTicks = CARAVANA_PREPARACION.kPorCarro × (nº carros − 1)` — una caravana de 1 carro sale al instante
(`prepTicks = 0`), las grandes tardan. La carga se **reserva del almacén ya** (se descuenta al preparar).

- **`cancelarCaravana`** mientras siga `'preparando'` la devuelve a `'disponible'` y **reingresa la carga
  entera** al almacén — igual que quitar una obra `'en_cola'` (Doc 4.2).
- Al vencer `preparaHasta`, el tick la pasa a `'en_transito'` sobre la ruta ya calculada (si al preparar no
  había ruta por tierra, el comando se rechaza — el agua es infranqueable, 3.10).
- Una caravana `'preparando'` **no es interceptable**: está en su ciudad, no en el camino.
- Al llegar a destino, **vuelca la carga en su almacén y paga la comisión de comercio** (3.5), como cualquier
  entrega; luego hace el viaje de vuelta (`'retornando'`). En este pase el lanzamiento manual **no se
  vincula a un trueque concreto** — es logística de recursos (mover mercancía a plaza propia o aliada); el
  reparto automático sigue cubriendo los trueques.

Ciclo completo: `disponible → preparando → en_transito → retornando → disponible`.

### 3.13.4 Escolta sin héroe — ✅ implementado

La tercera pata, la que no existía. Es distinta de la escolta por ejército (5.13.3), que exige a un jugador
marchando con la caravana.

- Un jugador **residente del asentamiento de origen** cede escuadrones de su guarnición a la caravana **al
  lanzarla** (`prepararCaravana` con `escoltaEscuadronIds`). No necesita estar físicamente presente ni
  acompañar el viaje; lo que viaja son sus escuadrones (`Caravana.escolta`).
- Los recupera **cuando la caravana vuelve** — cesión **por viaje**, no enganche permanente. Al volver se
  funden con su escuadrón de la guarnición si ya reclutó más de esa tropa mientras tanto.
- Mientras están cedidos: **salen de la guarnición** del asentamiento, así que no lo defienden (Doc 5.12.4) y
  **cuentan contra el Liderazgo del jugador** (Doc 5.11) — sumados a lo que ya tenga en otras escoltas. Ceder
  tropa a una escolta no libera Liderazgo, es coste de oportunidad puro. *(Gap conocido: el tope no cruza con
  lo que ese jugador lleve a la vez en un ejército — se afina cuando pique.)*
- **No consumen ración.** Una escolta no es una campaña; se abstrae el suministro (a diferencia de 5.13), y
  también el viaje de vuelta si la caravana cae: los supervivientes reaparecen en la guarnición del origen.
- **Cupo por nivel interno de Mercado**: 1 / 2 / 3 escuadrones por caravana (`CARAVANA_ESCOLTA`, placeholder).
- **Combate**: se resuelve contra el `poderTotal` de los escuadrones-escolta —con bono de cohesión— en vez de
  la defensa base fija, tanto contra un ejército interceptor (3.10) como contra bandidos (Doc 1.9). La escolta
  sufre bajas en los dos casos (leves si aguanta, fuertes si cae) y vuelve con el debuff de derrota
  (`heridoHasta`). Si la caravana es capturada se pierde con **carga y carros** (3.13.6), no con la tropa.

### 3.13.5 Reparto automático vs preparación manual

- `asignarCaravanasATrueque` (3.2) sigue vivo para trueque / NPC / laboratorio, usando las caravanas **no
  reservadas**.
- Una caravana con `reservadaManual = true` sale del pool automático **sea cual sea su tamaño**. Una de 1
  carro se puede reservar; una de 5 puede seguir en automático. La señal es explícita, no se infiere de la
  composición.
- El **NPC no compone** caravanas multi-carro ni asigna escolta en este pase — es afordancia de jugador. El
  batch queda protegido.
- La caravana es **persistente y se reconfigura**: entre viajes el jugador le añade carros y animales, y
  **mueve carros entre dos caravanas suyas** (`moverCarroCaravana`) — libre, sin coste ni tiempo, mientras
  ambas estén `'disponible'` en el mismo asentamiento. Es mantenimiento de flota, no una mecánica. (Quitar un
  carro suelto, sin destino, no existe todavía — se mueve a otra caravana.)
- La **planificación horaria** (dejar caravanas listas para cierta hora de mundo) queda diferida (3.13.7).

### 3.13.6 Captura — ✅ implementado

Si el atacante gana: **la caravana se elimina** (carros, animales y carga perdidos; la carga capturada va al
carro del atacante, como en 3.10). El dueño reconstruye contra su cupo con el `CARAVANA_COOLDOWN` de siempre.

La **escolta sin héroe no se pierde con el carro**: los supervivientes (permadeath de las bajas, Doc 5.8)
reaparecen en la guarnición del origen con el debuff de derrota. El viaje de vuelta se abstrae, igual que la
ración de la escolta — no marchan por el mapa como un ejército.

### 3.13.7 Diferido — se diseñó la forma, se implementa después

Planificación horaria, cría de animales, visibilidad por tamaño, inmunidad del camello al desierto, catálogo
ampliado de carros y unificación con `Ejercito.suministro` (5.13). **Cada uno con su forma y lo que le falta
en `Docs/Mecanicas a desarrollar.md` §8.1.**
