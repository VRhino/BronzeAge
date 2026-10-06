# Anexión y fusión con aceptación, y desarme del señor — decisiones y plan

> **Estado (2026-10-06): diseño cerrado con el usuario e IMPLEMENTADO en el motor, la sesión, la proyección al jugador y los bots; sin push.**
> Las **reglas de juego** viven en el canon: `Docs/Game/2` §2.4 (ruptura 4, el desarme) y §2.6 (la anexión y, desde el mismo día, la fusión: ver §5). Este documento guarda **las
> decisiones con sus alternativas descartadas**, la forma en el motor y lo que queda fuera. Enunciado original: `Docs/Mecanicas a desarrollar.md`
> §32 (cerrada y retirada).

## 1. De dónde viene

Dos huecos entre el canon y el código. El canon llamaba «voluntaria» a la anexión, pero el comando `anexionar` solo pedía autoridad en la
Facción **absorbente**: B no consentía nada, y tampoco había propuesta ni respuesta (`proponerRelacion` crea el vasallaje y la alianza activos
al instante; solo la paz es mutua). Y la ruptura 4 del vasallaje —la liberación cuando el señor «se desarma»— no tenía una línea de código.

## 2. Decisiones cerradas con el usuario (2026-10-06)

| # | Decisión | Alternativas descartadas |
|---|---|---|
| A1 | **Propone el Rey o el Embajador de A; acepta solo el Rey de B** | Propone solo el Rey de A (una anexión sería acto de Rey en ambos lados, pero el Embajador ya declara guerras y alianzas); cualquiera de las dos propone y la otra acepta (B podría ofrecerse: dos direcciones, más comandos y más estado) |
| A2 | **Las relaciones de B se cancelan sin penalización; sus vasallos pasan a A con el mismo tributo; B vasalla de un tercero bloquea** (su señor la libera antes) | Cancelarlas todas y liberar a los vasallos (A no hereda lo que B tenía); heredarlas todas (deja a A en guerra o aliada con quien ya tenía relación y hay que resolver los choques) |
| A3 | **Caduca a los 3 días de mundo, una pendiente por par, sin condición de poder ni de distancia** | 7 días (más tiempo para que el Rey vuelva a conectarse); no caduca (estado vivo y ruido en la proyección) |
| A4 | **El desarme no cambia la reputación de nadie** | Bonus a quien derrotó al señor (premia conquistar señores y puede incentivar farmeo); bonus a los vasallos liberados (la reputación mide lo que hace la Facción, y aquí no hacen nada) |

## 3. Decisiones de implementación (las tomé yo; se pueden revisar)

- **«Destruido» y «desarmado» son el mismo estado.** Una Facción que pierde su última plaza no desaparece (`registrarDerrota` solo anota
  `derrotadaPor`, Doc 5.15.5), así que una sola regla —señor con cero asentamientos— cubre las dos vías del canon.
- **Un barrido por tick, no un gancho en cada caída.** La conquista por asedio, por batalla de Unity y por ruina llegan por cuatro sitios
  distintos (`ejercitos.ts`, `resultadoBatalla.ts`, `militar.ts`, `simulation.ts`); engancharse a todos obligaba a pasar `relaciones` por cada
  uno. `liberarVasallosDeSenoresDesarmados` (`engine/diplomacia.ts`) corre en `simulation.ts` justo antes de cobrar el tributo, y la liberación
  llega en el tick siguiente al hecho. Coste: un tick de retraso, sin consecuencias (el tributo no se cobra sin plaza que lo reciba).
- **Estado aparte, no un campo de `Faccion`**: `GameSessionState.propuestasAnexion?: PropuestaAnexion[]` (opcional, sin migración). Las caducadas
  se quedan en la lista hasta que se escribe otra propuesta y ya no cuentan (`propuestasVigentes`), como `batallasActivas`.
- **`anexionar` desaparece como comando**: lo sustituyen `proponerAnexion`, `responderAnexion` y `retirarAnexion`. Ningún bot lo usaba; el contrato
  `src/contratos/v1/` no lo listaba. Quien lo llamara desde un cliente ha de cambiar de comando (no he comprobado si `BronzeAgeClient` lo usa).
- **Todo lo que lleva el id de una Facción se reasigna** (`engine/anexion.ts`): el `anexionar` anterior solo movía asentamientos y ciudadanos, y
  dejaba ejércitos, caravanas, rutas, Aedas residentes, miradas y memoria apuntando a una Facción que ya no existía. La memoria de niebla se funde
  (la exploración se une; las fichas e informes de plazas ajenas se unen y A gana en un choque; se descartan las de plazas que ahora son de A).
- **Se vuelve a validar al aceptar**: B pudo hacerse vasalla de un tercero mientras esperaba. Y no se acepta con una batalla abierta de cualquiera de
  las dos (un ejército en la batalla cambiaría de bando a mitad).
- **El Rey bot rechaza** las propuestas que recibe en su turno (`bots/cerebro/gobierno.ts`): un bot no entrega su Facción, y quien propone no espera
  a que caduquen. No hay facciones NPC en el motor (D53), así que no hubo que vetarlas.
- **Los vasallos de B con una relación ya activa con A quedan libres** en vez de heredados, para no dejar dos relaciones activas entre el mismo par.

## 4. Fuera de esta pasada

- **La fusión** se hizo después, ese mismo día: §5.
- **Cliente de jugador**: `ProyeccionJugador.propuestasAnexion` ya viaja; las pantallas de proponer, aceptar, rechazar y retirar son de
  `BronzeAgeClient` (aviso a Coordinación: `Docs/Coordinacion/01_Modelo_de_datos_compartido.md` §25).
- **Calibración**: `ANEXION.caducidadDias` entra en `Docs/Mecanicas a balancear.md`.

## 5. Fusión con aceptación (2026-10-06)

Mismo hueco que tenía la anexión (cualquiera de las dos Facciones la ejecutaba, sin consentimiento de la otra, y solo movía asentamientos y ciudadanos).
Reglas de juego en `Docs/Game/2` §2.6, opción 2; contrato en `Docs/Coordinacion/01_Modelo_de_datos_compartido.md` §26.

### 5.1 Decisiones cerradas con el usuario

| # | Decisión | Alternativas descartadas |
|---|---|---|
| F1 | **Propone solo el Rey de A; acepta el Rey de B** | Rey o Embajador de A como en la anexión (un Embajador podría proponer disolver una Facción cuyo Rey no ha dicho nada; habría que añadir una confirmación del Rey); que cada Rey acepte su lado sobre un borrador abierto por cualquiera (más estado y comandos) |
| F2 | **La propuesta fija el nombre y el Rey de C (uno de los dos Reyes); aceptar es votar** | Votación de ciudadanos tras el sí de ambos (estado nuevo, plazo, qué pasa si nadie vota; la fusión quedaría días en el limbo); Rey por regla automática, p. ej. el de más asentamientos o XP (desplaza a un Rey sin que lo decida, aunque haya aceptado) |
| F3 | **Relaciones como la anexión, en los dos lados**: alianzas y guerras de A y B se cancelan sin penalización, sus vasallos pasan a C con el mismo tributo, vasalla de un tercero bloquea | Heredar todo y resolver choques (alianza de una y guerra de la otra con un tercero: reglas de conflicto, y C arrastrada a guerras que no decidió); cancelarlo todo, vasallos incluidos (el señor que se fusiona regala su vasallaje) |
| F4 | **C hereda la experiencia mayor y la reputación mayor, y la unión de tecnologías** | Nueva de cero salvo tecnología (el nivel de Facción fija el cupo de asentamientos de nivel 2-3: fusionar dos Facciones fuertes las dejaba sin cupo); suma de XP y reputación media (la reputación de la peor sí cuenta) |

### 5.2 Decisiones de implementación (las tomé yo; se pueden revisar)

- **Una sola ruta de traslado.** El cuerpo de `anexionar` pasó a `engine/trasladoDeFaccion.ts` (`trasladarFaccion(mundo, origen, destino)`). La
  anexión lo llama una vez (B → A); la fusión crea C y lo llama dos veces (A → C, B → C). Así las alianzas y guerras de A **y** de B se cancelan
  (las reglas son las del origen) y el vasallaje entre ellas termina solo en la segunda pasada. `senorAjeno` y `propuestasVigentes`, que eran de la
  anexión, viven ahí también.
- **El Rey y el nombre se validan al proponer y al aceptar.** El Rey de A ha de ser quien la propuso (`PropuestaFusion.propuestaPor`; si cambia
  en los 3 días, el consentimiento ya no vale) y `nuevoReyId` ha de seguir siendo Rey de una de las dos. El motor no mira el actor: que proponga el Rey
  de A lo exige la matriz de autorización (`autorizacion.ts`), como en el resto de comandos.
- **Sigilo de A, sin reabrir el canon.** Doc 2.8.1 ya decía que la fusión hereda el de A; el de B se pierde y no choca con nadie (A y B desaparecen).
  Se queda así (el sigilo es identidad para siempre, y el de A es el de quien propone).
- **Experiencia y reputación**: el máximo de cada una por separado (leí «la mejor» así; si se quería la de una sola Facción, es un cambio de una línea en
  `engine/fusion.ts`). El nivel se recalcula de la experiencia.
- **Capital**: la marca `capitalDeFaccionId` de la plaza que el Rey de C había designado pasa a C (sin cooldown: C es nueva) y la de la otra se borra. Sin
  designación, la regla de siempre (la plaza viva más antigua).
- **Tecnología**: se une antes de trasladar (`aparecidas`, `adoptadas`, `reveladas` que aún no han aparecido); el traslado borra las entradas de A y B.
  `primeros` y las épicas cumplidas de los Aedas se reescriben igual que en la anexión.
- **Anexión y fusión pendientes a la vez** entre el mismo par pueden coexistir: la que se acepte primero borra la otra (el traslado limpia las dos listas).
- **`fusionar` desaparece como comando**: lo sustituyen `proponerFusion`, `responderFusion` y `retirarFusion`. Ningún bot lo usaba. `cliente/src/app/gameStore.ts`
  (cliente local) cambió sus métodos; no he comprobado si `BronzeAgeClient` llama a `fusionar`.
- **El Rey bot rechaza** las fusiones que recibe, igual que las anexiones (`bots/cerebro/gobierno.ts`).
- **La comprobación de batalla abierta** de ambos comandos es una (`faccionesConBatallaAbierta`, `session/batallas.ts`).
- **Caducidad**: `FUSION.caducidadDias` = 3, constante propia (placeholder, `Docs/Mecanicas a balancear.md` §32b).

### 5.3 Fuera de esta pasada

- **Cliente de jugador**: `ProyeccionJugador.propuestasFusion` ya viaja; las pantallas de proponer (con nombre y Rey de C), aceptar, rechazar y retirar son de
  `BronzeAgeClient`.
