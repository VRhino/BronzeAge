# 3. Sistema Económico y de Comercio

> Auditado contra el código real (trade.ts, market.ts, combate.ts, constants.ts). Cada sección indica su estado: ✅ implementado tal cual el diseño, 🔷 implementado con simplificación intencional de Fase 0, 🔶 implementado parcialmente, ❌ no implementado.

## 3.1 Oro — ✅ implementado
- NO es moneda acuñada: es METAL PRECIOSO EN BRUTO PESADO (sistema tipo siclo/shekel).
- ORIGEN: mina de oro en el mapa, mismo patrón que cantera/mina de cobre (nodo finito, con reemplazo automático al agotarse — ver Doc 4.2).
- USO: medio de pago en Mercado (3.3) y en comisiones (3.5). Compra de tecnología/mercenarios/sueldos: esos sistemas en sí (Aedas, mercenarios) no están implementados todavía en Fase 0, así que ese uso del oro no aplica aún en la práctica.
- ENTRE JUGADORES: recurso más de trueque, sin restricción especial.

## 3.2 Trueque de materiales (acuerdos entre Facciones) — 🔷 implementado con simplificación intencional
- Contrato MARCO abierto en el tiempo: `proponerTrueque` crea el acuerdo (cantidad total pactada por lado, plazo por defecto 200 ticks).
- **SIMPLIFICACIÓN DE FASE 0, revisada (ampliación de comercio, a petición del usuario)**: el diseño objetivo dice que un jugador debe TOMAR una caravana y MOVERLA físicamente — Fase 0 ya no genera caravanas de la nada como antes. Ahora exige tener una caravana PROPIA construida y **'disponible'** en el asentamiento origen (ver 3.12); `asignarCaravanasATrueque` la asigna automáticamente al envío pendiente, sustituyendo por ahora la elección manual del jugador ("solo simulación": en el diseño objetivo el jugador elige la caravana, la carga y la escolta a mano). Si no hay ninguna disponible, el envío simplemente espera al siguiente tick. **Plan: pasar a asignación/carga manual en Fase 1+**, tal como está descrito el diseño objetivo.
- Cuando hay menos caravanas disponibles que envíos pendientes en un mismo asentamiento (varios acuerdos compitiendo a la vez), se prioriza por un SCORE ponderado (ver 3.12: urgencia por expiración del acuerdo, urgencia por volumen pendiente, cercanía del destino) — no por orden de llegada.
- La caravana sí viaja de verdad por el mapa: desde Fase 0.3, sobre una RUTA calculada por pathfinding (rodea terreno costoso, ver Doc 1.6/3.6) en vez de una línea recta, con velocidad ×2 respecto a la original (ver 3.6) modulada por el coste del terreno que cruza en cada momento — y entrega proporcionalmente al llegar. Al entregar, la caravana propia vuelve a estar 'disponible' en el origen — no desaparece, es un activo persistente y con costo (3.12), no un objeto de un solo uso.
- Cumplir o incumplir un trueque ajusta el score de reputación de Facción (Doc 2.7) — esto sí está conectado.

## 3.3 Órdenes de mercado (comercio abierto) — 🔷 implementado con simplificación intencional
- Un asentamiento coloca órdenes de compra/venta; cualquier jugador puede dejar lo pedido o comprar lo ofrecido.
- Se pagan con ORO.
- **Gate nuevo (ampliación de comercio, a petición del usuario)**: colocar una orden ahora exige tener un Mercado activo (ver 3.12) — antes cualquier asentamiento podía hacerlo desde el tick 0, sin edificio.
- **SIMPLIFICACIÓN DE FASE 0 que sigue en pie (documentada en el propio código)**: las órdenes se EMPAREJAN Y LIQUIDAN AL INSTANTE entre cualquier par de asentamientos — no hay transporte/caravana modelado para este flujo (a diferencia del trueque, que sí usa la flota de caravanas, ver 3.2/3.12). **Plan: pasar a requerir transporte físico en Fase 1+**, coherente con el resto del sistema de comercio.

## 3.4 Precios dinámicos — ✅ implementado
- Precio de referencia por defecto: precio base escalado por escasez/abundancia GLOBAL (stock objetivo de referencia = 500, con clamp entre ×0.4 y ×3).
- El jugador puede sobreescribir el precio manualmente al colocar una orden.
- NO existe mercado NPC de respaldo — correcto según diseño.

## 3.5 Comisiones de comercio — ✅ implementado (parcial, como marca el propio diseño)
- Comisión del 3% dentro de la misma Facción vs. 8% externa, aplicada tanto en trueque como en mercado, modulable por política ("Aranceles/Comercio Abierto") y por reputación de Facción.
- PENDIENTE (sin cambios): en qué se usa la riqueza acumulada; nivel intermedio de comisión para Facciones aliadas/vasallas de la misma Liga.

## 3.6 Categorías de caravana (heredado de Iberia) — 🔶 parcial: catálogo existe, solo 1-2 de 4 se usan
1. **Comercial**: ✅ implementada y en uso — única categoría que el motor instancia realmente para Trueque/Mercado, y la única que ahora es un activo PROPIO y persistente (`costoConstruccion`, ver 3.12), en vez de efímera.
2. **Militar**: catálogo definido (capacidad/velocidad propias) pero el motor NUNCA la dispara ni le da comportamiento distinto.
3. **De construcción**: sí tiene uso real (Caravana de Fundación, Doc 1.8), pero es un mecanismo aparte del de Trueque/Mercado — no forma parte de la flota propia de 3.12.
4. **De contrabando**: solo datos, sin uso real.
- **Velocidad ×2 en las 4 categorías (ampliación de comercio, a petición del usuario)**: el diagnóstico mostró que a la velocidad original un trueque de tamaño moderado a distancia media podía necesitar más ticks de viaje (varios envíos en serie, ver 3.2) que el plazo por defecto del acuerdo (200 ticks) — expiraba antes de poder completarse pase lo que pase. Se dobló la velocidad de las 4 categorías a la vez para no dejar el catálogo inconsistente entre sí, aunque solo Comercial (y Construcción, Doc 1.8) tienen uso real hoy.
- **Movimiento con coste de terreno (Fase 0.3, ver Doc 1.1/1.5/1.6)**: la "velocidad" del catálogo sigue siendo la base, pero el avance real por tick ahora se divide entre el coste de terreno en la posición actual (llano≈1, colina/montaña más lento, agua/cima muy caro) — cruzar relieve accidentado de verdad tarda más ticks. Cada caravana calcula su propia RUTA por pathfinding al lanzarse (rodea relieve costoso en vez de ir en línea recta), salvo que ya exista un Camino Comercial para ese par de asentamientos (Doc 1.6), en cuyo caso reusa su polilínea y recibe además el bonus de velocidad del camino. Caravanas de partidas guardadas antes de Fase 0.3 (sin ruta) siguen moviéndose en línea recta sin coste de terreno — compatibilidad hacia atrás sin migración. Ver `engine/movimiento.ts`, `world/rutas.ts`.
- PENDIENTE: conectar Militar/Contrabando a sus disparadores correspondientes (equipo militar antes de asedio, mecánica de detección reducida).

## 3.7 Transporte individual espontáneo (heredado de Iberia) — ❌ no implementado
No existe inventario personal de jugador ni transporte sin pasar por Mercado/acuerdo. Sigue siendo diseño puro, sin código.

## 3.8 Bonificación por distancia (heredado de Iberia) — ✅ implementado (parcial, solo trueque)
- Aplica como bonus a la comisión de trueque: hasta ×1.5 a partir de 600 unidades de distancia recorrida.
- NO aplica a órdenes de mercado, que se liquidan al instante sin viaje (ver 3.3) — coherente con que ese flujo no simula transporte todavía.

## 3.9 Dependencia logística real (heredado de Iberia) — 🔶 implícita, no es mecánica dedicada
No hay un sistema que detecte explícitamente "cortar una ruta" como evento de guerra económica. En la práctica, si no llegan caravanas, el receptor simplemente no recibe el recurso — es una consecuencia natural del modelo de trueque, no una mecánica de intercepción/guerra económica dedicada todavía (para eso hace falta que el combate de caravanas —3.10— interactúe activamente con acuerdos en curso, lo cual no está conectado aún).

## 3.10 Combate de caravanas — ✅ implementado (simplificado)
- **Ya NO es un comando** (2026-09-04, Paso 11 del movimiento de ejércitos): interceptar dejó de declararse desde un asentamiento y pasó a ser lo que le ocurre a un ejército que se cruza con una caravana enemiga en el mapa (Doc 5.12.3). No hace falta General porque no hace falta orden: hace falta tener una columna ahí, que es más caro y más interesante. El botín viaja en su carro y llega a casa al replegarse.
- Calcula poder del atacante contra una defensa base de caravana FIJA — que sigue siendo el valor para una caravana SIN escolta. Con escolta ya no aplica: se resuelve contra el poder real del ejército que la acompaña (5.13.3), que era justo lo que este apartado daba por no modelado.
- Captura exactamente el 50% de la carga si el atacante gana (umbral de captura, coincide con el diseño).
- NO hay reparación NPC tras derrota — la caravana simplemente se elimina si es capturada (el diseño original preveía reparación; simplificado en Fase 0).
- **Amenaza NPC nueva — IMPLEMENTADO (a petición del usuario, ver Doc 1.9)**: campamentos de bandidos en bosques no reclamados atacan caravanas que pasen cerca, con la misma resolución de combate asimétrico que la intercepción entre Facciones, pero contra un bando NPC — ver Doc 1.9 para spawn, recompensa y reaparición, `engine/bandidos.ts` para la implementación.

**ESCOLTA DE CARAVANAS — resuelta por la mecánica de ejércitos (2026-09-02, ver Doc 5.13.3).** La escolta que este apartado daba por no modelada ya tiene forma: una caravana puede ir **adjunta a un ejército** y hacer su entrega normal mientras marcha con él. Deja de defenderse con la defensa base fija y pasa a estar protegida por el poder de combate real del ejército. Se autoequilibra con la regla de velocidad (Doc 5.12.5): el ejército va al ritmo de su integrante más lento, así que escoltar una caravana comercial (16) frena a un ejército ligero (20) y le quita la capacidad de cazar otras caravanas — **no se puede escoltar y depredar a la vez**. Si el ejército es derrotado, la caravana escoltada se pierde con él.

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
