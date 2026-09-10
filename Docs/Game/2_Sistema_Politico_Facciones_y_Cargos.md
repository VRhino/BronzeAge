# 2. Sistema Político: Facciones, Cargos y Diplomacia

## 2.1 Glosario rápido (ver página "Glosario de Entidades Políticas" para el detalle completo)
- **Jugador**: pertenece a 1 y solo 1 Facción, y reside en 1 y solo 1 asentamiento (Doc 2.5, a petición del usuario — es lo que le permite tener como mucho un escuadrón de cada tropa, ver Doc 5.8). Sus escuadrones son SUYOS, no del asentamiento (Doc 5.4), y su Liderazgo limita cuántos puede sacar a campaña (Doc 5.11).
- **Asentamiento**: pertenece a 1 y solo 1 Facción.
- **Facción**: entidad política soberana, agrupa jugadores y asentamientos. Tiene cargos de nivel Facción (Rey, Embajador).
- **Liga**: red de Facciones conectadas entre sí (por vasallaje y/o alianza). NO tiene cargos ni ciudadanía propia.
- **Vasallo/Señor**: relación ATADA entre 2 Facciones.
- **Aliado**: relación LIBRE y revocable entre 2 Facciones.
- **Gran Rey**: título de PRESTIGIO (no cargo mecánico) del Rey cuya Facción domina otras Facciones enteras.

## 2.2 Cargos

### Nivel Facción
- **Rey**: autoridad sobre vasallaje, designación de Gobernadores de los asentamientos de su Facción (ver más abajo), políticas superiores hacia asentamientos de la Facción (sugerencia u obligatorias), y todas las funciones del Embajador. Automático si la Liga se formó por vasallaje (Rey de la Facción señora); ELECTO por voto entre representantes si se formó por alianza entre iguales (sin empates permitidos, se repite la votación si hay).
  - **Fase 0 (IMPLEMENTADO, a petición del usuario 2026-09-10): una Facción SIEMPRE tiene Rey.** Quien la crea (`crearFaccion`) queda automáticamente como su primer Rey, sin votación — es el caso base de "Rey automático por vasallaje" aplicado a una Facción de un solo miembro. Al abandonar la Facción (`dejarFaccion`), el trono pasa al siguiente ciudadano de la lista (orden de ingreso, el fundador primero); solo queda vacío si se va el último ciudadano. Un traspaso deliberado (`asignarRey`) lo puede hacer solo el Rey vigente. Cierra el "PENDIENTE: sucesión de un Rey-por-vasallaje" que tenía este documento.
- **Embajador**: crea alianzas y declara guerras. Designado directamente por el Rey.

### Nivel asentamiento (el Gobernador designa a los demás; al Gobernador lo designa el Rey)
- **Gobernador**: máxima autoridad del asentamiento, designa al resto de cargos locales. Ve y controla la cola de auto-construcción (ver Doc 4.2). **Lo designa el REY de la Facción dueña del asentamiento** (Fase 0, a petición del usuario 2026-09-10 — antes lo hacía cualquier residente; el modelo canónico "ELECTO por ciudadanos" queda para cuando haya votación real). Es un acto de nivel Facción: el Rey no necesita residir ni estar presente en la plaza, igual que al designar Embajador. Los otros cuatro cargos los designa el Gobernador vigente, y para ello tiene que residir y estar PRESENTE en el asentamiento (ver 2.5).
- **Tesorero**: gestión económica completa (trueque + Mercado).
- **General**: mando militar del asentamiento — coordina el combate conjunto (ver Doc 5.2/5.10), pero NO es un gate de reclutamiento (corrección a petición del usuario, cierra la ambigüedad que había con 2.5: reclutar tropas es beneficio de RESIDENCIA, no de cargo — cualquier jugador residente recluta y amplía SU PROPIO escuadrón sin necesidad de que exista un General asignado).
- **Maestro de Obras**: gestiona prioridades de auto-construcción, bonus a tiempos de construcción. Puede reordenar, añadir y quitar proyectos de la cola — nunca elegir su ubicación (ver Doc 4.2).
- **Sacerdote**: acelerador de aparición de Nobleza (no requisito) + bonificación de felicidad.

### Reglas generales de cargos
- Un jugador PUEDE ejercer varios cargos a la vez.
- Cargo se LIBERA tras 1 semana de inactividad del jugador que lo ocupa.
- Sucesión del Rey al abandonar la Facción: RESUELTA en Fase 0 — el trono pasa al siguiente ciudadano (ver 2.2, "Nivel Facción"). El detalle fino de una sucesión con Liga (¿hereda el vasallaje?, ¿se re-vota en federación?) sigue pendiente hasta que haya votación real.

## 2.2.1 Nivel de Facción y cupo de expansión (IMPLEMENTADO — resuelve el "PENDIENTE" que tenía este documento)
- **Nivel de Facción por EXPERIENCIA** (`calcularNivelFaccion`, `engine/faccion.ts`; curva de umbrales `NIVEL_FACCION.xpParaNivel`, `constants.ts`; placeholder sin calibrar por simulación, tope nivel 10): la experiencia es MONÓTONA (nunca baja) y se otorga por evento, POR JUGADOR PARTICIPANTE en el caso de combate — combate (asedio/campo abierto/interceptar caravana/atacar campamento de bandidos), edificio nuevo completado, conquista, y defensa/ataque de caravana (`NIVEL_FACCION.xp`, `engine/faccion.ts` `aplicarAjustesExperiencia`). El nivel de Facción alimenta DOS cosas distintas — no confundir con el nivel de ASENTAMIENTO (Doc 4.5), que es un modelo de gates aparte:
  1. El **cap de fundación** (Doc 1.7): cuántos asentamientos puede FUNDAR la Facción en total.
  2. El **cupo de asentamientos en nivel 2/3** (siguiente punto): cuántos de sus asentamientos pueden OPERAR en cada nivel a la vez.
- **Cupo de asentamientos por nivel** (Consideraciones/`Fase_0_5_Definicion_Especializacion_y_Cupos.md` §5, a petición del usuario — anti-snowball: no todos los asentamientos de una Facción pueden ser de nivel alto a la vez): `CUPO_NIVEL_ASENTAMIENTO.maxNivel2`/`maxNivel3` (`constants.ts`) definen, por nivel de Facción, cuántos asentamientos propios pueden estar en `nivelActual` 2 y 3 simultáneamente (curvas placeholder sin calibrar: `maxNivel2 = [1,1,1,1,2,2,2,3,3,3]`, `maxNivel3 = [0,0,1,1,1,2,2,2,2,3]` para Facción nivel 1-10). Solo cubre nivel 2 y 3 — nivel 1 no tiene cupo (siempre libre) y nivel 4/5 (Doc 4.5) todavía no tienen curva definida, quedan sin tope mientras tanto.
  - **Enforcement**: `avanzarNivelAsentamiento` (`engine/mantenimiento.ts`) solo promueve un asentamiento si cumple los GATES de población/edificios (Doc 4.5) Y además hay cupo libre en el nivel objetivo — un asentamiento que cumple los gates de sobra pero no tiene cupo se queda "elegible, esperando cupo" indefinidamente (nunca se bloquea ni se degrada por eso), y lo consigue en cuanto se libere uno (subir de nivel 2 a 3 libera, dentro del mismo tick, el cupo de nivel 2 que abandona) o la Facción suba de nivel. El cupo se cuenta contra `nivelActual` (operativo), no contra `nivel`/nivelAlcanzado — una ciudad degradada por mal mantenimiento libera su cupo de verdad, no lo bloquea para siempre.
  - **UI**: el panel de un asentamiento (pestaña Asentamientos) muestra un aviso explícito cuando este cupo es el motivo de que no suba de nivel pese a tener el 100% de los gates cumplidos (`GameStore.cupoNivelInfo`, `src/app/gameStore.ts`).
- **Pestaña "Facción" (nueva, a petición del usuario)**: panel dedicado por Facción (mismo patrón de pestaña con selector que Asentamientos/Jugadores) con nivel, progreso de XP hacia el siguiente nivel, Rey/Embajador/reputación, cap de fundación y cupo de asentamientos nivel 2/3 (ocupados/total), tabla de asentamientos propios, relaciones diplomáticas (2.3/2.4), Liga (2.3) y títulos que ostenta (2.9). Implementado en `renderFaccionesTab`/`renderDetalleFaccion` (`src/main.ts`), apoyado en `GameStore.nivelFaccionInfo`/`cupoAsentamientosFaccion` (`src/app/gameStore.ts`).
- PENDIENTE: calibración por simulación de `NIVEL_FACCION.xpParaNivel` y de las curvas de cupo (ambas placeholder); curva de cupo para nivel de asentamiento 4/5 sin definir todavía (ver Doc 4.5 y `Fase_0_6_Definicion_Expansion_Niveles_Asentamiento.md`).

## 2.3 Formación de Liga (dos caminos)
1. **Por vasallaje** (jerarquía, SIN votación): una Facción reúne una o más Facciones vasallas → el Rey de la Facción señora es Rey por defecto.
2. **Por alianza entre iguales** (federación, CON votación): varias Facciones independientes se federan como iguales → se vota Rey.

Una Facción puede someter a otra Facción entera como vasalla, o federarse con otra como iguales, en cualquier escala (esto es lo que genera el título de Gran Rey, ver 2.7).

## 2.4 Reglas de vasallaje

**Obligaciones:**
1. El vasallo paga TRIBUTO al señor, en la forma que este decida.
2. El señor debe DEFENDER a sus vasallos: declarar guerra a un vasallo declara automáticamente guerra a su señor, y viceversa (guerra al señor = guerra a todos sus vasallos).

**Formación:** propuesta diplomática — protección a cambio de tributo pactado en recursos específicos.

**Ruptura (4 vías):**
1. Rebelión forzada del vasallo → declaración de guerra automática del vasallo hacia el señor (y resto de vasallos), Y cancelación inmediata de todos los acuerdos comerciales/tratados vigentes.
2. Liberación voluntaria por el señor.
3. Conquista por un tercero → destrucción total O anexión como vasalla del conquistador.
4. Liberación automática si el señor es destruido o se "desarma" (condición exacta de desarme pendiente).

## 2.5 Ciudadanía
- Se liga a la FACCIÓN del jugador (NO a la Liga completa — los vasallos mantienen ciudadanía separada de su señor).
- Obtención, TRES vías (la tercera y la posibilidad de abandonar, RESUELTAS a petición del usuario, 2026-08-27 — antes solo existían las dos primeras y no había forma de abandonar):
  1. **Fundar un asentamiento** — cada fundador recibe automáticamente una casa en el asentamiento recién fundado, ocupando un espacio del mismo cupo de vivienda que (2), y con ella ciudadanía inmediata.
  2. **Comprar una casa** en un asentamiento de la propia Facción (espacios limitados según nivel/tamaño). El cupo base de un asentamiento recién fundado es de 5 casas — igual al máximo de jugadores que pueden fundar juntos (Doc 1.2/1.3) — así que siempre hay sitio para todos los fundadores; si fundan menos de 5, los espacios restantes quedan libres para que otros jugadores compren casa después (p. ej. 2 fundadores dejan 3/5 libres).
  3. **Crear la Facción** — quien la crea queda como su primer ciudadano **y su primer Rey** de inmediato, sin necesidad de fundar ni comprar casa todavía (`crearFaccion`, `session/comandos/crearFaccion.ts`; ver 2.2 "una Facción SIEMPRE tiene Rey"). O **unirse a una ya existente** sin comprar casa ni residir en ningún asentamiento suyo (`unirseAFaccion`) — a diferencia de (2), no consume cupo de vivienda ni dan residencia, solo ciudadanía.
- **Abandonar** (`dejarFaccion`, sin parámetros — el actor solo puede dejar SU PROPIA Facción): quita la ciudadanía. Si el que se va era Rey, el trono pasa al siguiente ciudadano (queda vacío solo si era el último); si era Embajador, la embajada se libera. **No** libera la residencia (casa comprada o de fundador) ni cargos LOCALES (Gobernador, etc.) en asentamientos de la Facción abandonada — limitación conocida (no hay comando "vender casa" a secas).
- **Cambiar de residencia** (`cambiarResidencia(destinoId)`, 2026-09-08, `engine/faccion.ts`): atómico — deja la residencia actual (fuera de `casasCompradas` y de fundadores, se libera la vivienda, se vacían los cargos locales viejos) y toma una nueva en otro asentamiento **de la misma Facción** que tenga hueco de vivienda y lo permita (`politicaDeAcceso` ≠ `cerrado`, sin veto). Los escuadrones propios posados en la residencia vieja **se quedan** como guarnición de no-residente (Doc 5.4/5.8). Es el prerrequisito de que un jugador consolide una conquista. Un HUÉRFANO (le tomaron su única plaza) no tiene residencia de la que salir → usa `comprarCasa` / `unirseAFaccion`. Cooldown (`CIUDADANIA.cooldownCambioResidenciaDias`) reservado, **sin implementar todavía** (necesita jugador situado).
- **1 jugador, 1 Facción a la vez**: `crearFaccion` y `unirseAFaccion` rechazan si el actor ya es ciudadano de otra. Anti-abuso "crear, abandonar, crear" en bucle: tras abandonar, no se puede CREAR una Facción nueva hasta pasados 7 días (`CIUDADANIA.cooldownCreacionFaccionDias`, `constants.ts`) — el cooldown solo afecta a crear, no a unirse a una existente (esa vía sigue libre de inmediato tras abandonar).
- Beneficios: ejercer cargo, iniciar caravanas en Mercados de la Facción, votar políticas, reclutar tropas (matizado 2026-09-08: reclutar un escuadrón **nuevo** o cambiar de composición sigue siendo solo en tu residencia; **reponer** un escuadrón que ya tienes posado, y **mover** escuadrones propios, se pueden en cualquier plaza de tu Facción que lo permita, estando presente — ver Doc 5.8), comisiones de comercio más bajas dentro de la misma Facción.
- RESUELTO (a petición del usuario, cierra la ambigüedad anterior de esta línea): residencia es en UN solo asentamiento a la vez, nunca varios — comprar una segunda casa se rechaza mientras el jugador siga residiendo en otro (`comprarCasa`, engine/faccion.ts).
- **LA CIUDADANÍA HABILITA, LA PRESENCIA EJERCE** (a petición del usuario, 2026-09-06 — ver Doc 1.10). Ser ciudadano es lo que te da derecho a ejercer cargo, reclutar, comerciar y votar políticas; **estar dentro del asentamiento** es lo que te permite hacerlo ahí y ahora. Un Gobernador de campaña sigue siendo el Gobernador y no pierde el cargo, pero no gobierna a distancia: mientras está fuera no puede dar órdenes nuevas a su plaza. Lo ya ordenado sigue corriendo solo.
  - Consecuencia buscada: **la delegación pasa a importar**. Los cargos locales dejan de ser un adorno del que se los quedó primero y se vuelven la forma de que la ciudad siga funcionando mientras su gente está fuera.
  - Residencia y presencia son cosas distintas: se reside en un solo asentamiento (arriba), pero se puede estar dentro de cualquiera al que te dejen entrar (Doc 1.10.5). Reclutar un escuadrón **nuevo** sigue exigiendo las dos cosas — es en tu residencia, y estando en ella. **Reponer** y **mover** tropa propia solo exigen presencia + permiso de la plaza (2026-09-08, Doc 5.8).
- PENDIENTE: protección militar, voz en política exterior; nivel intermedio de comisiones para Facciones aliadas/vasallas.
  - **Aviso para cuando se aborde:** proteger a un aliado o a un vasallo se hace **con columna propia**, marchando a defenderlo. **Un aliado NO puede marchar dentro de un Ejército ajeno** (Doc 5.14.1): un ejército lo componen ciudadanos de una sola Facción. Dos columnas amigas pueden compartir camino sin atacarse y llegar a la vez, pero pelean cada una por su cuenta. Una alianza se ejerce en el comercio y en la no agresión — no compartiendo columna.

## 2.6 Fusión/anexión voluntaria entre 2 Facciones
Menú con 2 opciones al ejecutar la acción:
1. **Anexión** (A absorbe a B): A mantiene nombre/Rey/Embajador sin voto. Todo lo de B pasa a A. Cargos de Facción de B se disuelven; cargos LOCALES de asentamiento de B se mantienen.
2. **Fusión** (nace Facción C): A y B se disuelven, se VOTA Rey de C entre representantes de ambas (lógica de Liga-por-alianza). Cargos de Facción anteriores se disuelven y re-designan; cargos locales se mantienen.

No cuenta contra el Cap de Fundación (vía "pacífica" de crecimiento). PENDIENTE: si requiere aceptación mutua explícita o Opción 1 se puede forzar unilateralmente con suficiente diferencia de poder.

## 2.7 Sistema de reputación/confiabilidad de Facción
Score PÚBLICO de -100 (nada confiable) a +100 (muy confiable).

**Penalizaciones:** romper Alianza unilateralmente; no defender a un vasallo atacado; rebelión de vasallo por incumplimiento del señor; incumplir acuerdo de trueque/orden de mercado aceptada; atacar a un Aliado sin romper la relación antes (la más severa).

**Bonificaciones:** defender exitosamente a un vasallo; mantener Alianza activa mucho tiempo; cumplir acuerdos de trueque; liberar voluntariamente a un vasallo.

**Decaimiento:** el score decae lentamente hacia 0 con el tiempo sin eventos nuevos.

**Usos del score:**
1. Términos de comercio asimétricos (score bajo = pagar primero en trueques).
2. Coste de mercenarios/escoltas (más caro con score bajo).
3. Restricciones del Embajador (cooldown/coste extra para proponer alianzas con score bajo).
4. Dificultad para atraer Aedas residentes con score muy bajo.

El score es PÚBLICO y total (no hay sistema de rumores/espionaje que lo oculte parcialmente).

PENDIENTE: valores numéricos exactos de cada evento, velocidad de decaimiento, umbrales exactos de cada uso.

## 2.8 Mecánicas heredadas de Iberia (política local)
- **Exilio**: el Gobernador puede decretar exilio de jugadores de Facciones rivales de su territorio; el exiliado pierde parte de sus materiales como "tasas de emergencia" y debe desplazarse a recuperarlos.
- **Identidad visual**: cada Facción tiene su propio sigilo/estandarte; una Liga puede tener uno colectivo.

## 2.9 Progresión sin condición de victoria
El juego es un SANDBOX de guerra persistente, SIN condiciones de victoria PARA EL JUGADOR. En vez de victoria personal, existen TÍTULOS DINÁMICOS de PRESTIGIO (sin beneficio mecánico, solo prestigio) que cambian de mano según el poder relativo, recalculados PERIÓDICAMENTE (no en tiempo real). Ejemplos de referencia (lista abierta, no cerrada): imperio/Facción más grande, general con más victorias, Facción con mayor poder económico, ejército más grande, "Gran Rey". El histórico de títulos se narra por los AEDAS/POETAS (ver Doc 6), consultable vía interfaz dedicada Y eventos in-game.

**NOTA (ampliación, a petición del usuario — inspirado en análisis comparativo con Travian, ver `Roadmap_Escalado.md` Eje 4)**: lo de arriba sigue siendo cierto a nivel de JUGADOR — nadie "gana" la partida. Pero cada INSTANCIA DE SERVIDOR sí tiene un ciclo de vida acotado (~12 meses, o antes si una Facción completa la Maravilla del ciclo) que termina en un reseteo del mundo. Ver `Roadmap_Escalado.md` Eje 4 para el mecanismo completo, incluyendo el legado NPC de la Facción que completa la Maravilla.

## 2.10 Gremios (edificios especiales, escasos a nivel de servidor)
No todos los asentamientos pueden tenerlos — solo las ciudades más importantes. Se obtienen cuando el gremio correspondiente "propone" colocar una sede, mediante TIRADA PERIÓDICA mientras se cumplan los requisitos (no es una barra de progreso).

**Requisitos (distintos por gremio, misma naturaleza de 4 condiciones independientes agrupadas en 3 puntos):**
1. Score de reputación de Facción por encima de un umbral (ref. inicial: >90).
2. Título de servidor específico (uno de los Títulos Dinámicos de Prestigio).
3. AMBOS a la vez (confirmado): Nivel de asentamiento en el máximo (Nivel 3 en Fase 0, ver Doc 4.5) Y medidor de Mantenimiento por encima de un umbral (ref. inicial: >90%). No es uno u otro — son dos condiciones independientes que deben cumplirse simultáneamente.

**Pérdida (RESUELTO, dos mecanismos distintos según la causa):**
1. **Pérdida del Título de servidor**: el título SOLO se evalúa en el momento de la tirada de aparición inicial — una vez construido el gremio, el título deja de vigilarse en tiempo real. En su lugar, el gremio tiene una DURACIÓN MÍNIMA garantizada en la ciudad. Al cumplirse ese plazo, SI el dueño actual ya no posee el título, se lanza el evento de tirada a cualquier otra Facción/asentamiento que cumpla las 4 condiciones en ese momento. Si un nuevo candidato ACEPTA construir el gremio, el dueño anterior lo PIERDE (transferencia real a la nueva sede).
2. **Pérdida de Score de reputación, Nivel de asentamiento, o Mantenimiento** (los otros 3 requisitos): si se incumple cualquiera de estos EN CUALQUIER MOMENTO (sin esperar a la duración mínima), el gremio se va y el asentamiento se queda SIN SEDE de ese gremio — no hay transferencia automática a otro candidato; el gremio queda disponible en el servidor hasta que alguna Facción/asentamiento vuelva a cumplir las 4 condiciones y gane la siguiente tirada periódica.

**Los 4 gremios:**
- **Comerciantes:** comisiones aún más bajas y/o slot extra de órdenes de mercado y/o rutas/Aedas comerciales especiales (detalle pendiente).
- **Artesanos:** recetas/equipo de tier superior exclusivo y/o bonus de producción en Fundición/Curtidor-Armero (detalle pendiente).
- **Constructores:** bonus adicional de velocidad de auto-construcción y/o edificios únicos (detalle pendiente).
- **Ladrones (confirmado):** información sobre acuerdos de comercio de OTRAS Facciones, información general de caravanas, e información de otras Facciones no visible de otra forma. NO revive el sistema de rumores/espionaje general (sigue descartado como mecánica base) — es un beneficio específico y acotado, diseñado para no ser demasiado diferenciador.

PENDIENTE: valores numéricos exactos por gremio, título de servidor asociado a cada uno, detalle de beneficios de los 3 gremios no confirmados, duración exacta de la duración mínima garantizada (nueva, ver mecanismo de pérdida por título).
