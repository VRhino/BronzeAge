# El comercio deja de ser magia — decisiones y plan

> **Estado (2026-09-07): IMPLEMENTADO** (plan §5 completo, snapshot v10→v11). Cerró las **dos únicas
> simplificaciones de Fase 0 que quedaban vivas en el sistema económico**, y que el propio canon marcaba como
> deuda con un "Plan: …en Fase 1+" desde que se auditó (Doc 3.2 y 3.3). También cierra la entrada que fue §15
> de `Docs/Mecanicas a desarrollar.md` (comerciar con una plaza ajena desde su puerta) — ver §8. Quedan dos
> palancas menores, sin diseñar (§8).
>
> Reglas de juego → `Docs/Game/3` §3.2, §3.3, §3.7, §3.8. Aquí solo decisiones y plan.

## 0. El punto de partida

Las dos son deuda declarada, no descuidos. Están escritas en el código y en el canon, con su motivo.

**1. Las órdenes de mercado se teletransportan.** `avanzarMercado` empareja órdenes de CUALQUIER par de
plazas y liquida en el acto: descuenta piedra en A, la suma en B, mueve el oro al revés. Nada recorre el
mapa. El comentario de la función lo dice sin adornos:

> *"Simplificación de Fase 0: la transacción se liquida al instante (sin caravana física) — a diferencia del
> trueque, el diseño no exige transporte para estas órdenes."*

**2. El trueque se acepta solo.** `proponerTrueque` nace `'activo'`. El otro lado no dice nada porque no se
le pregunta:

> *"En Fase 0 se acepta al proponerse (el flujo real de 'el Tesorero de B acepta' requiere un jugador
> interactivo real, fuera de alcance del prototipo)."*

Ese motivo **ha caducado**. El jugador interactivo real existe desde el jugador situado: está en un sitio
concreto del mapa, entra y sale por la puerta, y tiene un carro. Lo que faltaba ya no falta.

## 1. Decisiones cerradas con el usuario (2026-09-07)

1. **Una orden de mercado es una oferta EN PIE EN UNA PLAZA, no una entrada en una bolsa global.** Se cumple
   en el mostrador: alguien tiene que estar allí, con la mercancía o con el oro encima.

2. **El emparejamiento automático plaza↔plaza desaparece.** Era la teletransportación entera. No se sustituye
   por "la plaza manda una caravana": eso ya es el trueque (3.2), y tener dos caminos para el mismo hecho
   —mover mercancía entre dos plazas— es lo que hay que evitar, no lo que hay que duplicar.

3. **El viaje completo es el del jugador**, y ya son cuatro actos, no uno: cargar el oro en tu plaza → llegar
   a la plaza que vende → comprar allí → volver y depositar. Igual en espejo cuando la plaza compra: cargas
   la mercancía, la entregas, cobras, y el oro vuelve en tu carro.

4. **El oro viaja como carga y PESA.** No hay transferencia, ni crédito, ni pagaré: es metal precioso pesado
   (Doc 3.1) y ocupa carro como cualquier otra cosa. Eso pone un techo natural a cuánto puede uno comprar de
   una tacada, que es exactamente la clase de límite que este juego quiere.

5. **El trueque pasa a propuesta → aceptación.** Nace `'propuesto'` y no obliga a nadie hasta que el otro
   lado dice que sí. Un trueque no contestado expira solo, con el plazo que ya tenía.

## 2. Lo que ya estaba hecho y no hay que inventar

Conviene decirlo porque cambia el tamaño del trabajo: **la mitad del viaje ya existe**.

- **El carro es un inventario personal de propósito general.** `Ejercito.suministro` es un
  `Record<string, number>` y `cargarCarroElegido` ya deja sacar del almacén lo que el jugador elija, con su
  tope de capacidad (`LOGISTICA.capacidadCarroPorJugador` × participantes, más las caravanas adjuntas). El
  canon aún dice en §3.7 que "no existe inventario personal de jugador" — eso quedó viejo con el jugador
  situado.
- **Volver a casa y depositar ya funciona.** Entrar en tu residencia disuelve la columna y vuelca el carro al
  almacén (`absorberColumna`), y es la MISMA función que usa un ejército que vuelve replegado.
- **Cargar y entregar a mano en una plaza ya tiene precedente**: `cargarCaravanaAdjunta` y
  `entregarDesdeCaravanaAdjunta` (Doc 5.13.3) hacen justo eso para el trueque escoltado.

Lo que falta es **el mostrador**: la operación de tomar una orden estando allí.

## 3. La tensión de verdad: qué le pasa al laboratorio

Quitar el clearing automático es quitar una fuente de movimiento económico en el batch, donde **no hay
jugadores que viajen**. Hay que decir con precisión qué se pierde, porque no es lo que parece:

- El comercio automático del laboratorio (`simulacionAutoComercio`) y el de las Facciones NPC
  (`npcGobernanza`) van **por trueque, con caravanas que sí recorren el mapa**. Eso no lo toca este cambio.
- Las **órdenes de mercado no tenían ningún emisor automático** hasta ayer, cuando las plazas NPC empezaron a
  publicarlas (`publicarOrdenesNpc`, `Entrada_Al_Mundo_Definicion.md` §3).

O sea: el clearing llevaba desde siempre **emparejando casi nada**, porque casi nadie colocaba órdenes. Se
quita un atajo que duplicaba a las caravanas, no un pilar de la economía. Y lo de ayer sigue en pie y de
hecho **mejora**: la plaza NPC publica, y ahora el jugador tiene que ir hasta allí a comprarle. Eso es
precisamente el "socio de comercio" de la decisión 5 de la entrada al mundo, y no un escaparate que se sirve
solo.

**Lo que sí aparece como problema nuevo:** sin clearing, una orden publicada que nadie toma **no se cierra
nunca**. Hoy nada la caduca. Con las plazas NPC publicando cada tick, en cien ticks cada plaza tendría sus
seis órdenes activas eternas y no volvería a ajustar precios jamás. Así que este cambio **obliga** a que las
órdenes expiren, igual que expira un trueque. No es un extra: es la consecuencia directa de quitar el
emparejamiento.

## 4. Quién acepta un trueque cuando no hay nadie

La aceptación explícita necesita alguien que la dé, y en dos de los tres casos no hay jugador:

- **Jugador ↔ jugador:** el flujo real. Lo acepta el cargo con competencia comercial del otro lado.
- **NPC:** decide `npcGobernanza`. Y esto **desbloquea algo**: hoy el NPC solo propone trueques a otros NPC,
  y el motivo escrito en el código es exactamente que `proponerTrueque` pactaba sin consentimiento. En cuanto
  hay aceptación, **el NPC puede proponerle a un jugador** sin comprometerle nada.
- **Laboratorio:** `simulacionAutoComercio` representa a las dos plazas, así que acepta por la receptora —
  pero **pasando por la misma función** que usaría un jugador, no saltándosela. Si el batch tuviera un camino
  propio, mediríamos una economía que no es la del juego.

## 5. Plan

1. ~~**Trueque: propuesta → aceptación.**~~ **HECHO (2026-09-07).** Estado `'propuesto'`, comandos de aceptar
   y rechazar, el camino comercial se traza al ACEPTAR y no al proponer, caducidad de la propuesta sin
   contestar sin penalizar a nadie, y los tres aceptantes de §4. Sin migración de snapshot: los estados viejos
   siguen siendo válidos y un acuerdo guardado como `'activo'` ya estaba pactado.
2. ~~**Órdenes con caducidad.**~~ **HECHO.** `MERCADO.plazoOrdenMinutos` = 200, el mismo plazo que un trueque:
   son la misma clase de compromiso y darles vidas distintas sería una diferencia sin justificar.
3. ~~**El mostrador.**~~ **HECHO.** `comerciarEnPlaza` en el motor, su comando, y fuera el emparejamiento
   automático del tick (queda solo `caducarOrdenes`). Snapshot v10 → v11.
4. ~~**Canon y medición.**~~ **HECHO.** Doc 3.2/3.3/3.5/3.7/3.8.

### Dos cosas que aparecieron implementando, y no estaban en el plan

- **La visibilidad del escaparate.** El mostrador nace inusable: la proyección solo enseñaba las órdenes de tu
  propia Facción, así que había que comprar a ciegas en una plaza ajena. Se abre a **quien está en su puerta**,
  y solo a lo que sigue en pie — la regla de un mercado de verdad. Una lista global habría dejado leer los
  precios del mundo entero sin moverse, que es justo lo que esta mecánica quita.
- **La comisión deja de acuñarse.** El emparejamiento viejo la creaba de la nada y se la regalaba al vendedor.
  En el mostrador la paga quien toma la orden y se la queda la plaza: el oro se conserva, y la comisión pasa a
  significar algo concreto — lo que cuesta usar el mercado de otro.

## 7. Medido

**El tick no se mueve**: 35,5 ms a 100 asentamientos con 100 vivos, frente a 35 antes de tocar nada, y el
escalado sigue en O(n^0,82). Quitar el emparejamiento **no abarata el tick porque casi nunca emparejaba**,
que era la predicción de §3.

**La economía tampoco.** Batch de 400 ticks y 40 Facciones, la misma semilla, contra el commit anterior a
todo esto (`4d13d73`):

| | antes | después |
|---|---|---|
| asentamientos vivos | 47 | 51 |
| oro medio | 155,7 | 155,2 |
| acuerdos activos | 16 | 24 |
| acuerdos cumplidos | 1 | 1 |
| pesants medios | 186,2 | 187,1 |
| ejércitos vivos | 21 | 26 |
| nutrición media | 100 | 100 |
| excepciones | 0 | 0 |

Lo que importa de esa tabla es **"acuerdos cumplidos: 1" en las dos columnas**. Era el control: si la
aceptación explícita hubiera roto la cadena del trueque NPC, ahí saldría un 0. Sale lo mismo — o sea que el
número bajo es una propiedad de una corrida de 400 ticks y no algo que este cambio haya causado.

Los acuerdos activos suben de 16 a 24. Lectura plausible, no demostrada: el NPC ahora **rechaza** lo que no
puede servir, y una petición sin contestar ya cuenta como ayuda pedida (`yaTieneAyudaEnCaminoPara`), así que
deja de amontonar pactos redundantes sobre un socio que iba a fallar. Con una sola semilla y 400 ticks no da
para más que eso.

**Lo que sigue sin medir**, y conviene no olvidarlo: el efecto sobre los **precios de referencia**. Las plazas
NPC publican órdenes que ahora **nadie del laboratorio toma** —no hay jugadores que viajen—, así que en batch
esas ofertas nacen y caducan. El precio de referencia se calcula sobre el stock global (Doc 3.4) y no sobre
las órdenes, así que no debería moverse por esto; pero es una predicción, no una medida.

## 8. Puntos abiertos — comercio del forastero desde la puerta

`comerciarEnPlaza` cubre el enunciado (un forastero en la puerta toma las órdenes activas de esa plaza, contra
su carro, con comisión externa). Esto cierra la entrada que fue §15 de `Docs/Mecanicas a desarrollar.md`.
Quedan dos palancas menores, sin diseñar:

- **¿El forastero puede publicar sus propias órdenes en un mercado ajeno, o solo tomar las publicadas?** Hoy
  `colocarOrdenMercado` exige residencia/ciudadanía, así que solo toma. Abrirlo pediría decidir el cupo, la
  comisión de publicar siendo de fuera, y qué pasa con la orden si el forastero se marcha.
- **¿Puede el Gobernador cerrar el mercado sin cerrar la puerta?** Es la palanca diplomática obvia —dejar
  pasar pero no vender, o vender solo a aliados—. Hoy `politicaDeAcceso` (jugador situado, Doc 1.10) controla
  la puerta pero no el mostrador. Sería una segunda política de acceso, o un modificador de la misma.
