# 2. Sistema Político: Facciones, Cargos y Diplomacia

## 2.1 Glosario rápido (ver página "Glosario de Entidades Políticas" para el detalle completo)
- **Héroe** (el personaje de cada Jugador en el mundo, uno por mundo, Doc 5.16): pertenece a 1 y solo 1 Facción, y reside en 1 y solo 1 asentamiento (Doc 2.5). Tiene como mucho un escuadrón de cada tropa en toda la partida (Doc 5.8); sus escuadrones son SUYOS, no del asentamiento (Doc 5.4), y su Liderazgo limita cuántos puede llevarse consigo (Doc 5.11).
- **Asentamiento**: pertenece a 1 y solo 1 Facción.
- **Facción**: entidad política soberana, agrupa héroes y asentamientos. Tiene cargos de nivel Facción (Rey, Embajador).
- **Liga**: red de Facciones conectadas entre sí (por vasallaje y/o alianza). NO tiene cargos ni ciudadanía propia.
- **Vasallo/Señor**: relación ATADA entre 2 Facciones.
- **Aliado**: relación LIBRE y revocable entre 2 Facciones.
- **Gran Rey**: título de PRESTIGIO (no cargo mecánico) del Rey cuya Facción domina otras Facciones enteras.

## 2.2 Cargos

### Nivel Facción
- **Rey**: autoridad sobre vasallaje, designación de Gobernadores de los asentamientos de su Facción (ver más abajo), políticas superiores hacia asentamientos de la Facción (sugerencia u obligatorias), y todas las funciones del Embajador. Automático si la Liga se formó por vasallaje (Rey de la Facción señora); ELECTO por voto entre representantes si se formó por alianza entre iguales (sin empates permitidos, se repite la votación si hay).
  - **Fase 0: una Facción SIEMPRE tiene Rey.** Quien la crea queda automáticamente como su primer Rey, sin votación — es el caso base de "Rey automático por vasallaje" aplicado a una Facción de un solo miembro. Al abandonar la Facción, el trono pasa al siguiente ciudadano de la lista (orden de ingreso, el fundador primero); solo queda vacío si se va el último ciudadano. Un traspaso deliberado (`asignarRey`) solo lo puede hacer el Rey vigente.
- **Embajador**: crea alianzas y declara guerras. Designado directamente por el Rey.

### Nivel asentamiento (el Gobernador designa a los demás; al Gobernador lo designa el Rey)
- **Gobernador**: máxima autoridad del asentamiento, designa al resto de cargos locales. Ve y controla la cola de auto-construcción (ver Doc 4.2). **Lo designa el REY de la Facción dueña del asentamiento** (Fase 0; el modelo canónico, ELECTO por ciudadanos, llega con la votación real). Es un acto de nivel Facción: el Rey no necesita residir ni estar presente en la plaza, igual que al designar Embajador. Los otros cuatro cargos los designa el Gobernador vigente, y para ello tiene que residir y estar PRESENTE en el asentamiento (ver 2.5).
- **Tesorero**: gestión económica completa (trueque + Mercado).
- **General**: mando militar del asentamiento — coordina el combate conjunto (ver Doc 5.2), pero NO es un gate de reclutamiento: reclutar es beneficio de RESIDENCIA (2.5), así que cualquier héroe residente recluta y repone SUS escuadrones sin necesidad de que exista un General asignado.
- **Maestro de Obras**: gestiona prioridades de auto-construcción, bonus a tiempos de construcción. Puede reordenar, añadir y quitar proyectos de la cola — nunca elegir su ubicación (ver Doc 4.2).
- **Sacerdote**: acelerador de aparición de Nobleza (no requisito) + bonificación de felicidad.

### Reglas generales de cargos
- Un jugador PUEDE ejercer varios cargos a la vez.
- Cargo se LIBERA tras 1 semana de inactividad del jugador que lo ocupa.
- Sucesión del Rey al abandonar la Facción: el trono pasa al siguiente ciudadano (ver "Nivel Facción").

## 2.2.1 Nivel de Facción y cupo de expansión
- **Nivel de Facción por EXPERIENCIA** (curva de umbrales `NIVEL_FACCION.xpParaNivel`, placeholder, tope nivel 10): la experiencia es MONÓTONA (nunca baja) y se otorga por evento (`NIVEL_FACCION.xp`): combate (asedio, campo abierto), POR HÉROE PARTICIPANTE; conquista; edificio nuevo completado; y **crecer en paz** (decidido el 2026-09-27): terminar la subida de un asentamiento (30 × el nivel alcanzado), fundar un asentamiento que no sea el primero de la Facción (20) y cerrar un trueque (2 a cada lado). **Los campamentos de bandidos** dan 1 por campamento destruido (no por héroe), dividido por el nivel de la Facción y nada desde el nivel 5: enseñan a una Facción joven y dejan de contar después. **Solo el combate DIGNO da experiencia de combate** (decidido el 2026-09-27; los bandidos van aparte, arriba): el bando más débil tiene al menos la mitad del poder del más fuerte (`NIVEL_FACCION.ratioCombateDigno`); aplastar a quien no puede defenderse, o estrellarse contra quien no se puede vencer, no da nada. Ese mismo día la curva de umbrales se duplicó y la experiencia de la guerra (combate y conquista) se redujo a la mitad: en la Era I medida, 6 de 12 Facciones llegaban a nivel 10 y una lo hacía en 1,2 días. Las batallas de Unity dan la experiencia de combate sin este filtro, porque su resultado no trae el poder de cada bando. El nivel de Facción alimenta DOS cosas distintas — no confundir con el nivel de ASENTAMIENTO (Doc 4.5), que es un modelo de requisitos aparte:
  1. El **cap de fundación** (Doc 1.7): cuántos asentamientos puede FUNDAR la Facción en total.
  2. El **cupo de asentamientos en nivel 2/3** (siguiente punto): cuántos de sus asentamientos pueden OPERAR en cada nivel a la vez.
- **Cupo de asentamientos por nivel** (`Consideraciones/Fase_0_5_Definicion_Especializacion_y_Cupos.md` §5 — anti-snowball: no todos los asentamientos de una Facción pueden ser de nivel alto a la vez): `CUPO_NIVEL_ASENTAMIENTO.maxNivel2`/`maxNivel3` definen, por nivel de Facción, cuántos asentamientos propios pueden estar en nivel 2 y 3 simultáneamente (curvas placeholder: `maxNivel2 = [1,1,1,1,2,2,2,3,3,3]`, `maxNivel3 = [0,0,1,1,1,2,2,2,2,3]` para Facción nivel 1-10). Solo cubre nivel 2 y 3: el nivel 1 no tiene cupo, y los niveles 4 y 5 (Doc 4.5) tampoco.
  - Para PEDIR la subida de nivel de un asentamiento (Doc 4.5) tiene que haber cupo libre en el nivel objetivo. **El cupo se reserva al pedirla**: ocupan plaza tanto los asentamientos que ya están en ese nivel como los que tienen una obra de ascenso en marcha hacia él, así que dos asentamientos de la misma Facción no pueden pagar a la vez por la última plaza, y al terminar la obra no se vuelve a mirar. Subir de 2 a 3 libera el cupo de nivel 2 que se abandona. El cupo se cuenta contra el nivel OPERATIVO (`nivelActual`), no contra el alcanzado: una ciudad degradada por mal mantenimiento libera su cupo de verdad.
  - La interfaz enseña cuándo el cupo es uno de los motivos que impiden pedir la subida.

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
4. Liberación automática si el señor es destruido o se "desarma".

### 2.4.1 Guerra (decidido el 2026-10-02)
- **Se declara libre**: sin frontera compartida ni casus belli. La declara el Rey o el Embajador de la Facción (comando `declararGuerra`), contra cualquier Facción con la que no tenga ya una relación activa: para atacar a una aliada o a un señor o vasallo propio hay que romper antes la relación.
- **Se arrastra por vasallaje** (2.4): la guerra a un vasallo es guerra a su señor y a los demás vasallos de este; la guerra a un señor, a todos sus vasallos. Las Facciones del bando contrario con las que ya hay una relación activa no se arrastran.
- **La rebelión de un vasallo** (2.4, ruptura 1) la declara: el vasallo queda en guerra con su señor y con el resto de sus vasallos.
- **Termina por paz mutua**: cualquiera de las dos Facciones la ofrece (`proponerPaz`) y acaba cuando la otra la ofrece también. No hay otra vía; una guerra no se rompe con `romperRelacion`.
- **Qué cambia hoy**: solo el estado. Marca quiénes son «enemigos» para la puerta de un asentamiento (Doc 1.10.5). Atacar o asediar sigue sin exigir guerra declarada, ni la guerra bloquea nada más; se endurecerá cuando se decida.

## 2.5 Ciudadanía
- Se liga a la FACCIÓN del héroe (NO a la Liga completa — los vasallos mantienen ciudadanía separada de su señor).
- Obtención, TRES vías:
  1. **Fundar un asentamiento** — cada fundador recibe automáticamente una casa en el asentamiento recién fundado, y con ella ciudadanía inmediata.
  2. **Comprar una casa** en un asentamiento de la propia Facción. **No hay tope de residentes** por asentamiento: el tope que cuenta es el de héroes que entran en una batalla (Doc 5.15.1).
  3. **Crear la Facción** — quien la crea queda como su primer ciudadano **y su primer Rey** de inmediato, sin necesidad de fundar ni comprar casa todavía (ver 2.2, "una Facción SIEMPRE tiene Rey"). O **unirse a una ya existente** sin comprar casa ni residir en ningún asentamiento suyo — a diferencia de (2), no da residencia, solo ciudadanía.
- **Abandonar** (el héroe solo puede dejar SU PROPIA Facción): quita la ciudadanía. Si el que se va era Rey, el trono pasa al siguiente ciudadano (queda vacío solo si era el último); si era Embajador, la embajada se libera. **No** libera la residencia (casa comprada o de fundador) ni los cargos LOCALES (Gobernador, etc.) en asentamientos de la Facción abandonada.
- **Cambiar de residencia** (`cambiarResidencia`): atómico — deja la residencia actual (se libera su vivienda y se vacían sus cargos locales ahí) y toma una nueva en otro asentamiento **de la misma Facción** que lo permita (sin veto). **El héroe traslada con ella su campamento**: sus escuadrones pasan a la nueva residencia, y los que tuviera en la guarnición de la vieja dejan de serlo, porque solo se guarnece donde se reside (Doc 5.15.3). Es lo que hace un héroe para consolidar una conquista. Un HUÉRFANO (Doc 0) no tiene residencia de la que salir: usa comprar casa o unirse a una Facción.
- **1 héroe, 1 Facción a la vez**: crear una Facción o unirse a una se rechaza si el héroe ya es ciudadano de otra. Anti-abuso "crear, abandonar, crear" en bucle: tras abandonar, no se puede CREAR una Facción nueva hasta pasados 7 días (`CIUDADANIA.cooldownCreacionFaccionDias`) — el cooldown solo afecta a crear, no a unirse a una existente (esa vía sigue libre de inmediato tras abandonar).
- Beneficios: ejercer cargo, iniciar caravanas en Mercados de la Facción, votar políticas, reclutar tropas (reclutar un escuadrón **nuevo** solo en tu residencia; **reponer** un escuadrón que ya tienes y **mover** escuadrones propios, en cualquier plaza de tu Facción que lo permita, estando presente — ver Doc 5.8), comisiones de comercio más bajas dentro de la misma Facción.
- La residencia es UN solo asentamiento a la vez, nunca varios: comprar una segunda casa se rechaza mientras el héroe siga residiendo en otro.
- **LA CIUDADANÍA HABILITA, LA PRESENCIA EJERCE** (ver Doc 1.10). Ser ciudadano es lo que te da derecho a ejercer cargo, reclutar, comerciar y votar políticas; **estar dentro del asentamiento** es lo que te permite hacerlo ahí y ahora. Un Gobernador de campaña sigue siendo el Gobernador y no pierde el cargo, pero no gobierna a distancia: mientras está fuera no puede dar órdenes nuevas a su plaza. Lo ya ordenado sigue corriendo solo.
  - Consecuencia buscada: **la delegación importa**. Los cargos locales no son un adorno del que se los quedó primero: son la forma de que la ciudad siga funcionando mientras su gente está fuera.
  - Residencia y presencia son cosas distintas: se reside en un solo asentamiento (arriba), pero se puede estar dentro de cualquiera al que te dejen entrar (Doc 1.10.5). Reclutar un escuadrón **nuevo** exige las dos cosas — es en tu residencia, y estando en ella. **Reponer** y **mover** tropa propia solo exigen presencia + permiso de la plaza (Doc 5.8).

## 2.6 Fusión/anexión voluntaria entre 2 Facciones
Menú con 2 opciones al ejecutar la acción:
1. **Anexión** (A absorbe a B): A mantiene nombre/Rey/Embajador sin voto. Todo lo de B pasa a A. Cargos de Facción de B se disuelven; cargos LOCALES de asentamiento de B se mantienen.
2. **Fusión** (nace Facción C): A y B se disuelven, se VOTA Rey de C entre representantes de ambas (lógica de Liga-por-alianza). Cargos de Facción anteriores se disuelven y re-designan; cargos locales se mantienen.

No cuenta contra el Cap de Fundación (vía "pacífica" de crecimiento).

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

## 2.8 Mecánicas heredadas de Iberia (política local)
- **Exilio**: la puerta de cada asentamiento se puede cerrar por grupo —neutrales, aliados o vasallos, enemigos en guerra (2.4.1)—; nunca a la propia Facción. Lo cambia el Gobernador de ese asentamiento o el Rey, sin coste (decidido e implementado el 2026-10-02, Doc 1.10.5). La pérdida de materiales y tropas al perder la casa es la reubicación por conquista (Doc 5.15.5), no el exilio.
- **Identidad visual**: cada Facción tiene su propio sigilo/estandarte; una Liga puede tener uno colectivo.

## 2.9 Progresión sin condición de victoria
El juego es un SANDBOX de guerra persistente, SIN condiciones de victoria PARA EL JUGADOR. En vez de victoria personal, existen TÍTULOS DINÁMICOS de PRESTIGIO (sin beneficio mecánico, solo prestigio) que cambian de mano según el poder relativo, recalculados PERIÓDICAMENTE (no en tiempo real). Ejemplos de referencia (lista abierta, no cerrada): imperio/Facción más grande, general con más victorias, Facción con mayor poder económico, ejército más grande, "Gran Rey". El histórico de títulos se narra por los AEDAS/POETAS (ver Doc 6), consultable vía interfaz dedicada Y eventos in-game.

**El servidor sí tiene un final.** Nadie "gana" la partida como jugador, pero cada INSTANCIA DE SERVIDOR tiene un ciclo de vida acotado (~12 meses, o antes si una Facción completa la Maravilla del ciclo) que termina en un reseteo del mundo, con la Facción que completa la Maravilla persistiendo como legado NPC. Mecanismo completo en `Roadmap_Escalado.md` Eje 4.

## 2.10 Gremios (edificios especiales, escasos a nivel de servidor)
No todos los asentamientos pueden tenerlos — solo las ciudades más importantes. Se obtienen cuando el gremio correspondiente "propone" colocar una sede, mediante TIRADA PERIÓDICA mientras se cumplan los requisitos (no es una barra de progreso).

**Requisitos (distintos por gremio, misma naturaleza de 4 condiciones independientes agrupadas en 3 puntos):**
1. Score de reputación de Facción por encima de un umbral (ref. inicial: >90).
2. Título de servidor específico (uno de los Títulos Dinámicos de Prestigio).
3. AMBOS a la vez: nivel de asentamiento en el máximo (Doc 4.5) Y medidor de Mantenimiento por encima de un umbral (ref. inicial: >90%). No es uno u otro — son dos condiciones independientes que deben cumplirse simultáneamente.

**Pérdida (dos mecanismos distintos según la causa):**
1. **Pérdida del Título de servidor**: el título SOLO se evalúa en el momento de la tirada de aparición inicial — una vez construido el gremio, el título deja de vigilarse en tiempo real. En su lugar, el gremio tiene una DURACIÓN MÍNIMA garantizada en la ciudad. Al cumplirse ese plazo, SI el dueño actual ya no posee el título, se lanza el evento de tirada a cualquier otra Facción/asentamiento que cumpla las 4 condiciones en ese momento. Si un nuevo candidato ACEPTA construir el gremio, el dueño anterior lo PIERDE (transferencia real a la nueva sede).
2. **Pérdida de Score de reputación, Nivel de asentamiento, o Mantenimiento** (los otros 3 requisitos): si se incumple cualquiera de estos EN CUALQUIER MOMENTO (sin esperar a la duración mínima), el gremio se va y el asentamiento se queda SIN SEDE de ese gremio — no hay transferencia automática a otro candidato; el gremio queda disponible en el servidor hasta que alguna Facción/asentamiento vuelva a cumplir las 4 condiciones y gane la siguiente tirada periódica.

**Los 4 gremios:**
- **Comerciantes:** comisiones aún más bajas y/o slot extra de órdenes de mercado y/o rutas/Aedas comerciales especiales.
- **Artesanos:** recetas/equipo de tier superior exclusivo y/o bonus de producción en Fundición/Curtidor-Armero.
- **Constructores:** bonus adicional de velocidad de auto-construcción y/o edificios únicos.
- **Ladrones:** información sobre acuerdos de comercio de OTRAS Facciones, información general de caravanas, e información de otras Facciones no visible de otra forma. NO revive el sistema de rumores/espionaje general (sigue descartado como mecánica base) — es un beneficio específico y acotado, diseñado para no ser demasiado diferenciador.
