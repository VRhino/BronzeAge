# La entrada al mundo — decisiones y plan

> **Estado (2026-09-07): diseñado a medias, en curso.** Nace de un hueco que el paso 9 del jugador situado
> destapó y que es más de fondo que ese paso: **no hay sitio donde existir antes de tener bandera**, y nada
> impide que 800 jugadores funden 800 asentamientos en el primer minuto.
>
> Reglas de juego → `Docs/Game/1` §1.3. Aquí solo decisiones y plan.

## 0. El punto de partida

Dos hechos medidos en el código, no supuestos:

**1. Un jugador sin Facción no puede existir.** `proyectarParaJugador` da ojos a quien tiene Facción o tiene
escuadrones dentro de una columna:

```ts
const ejercitosPropios = estado.ejercitos.filter(
  (e) => (faccionId !== null && e.faccionId === faccionId) || e.escuadrones.some((esc) => esc.jugadorId === jugadorId)
);
```

Un recién llegado no cumple ninguna de las dos. `seVeAhora(punto, [], [])` es siempre falso, así que no ve
ciudades, ni columnas, ni campamentos, ni tiene niebla que levantar — **ni se ve a sí mismo**, porque su
columna viaja en `ejercitos: ejercitosPropios`, que está vacío. Caminaría por un mapa negro.

**2. Nada frena la ola de fundaciones.** `crearFaccion: { rolesPermitidos: ['jugador'] }` — cero condiciones.
El cooldown de `CIUDADANIA.cooldownCreacionFaccionDias` solo aplica **tras abandonar** una Facción, así que
una cuenta nueva no tiene ninguno. El cap por nivel limita a 1 asentamiento **por Facción**, no cuántas
Facciones hay. Y la fundación grupal de 5 *permite* compartir pero no obliga a nada.

O sea: 800 jugadores → 800 Facciones → 800 asentamientos. El freno que la fundación grupal pretendía ser
nunca llegó a existir.

## 1. Decisiones cerradas con el usuario (2026-09-07)

1. **El vestíbulo es una CONDICIÓN, no un lugar** (opción A), **y además un sitio donde llegar** (opción B):
   se construye A ahora y B encima, sin rehacer nada. Se descartó el mapa de tutorial aparte (opción C): un
   segundo mundo con su propia simulación, y encima enseñando las mecánicas **donde no importan** — en un
   juego cuya tensión es que todo es persistente, eso juega en contra.

2. **Para fundar: ciudadanía previa + fundación grupal obligatoria.** Fundar una Facción nueva deja de ser el
   primer acto y pasa a ser un **cisma** — gente que ya vivía en algún sitio y se marcha. Se autolimita sin
   ninguna cuenta atrás, y encaja con la ficción mejor que un cronómetro.

3. **Pero AMBOS parametrizados, y en la primera prueba desactivados**: tamaño de grupo **1** y ciudadanía
   previa **opcional**. Con 5 testers el freno estorba; con 800 hace falta. Tiene que poder cambiarse sin
   tocar código, así que las dos son constantes y no `if`s repartidos.

4. **El servidor arranca con Facciones NPC ya asentadas**, para que el primero en llegar encuentre un mundo
   habitado y no un vacío. El motor ya sabe hacerlo: `npcGobernanza` gobierna Facciones enteras.

5. **Esas Facciones son DEFENSIVAS, y son socios de comercio.** No dan caza a los recién llegados; se
   defienden si las tocan, y ofrecen con qué comerciar. Un vecino, no un depredador.

## 2. El hallazgo que obliga a la decisión 5

**La conducta agresiva la introduje yo el 2026-09-06**, en el paso 8e del jugador situado, y por un motivo
bueno: sin ella el laboratorio se quedaba sin combates y las constantes militares se medían sobre un mundo en
paz. Pero `fijarPersecucionesNpc` da caza a **cualquier columna o caravana no aliada dentro de 150**:

```ts
.filter((o) => o.id !== cazador.id && enemiga(cazador.faccionId, o.faccionId) && !enTregua(o, instante))
.filter((o) => distancia(o.posicionActual, cazador.posicionActual) <= VISION.ejercito)
```

Un recién llegado no es aliado de nadie. Así que **hoy, una Facción NPC sembrada al arrancar cazaría a cada
novato que pasara a 150 de una de sus columnas**, y le interceptaría las caravanas de comercio. Exactamente
el mundo que la decisión 5 no quiere. Y `lanzarCampanas` manda columnas contra plazas rivales, que incluye
las de los jugadores.

**Y hay una tensión de verdad, no un descuido:** el batch NECESITA esa agresividad. Es lo que sostiene el
riesgo 5 del jugador situado —"el laboratorio se queda sin combates y nadie se entera"— y hay dos tests que
fallan si desaparece. Así que la postura no puede ser un borrado: tiene que ser un **ajuste**, con el
laboratorio en agresiva y el mundo sembrado en defensiva.

## 3. Qué le falta al NPC para ser un socio de comercio

Medido, y no es lo que parecía: **el NPC no puede comerciar con un jugador hoy**, y por una razón correcta.

- **Trueques:** solo los propone entre Facciones NPC, deliberadamente. El comentario del código lo dice:
  *"El SOCIO también tiene que ser NPC: `proponerTrueque` pacta sin pedir consentimiento al otro"*. Proponerle
  uno a un jugador le comprometería recursos sin preguntarle.
- **Órdenes de mercado:** el NPC **nunca coloca ninguna** (cero referencias a `colocarOrdenMercado`).

La vía limpia es la segunda: **que las plazas NPC publiquen órdenes de compra y venta**. El consentimiento
está por construcción —el jugador la toma o no la toma—, la maquinaria existe entera (`engine/market.ts`), y
convierte a la plaza NPC en el socio que la decisión 5 pide sin necesidad de ninguna IA de negociación.

## 4. Lo que NO cambia: el peligro sigue existiendo

Facciones defensivas no significan un mundo inofensivo, y conviene decirlo porque es la objeción obvia. El
peligro de base ya lo dan los **bandidos** (`engine/bandidos.ts`), que atacan caravanas por su cuenta y no
son de nadie. Queda un reparto limpio:

- **Bandidos:** la amenaza. Por eso hay que construir murallas y reclutar.
- **Facciones NPC:** los vecinos. Con quien comerciar, y de quien aprender.
- **Otros jugadores:** la guerra de verdad.

## 5. Abierto: qué HACE un huésped

Idea del usuario, y no es un adorno: **es el contenido del vestíbulo**. Sin ella la fase de huésped es
"espera a poder fundar", que es la peor versión de esto.

> Ganar puntos dentro de la Facción IA haciendo cosas para ella: escoltar sus caravanas, explorar, buscar
> cosas en el mapa. Acciones que sirven de tutorial y dan recompensa dentro de esa Facción.

Lo que ya existe y reutilizaría: la escolta de caravanas (`adjuntarCaravana`, Doc 5.13.3), la exploración
(`engine/exploracion.ts`), y la reputación de Facción (`REPUTACION`, `aplicarAjustesExperiencia`).

Lo que NO existe: **standing por jugador dentro de una Facción**. Hoy la reputación y la experiencia son de
la Facción entera, no de cada uno de sus miembros.

Es una mecánica propia y merece su ficha aparte, no una sección de esta. Se anota aquí para que no se pierda,
y porque decide si el vestíbulo es un sitio donde se juega o una sala de espera.

## 6. Plan

1. **La visión de quien no tiene bandera.** `ejercitosPropios` pasa a incluir las columnas donde el jugador
   es **participante**, no solo donde tiene tropas — el campo existe desde el paso 1 del jugador situado. Más
   `VISION.jugadorSolo`. Sin esto no hay vestíbulo posible, ni aquí ni en un mapa aparte.
2. **La puerta de fundación**, con sus dos constantes parametrizadas (decisión 3). Es la costura: lo que se
   decida después se enchufa ahí sin tocar el onboarding.
3. **Postura del NPC** (decisión 5), con el laboratorio en agresiva.
4. **Órdenes de mercado del NPC** (§3), que es lo que lo convierte en socio.
5. El resto del paso 9 del jugador situado: spawn aleatorio, memoria personal, fundar donde se está.
6. **Aparte, sin fecha:** qué hace un huésped (§5).
