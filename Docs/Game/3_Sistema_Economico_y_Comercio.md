# 3. Sistema Económico y de Comercio

> Auditado contra el código real (trade.ts, market.ts, combate.ts, constants.ts). Cada sección indica su estado: ✅ implementado tal cual el diseño, 🔷 implementado con simplificación intencional de Fase 0, 🔶 implementado parcialmente, ❌ no implementado.

## 3.1 Oro — ✅ implementado
- NO es moneda acuñada: es METAL PRECIOSO EN BRUTO PESADO (sistema tipo siclo/shekel).
- ORIGEN: mina de oro en el mapa, mismo patrón que cantera/mina de cobre (nodo finito, con reemplazo automático al agotarse — ver Doc 4.2).
- USO: medio de pago en Mercado (3.3) y en comisiones (3.5). Compra de tecnología/mercenarios/sueldos: esos sistemas en sí (Aedas, mercenarios) no están implementados todavía en Fase 0, así que ese uso del oro no aplica aún en la práctica.
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
1. **Comercial**: ✅ implementada y en uso — única categoría que el motor instancia realmente para Trueque/Mercado, y la única que ahora es un activo PROPIO y persistente (`costoConstruccion`, ver 3.12), en vez de efímera.
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
- Calcula poder del atacante contra una defensa base de caravana FIJA — que sigue siendo el valor para una caravana SIN escolta. Con escolta ya no aplica: se resuelve contra el poder real del ejército que la acompaña (5.13.3), que era justo lo que este apartado daba por no modelado.
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
