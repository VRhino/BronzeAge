# Anexión con aceptación y desarme del señor — decisiones y plan

> **Estado (2026-10-06): diseño cerrado con el usuario e IMPLEMENTADO en el motor, la sesión, la proyección al jugador y los bots; sin push.**
> Las **reglas de juego** viven en el canon: `Docs/Game/2` §2.4 (ruptura 4, el desarme) y §2.6 (la anexión). Este documento guarda **las
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

- **La fusión** (Doc 2.6, opción 2) tiene el mismo hueco de consentimiento (cualquiera de las dos Facciones la ejecuta) y no traslada ejércitos,
  caravanas, rutas ni memoria. Se retoma con la votación real de Rey. Queda como entrada en `Docs/Mecanicas a desarrollar.md`.
- **Cliente de jugador**: `ProyeccionJugador.propuestasAnexion` ya viaja; las pantallas de proponer, aceptar, rechazar y retirar son de
  `BronzeAgeClient` (aviso a Coordinación: `Docs/Coordinacion/01_Modelo_de_datos_compartido.md` §25).
- **Calibración**: `ANEXION.caducidadDias` entra en `Docs/Mecanicas a balancear.md`.
