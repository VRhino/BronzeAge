# 3. Sistema Económico y de Comercio

> Auditado contra el código real (trade.ts, market.ts, combate.ts, constants.ts). Cada sección indica su estado: ✅ implementado tal cual el diseño, 🔷 implementado con simplificación intencional de Fase 0, 🔶 implementado parcialmente, ❌ no implementado.

## 3.1 Oro — ✅ implementado
- NO es moneda acuñada: es METAL PRECIOSO EN BRUTO PESADO (sistema tipo siclo/shekel).
- ORIGEN: mina de oro en el mapa, mismo patrón que cantera/mina de cobre (nodo finito, con reemplazo automático al agotarse — ver Doc 4.2).
- USO: medio de pago en Mercado (3.3) y en comisiones (3.5). Compra de tecnología/mercenarios/sueldos: esos sistemas en sí (Aedas, mercenarios) no están implementados todavía en Fase 0, así que ese uso del oro no aplica aún en la práctica.
- ENTRE JUGADORES: recurso más de trueque, sin restricción especial.

## 3.2 Trueque de materiales (acuerdos entre Facciones) — 🔷 implementado con simplificación intencional
- Contrato MARCO abierto en el tiempo: `proponerTrueque` crea el acuerdo (cantidad total pactada por lado, plazo por defecto 200 ticks).
- **SIMPLIFICACIÓN DE FASE 0 (intencional, confirmada)**: el diseño objetivo dice que un jugador debe TOMAR una caravana y MOVERLA físicamente. En Fase 0, `despacharTrueques` genera la caravana AUTOMÁTICAMENTE en cuanto hay stock disponible y cupo pendiente en el acuerdo — sin acción manual del jugador. **Plan: pasar a movimiento manual en Fase 1+**, tal como está descrito el diseño objetivo.
- La caravana sí viaja de verdad por el mapa (progreso según distancia/velocidad) y entrega proporcionalmente al llegar — solo el disparo inicial es automático, no el viaje en sí.
- Cumplir o incumplir un trueque ajusta el score de reputación de Facción (Doc 2.7) — esto sí está conectado.

## 3.3 Órdenes de mercado (comercio abierto) — 🔷 implementado con simplificación intencional
- Un asentamiento coloca órdenes de compra/venta; cualquier jugador puede dejar lo pedido o comprar lo ofrecido.
- Se pagan con ORO.
- **SIMPLIFICACIÓN DE FASE 0 (intencional, documentada en el propio código)**: las órdenes se EMPAREJAN Y LIQUIDAN AL INSTANTE entre cualquier par de asentamientos — no hay transporte/caravana modelado en absoluto para este flujo (a diferencia del trueque, que sí simula el viaje). **Plan: pasar a requerir transporte físico en Fase 1+**, coherente con el resto del sistema de comercio.

## 3.4 Precios dinámicos — ✅ implementado
- Precio de referencia por defecto: precio base escalado por escasez/abundancia GLOBAL (stock objetivo de referencia = 500, con clamp entre ×0.4 y ×3).
- El jugador puede sobreescribir el precio manualmente al colocar una orden.
- NO existe mercado NPC de respaldo — correcto según diseño.

## 3.5 Comisiones de comercio — ✅ implementado (parcial, como marca el propio diseño)
- Comisión del 3% dentro de la misma Facción vs. 8% externa, aplicada tanto en trueque como en mercado, modulable por política ("Aranceles/Comercio Abierto") y por reputación de Facción.
- PENDIENTE (sin cambios): en qué se usa la riqueza acumulada; nivel intermedio de comisión para Facciones aliadas/vasallas de la misma Liga.

## 3.6 Categorías de caravana (heredado de Iberia) — 🔶 parcial: catálogo existe, solo 1 de 4 se usa
1. **Comercial**: ✅ implementada y en uso — la única categoría que el motor instancia realmente.
2. **Militar**: catálogo definido (capacidad/velocidad propias) pero el motor NUNCA la dispara ni le da comportamiento distinto.
3. **De construcción**: igual — solo datos, sin uso real.
4. **De contrabando**: igual — solo datos, sin uso real.
- PENDIENTE: conectar las 3 categorías restantes a sus disparadores correspondientes (equipo militar antes de asedio, materiales de fundación/ascenso, mecánica de detección reducida).

## 3.7 Transporte individual espontáneo (heredado de Iberia) — ❌ no implementado
No existe inventario personal de jugador ni transporte sin pasar por Mercado/acuerdo. Sigue siendo diseño puro, sin código.

## 3.8 Bonificación por distancia (heredado de Iberia) — ✅ implementado (parcial, solo trueque)
- Aplica como bonus a la comisión de trueque: hasta ×1.5 a partir de 600 unidades de distancia recorrida.
- NO aplica a órdenes de mercado, que se liquidan al instante sin viaje (ver 3.3) — coherente con que ese flujo no simula transporte todavía.

## 3.9 Dependencia logística real (heredado de Iberia) — 🔶 implícita, no es mecánica dedicada
No hay un sistema que detecte explícitamente "cortar una ruta" como evento de guerra económica. En la práctica, si no llegan caravanas, el receptor simplemente no recibe el recurso — es una consecuencia natural del modelo de trueque, no una mecánica de intercepción/guerra económica dedicada todavía (para eso hace falta que el combate de caravanas —3.10— interactúe activamente con acuerdos en curso, lo cual no está conectado aún).

## 3.10 Combate de caravanas — ✅ implementado (simplificado)
- Requiere el cargo de General. Calcula poder del atacante contra una defensa base de caravana FIJA (placeholder) — la escolta de jugadores reales (hasta 15, ver diseño original) NO está modelada todavía, es un valor fijo sustituto.
- Captura exactamente el 50% de la carga si el atacante gana (umbral de captura, coincide con el diseño).
- NO hay reparación NPC tras derrota — la caravana simplemente se elimina si es capturada (el diseño original preveía reparación; simplificado en Fase 0).

## 3.11 Comercio marítimo — ❌ fuera de alcance (correcto, según diseño)
Requiere tecnología de barcos + puertos. Nada implementado — consistente con que Fase 0 es 100% terrestre (eje naval pospuesto a fase completa).
