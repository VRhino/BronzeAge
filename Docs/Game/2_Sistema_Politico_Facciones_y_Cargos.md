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
  - **Ajuste permanente «Admitir a otras Facciones en nuestros ataques»** (2026-10-06, Doc 5.15.1b): solo lo cambia el Rey, desactivado por defecto. Activado, en los asedios y asaltos de caravana que abra su Facción pueden unirse al ataque héroes de Facciones neutrales o enemigas del defensor (nunca aliadas suyas), mientras quede sitio.
- **Embajador**: crea alianzas y declara guerras. Designado directamente por el Rey.
- **Capital** (decidido el 2026-10-02): la **designa el Rey** (`designarCapital`), entre los asentamientos de la Facción que tengan un **Palacio activo** (4.2.1). Es donde el Rey adopta tecnología (Doc 6.5) y el centro del mantenimiento por distancia (Doc 4.5). **Trasladarla tiene un cooldown** (`CAPITAL.cooldownDias`, placeholder 14 días de mundo) y no cuesta recursos; el primer nombramiento es libre. Mientras no haya una designada, y si la designada se pierde (se conquista el asentamiento o se queda sin Palacio), la capital es el **asentamiento vivo más antiguo** hasta que el Rey designe otra, sin esperar el cooldown. No hay cargo de Facción nuevo: es un acto del Rey, como designar Embajador.

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
4. Liberación automática si el señor es destruido o se «desarma» (decidido e implementado el 2026-10-06): un señor está **desarmado** cuando su Facción **se queda sin ningún asentamiento**, sea por conquista, ruina o abandono — «destruido» y «desarmado» son el mismo estado, y una Facción sin plazas no desaparece del mundo (Doc 5.15.5). En ese momento sus vasallos quedan libres: el vasallaje pasa a `rota` y se cantan en la crónica de los Aedas. **No cambia la reputación de nadie** (el señor no decide nada: el bonus de 2.7 es por liberar *voluntariamente*) **ni declara guerra**. Se barre una vez por tick, así que la liberación llega en el tick siguiente al hecho. Si el señor vuelve a fundar, ya no tiene vasallos.

### 2.4.1 Guerra (decidido el 2026-10-02)
- **Se declara libre**: sin frontera compartida ni casus belli. La declara el Rey o el Embajador de la Facción (comando `declararGuerra`), contra cualquier Facción con la que no tenga ya una relación activa: para atacar a una aliada o a un señor o vasallo propio hay que romper antes la relación.
- **Se arrastra por vasallaje** (2.4): la guerra a un vasallo es guerra a su señor y a los demás vasallos de este; la guerra a un señor, a todos sus vasallos. Las Facciones del bando contrario con las que ya hay una relación activa no se arrastran.
- **La rebelión de un vasallo** (2.4, ruptura 1) la declara: el vasallo queda en guerra con su señor y con el resto de sus vasallos.
- **Termina por paz mutua**: cualquiera de las dos Facciones la ofrece (`proponerPaz`) y acaba cuando la otra la ofrece también. No hay otra vía; una guerra no se rompe con `romperRelacion`.
- **Qué cambia hoy**: solo el estado. Marca quiénes son «enemigos» para la puerta de un asentamiento (Doc 1.10.5). Atacar o asediar sigue sin exigir guerra declarada, ni la guerra bloquea nada más; se endurecerá cuando se decida.

## 2.5 Ciudadanía
- Se liga a la FACCIÓN del héroe (NO a la Liga completa — los vasallos mantienen ciudadanía separada de su señor).
- Obtención, TRES vías (2026-10-04, D31: no hay compra de casa):
  1. **Crear la Facción** — quien la crea queda como su primer ciudadano **y su primer Rey** de inmediato, sin necesidad de fundar todavía (ver 2.2, "una Facción SIEMPRE tiene Rey").
  2. **Pedir el ingreso** en una existente (`solicitarIngreso`, D46): el héroe entra en su **lista de solicitantes** y **solo el Rey** acepta o deniega (`responderSolicitud`). Aceptado, es ciudadano y sus solicitudes en otras Facciones caen; denegado, sale de la lista. Solo da ciudadanía, no residencia.
  3. **Fundar un asentamiento** — cada fundador recibe automáticamente una casa en el asentamiento recién fundado, y con ella ciudadanía inmediata.
- **No hay tope de residentes** por asentamiento: el tope que cuenta es el de héroes que entran en una batalla (Doc 5.15.1).
- **Abandonar** (el héroe solo puede dejar SU PROPIA Facción): quita la ciudadanía **y la casa**: deja su residencia y sus cargos LOCALES (2026-10-02). Sigue siendo suyo lo que lleva. Si el que se va era Rey, el trono pasa al siguiente ciudadano (queda vacío solo si era el último); si era Embajador, la embajada se libera. 
- **Dejar la residencia** (`dejarResidencia`, 2026-10-02): libera la vivienda y vacía los cargos locales ahí, sin dejar la Facción ni tomar otra casa; el héroe pasa a residir en el campamento de mercenarios más cercano a donde está (abajo), hasta que se mude a otra plaza. No hay reembolso.
- **Residir en un campamento de mercenarios** (`residirEnCampamento`, 2026-10-02; Doc 1.9b): cualquier héroe, de cualquier Facción y **aunque la suya tenga asentamientos**; sin límite de plazas, sin guarnición ni cargos. Deja la casa que tuviera (y sus cargos locales). Es también la casa de quien se queda sin asentamiento: **se acaba el huérfano** (Doc 0). La reubicación ocurre **en el momento del hecho**, no en un barrido: al caer su plaza por conquista, o desaparecer por ruina, o al dejar la casa, el héroe pasa en el acto al campamento más cercano al sitio que perdió, con sus escuadras (si su Facción conserva otra plaza, va a la más cercana de ella, Doc 5.15.5). Un ciudadano recién aceptado sigue residiendo donde estaba (su campamento). Para vivir en un asentamiento de su Facción se muda (`cambiarResidencia`), que lo saca del campamento.
- **Almacén personal** (`guardarEnAlmacenPersonal` / `sacarDelAlmacenPersonal`): todo héroe tiene uno, de 1000 unidades en total, que viaja con él en cada cambio de residencia. Se llena con lo que lleva en el carro de su columna (botín, compras) y se vacía de vuelta al carro; solo el Líder de la columna, porque el carro es común (Doc 5.13.2). No retira nada del almacén de una plaza.
- **Cooldown de residencia** (`CIUDADANIA.cooldownCambioResidenciaDias`, 3 días de mundo, placeholder): tras `cambiarResidencia`, `dejarResidencia` o `residirEnCampamento` no se puede cambiar otra vez hasta que pase. El primero es libre, y la reubicación forzosa por conquista (Doc 5.15.5) no cuenta.
- **Cambiar de residencia** (`cambiarResidencia`): atómico — deja la residencia actual (se libera su vivienda y se vacían sus cargos locales ahí) y toma una nueva en otro asentamiento **de la misma Facción** que lo permita (sin veto). **El héroe traslada con ella su campamento**: sus escuadrones pasan a la nueva residencia, y los que tuviera en la guarnición de la vieja dejan de serlo, porque solo se guarnece donde se reside (Doc 5.15.3). Es lo que hace un héroe para consolidar una conquista, y lo que hace un ciudadano que vive en un campamento para entrar a vivir en una plaza de su Facción: sin plaza de la que salir, solo toma la nueva y deja el campamento.
- **1 héroe, 1 Facción a la vez**: crear una Facción o pedir el ingreso en una se rechaza si el héroe ya es ciudadano de otra. Anti-abuso "crear, abandonar, crear" en bucle: tras abandonar, no se puede CREAR una Facción nueva hasta pasados 7 días (`CIUDADANIA.cooldownCreacionFaccionDias`) — el cooldown solo afecta a crear, no a pedir el ingreso en una existente (esa vía sigue libre de inmediato tras abandonar).
- Beneficios: ejercer cargo, iniciar caravanas en Mercados de la Facción, votar políticas, reclutar tropas (reclutar un escuadrón **nuevo** solo en tu residencia; **reponer** un escuadrón que ya tienes y **mover** escuadrones propios, en cualquier plaza de tu Facción que lo permita, estando presente — ver Doc 5.8), comisiones de comercio más bajas dentro de la misma Facción.
- La residencia es UN solo asentamiento a la vez, nunca varios: comprar una segunda casa se rechaza mientras el héroe siga residiendo en otro.
- **LA CIUDADANÍA HABILITA, LA PRESENCIA EJERCE** (ver Doc 1.10). Ser ciudadano es lo que te da derecho a ejercer cargo, reclutar, comerciar y votar políticas; **estar dentro del asentamiento** es lo que te permite hacerlo ahí y ahora. Un Gobernador de campaña sigue siendo el Gobernador y no pierde el cargo, pero no gobierna a distancia: mientras está fuera no puede dar órdenes nuevas a su plaza. Lo ya ordenado sigue corriendo solo.
  - Consecuencia buscada: **la delegación importa**. Los cargos locales no son un adorno del que se los quedó primero: son la forma de que la ciudad siga funcionando mientras su gente está fuera.
  - Residencia y presencia son cosas distintas: se reside en un solo asentamiento (arriba), pero se puede estar dentro de cualquiera al que te dejen entrar (Doc 1.10.5). Reclutar un escuadrón **nuevo** exige las dos cosas — es en tu residencia, y estando en ella. **Reponer** y **mover** tropa propia solo exigen presencia + permiso de la plaza (Doc 5.8).

## 2.6 Fusión/anexión voluntaria entre 2 Facciones
Menú con 2 opciones al ejecutar la acción:
1. **Anexión** (A absorbe a B): A mantiene nombre/Rey/Embajador sin voto. Todo lo de B pasa a A. Cargos de Facción de B se disuelven; cargos LOCALES de asentamiento de B se mantienen. **Exige la aceptación de B** (decidido e implementado el 2026-10-06):
   - **Propuesta y respuesta.** La propone el **Rey o el Embajador de A** (`proponerAnexion`) y queda pendiente; la contesta **solo el Rey de B** (`responderAnexion`, aceptar o rechazar). Aceptar la ejecuta en el acto. A puede retirarla mientras nadie la ha contestado (`retirarAnexion`).
   - **Caduca** a los `ANEXION.caducidadDias` días de mundo (3, placeholder). Entre dos Facciones hay **una sola propuesta pendiente**, en cualquier sentido. B tiene que tener Rey (si no, nadie puede aceptar). No hay condición de poder, de proximidad ni de nivel: es voluntaria.
   - **B vasalla de un tercero**: no se puede proponer ni aceptar mientras lo siga siendo; su señor la libera antes. Si el señor de B es la propia A, el vasallaje termina con la anexión.
   - **Qué pasa a A**: los asentamientos de B (con sus cargos locales y su guarnición), sus ciudadanos (con su residencia), sus ejércitos, caravanas, rutas comerciales, Aedas residentes y épicas cumplidas, miradas de la taberna, y lo que B recordaba del mundo (la niebla se funde con la de A). **A conserva** su nombre, sigilo, Rey, Embajador, reputación, experiencia y tecnología; **de B se pierden** su sigilo, sus cargos de Facción, su reputación, su experiencia y su tecnología.
   - **Relaciones de B**: sus alianzas y guerras se cancelan **sin penalización de reputación**; sus **vasallos pasan a ser vasallos de A** con el mismo tributo (si A ya tiene una relación activa con ese vasallo, queda libre). Las propuestas pendientes que colgaban de B desaparecen; las que otros hacen a A siguen en pie.
   - **Batalla en curso**: mientras alguna de las dos Facciones tenga una batalla abierta, la anexión no se acepta.
   - **Quién lo ve**: la propuesta pendiente viaja solo en la proyección de las dos Facciones (`propuestasAnexion`), aunque sus eventos son públicos como los demás de diplomacia; el Rey bot rechaza las que recibe. La anexión consumada se canta en la crónica de los Aedas.
2. **Fusión** (nace Facción C): A y B se disuelven y nace C. **Exige el consentimiento de los dos Reyes** (decidido e implementado el 2026-10-06):
   - **Propuesta y respuesta.** La propone **solo el Rey de A** (`proponerFusion`), que **ya fija el nombre y el Rey de C**; este ha de ser **el Rey de A o el de B**. Queda pendiente y la contesta **solo el Rey de B** (`responderFusion`, aceptar o rechazar). Aceptar la ejecuta en el acto; el Rey de A puede retirarla mientras nadie la ha contestado (`retirarFusion`).
   - **La votación es el pacto**: el sí del Rey de B es su voto y el de A está en la propuesta. El Rey que no lo es de C pasa a ser un ciudadano más. Los cargos de Facción de A y B se disuelven (el Embajador de C queda vacante hasta que su Rey designe); los cargos locales de asentamiento se mantienen.
   - **Caduca** a los `FUSION.caducidadDias` días de mundo (3, placeholder). Entre dos Facciones hay **una sola propuesta de fusión pendiente**, en cualquier sentido. Las dos necesitan Rey; no hay condición de Liga, proximidad ni nivel.
   - **Al aceptar se vuelve a validar**: el Rey de A ha de seguir siendo quien la propuso, el Rey nombrado ha de seguir siendo Rey de una de las dos, y ninguna puede ser vasalla de un tercero (su señor la libera antes; si el señor de una es la otra, el vasallaje termina con la fusión). Con una batalla abierta de cualquiera de las dos no se acepta.
   - **Qué hereda C**: el **sigilo de A** (el de B se pierde, Doc 2.8.1), la **experiencia mayor** y la **reputación mayor** de las dos (el nivel se recalcula, así que no se pierde cupo) y la **unión de las tecnologías** (aparecidas, adoptadas y reveladas). Su capital es la que había designado el Rey de C (si no, la plaza más antigua).
   - **Qué pasa a C**: lo mismo que en la anexión — asentamientos con sus cargos locales y su guarnición, ciudadanos con su residencia, ejércitos, caravanas, rutas, Aedas residentes y épicas cumplidas, miradas y lo que recordaban del mundo — por la **misma ruta de traslado**.
   - **Relaciones**: las alianzas y guerras de A y B se cancelan **sin penalización de reputación** y C nace sin ellas; los **vasallos de las dos pasan a ser de C** con el mismo tributo; la relación entre A y B termina. Las propuestas pendientes (de fusión y de anexión) que nombraban a A o a B desaparecen.
   - **Quién lo ve**: como la anexión — la propuesta pendiente viaja solo a las dos Facciones (`propuestasFusion`), los eventos son públicos y el Rey bot rechaza las que recibe. La fusión consumada se canta en la crónica de los Aedas.

No cuenta contra el Cap de Fundación (vía "pacífica" de crecimiento). Decisiones y alternativas descartadas: `Consideraciones/Anexion_Y_Desarme_Definicion.md`.

## 2.7 Sistema de reputación/confiabilidad de Facción
Score PÚBLICO de -100 (nada confiable) a +100 (muy confiable).

**Uso en mercenarios** (2026-10-02): una Facción con reputación baja paga más al reclutar en un campamento de mercenarios (Doc 5.8), con el mismo recargo que en comercio.

**Penalizaciones:** romper Alianza unilateralmente; no defender a un vasallo atacado; rebelión de vasallo por incumplimiento del señor; incumplir acuerdo de trueque/orden de mercado aceptada; atacar a un Aliado sin romper la relación antes (la más severa).

**Bonificaciones:** defender exitosamente a un vasallo; mantener Alianza activa mucho tiempo; cumplir acuerdos de trueque; liberar voluntariamente a un vasallo.

**Decaimiento:** el score decae lentamente hacia 0 con el tiempo sin eventos nuevos.

**Usos del score:**
1. Términos de comercio asimétricos (score bajo = pagar primero en trueques).
2. Coste de mercenarios/escoltas (más caro con score bajo).
3. Restricciones del Embajador (cooldown/coste extra para proponer alianzas con score bajo).
4. Dificultad para atraer Aedas residentes con score muy bajo.

El score es PÚBLICO y total (no hay sistema de rumores/espionaje que lo oculte parcialmente). La taberna (Doc 5.12.10) vende información militar y de territorio pagada y con caducidad; no es un sistema de rumores sobre el score ni sobre la reputación.

## 2.8 Mecánicas heredadas de Iberia (política local)
- **Exilio**: la puerta de cada asentamiento se puede cerrar por grupo —neutrales, aliados o vasallos, enemigos en guerra (2.4.1) y los Aedas (6.7)—; nunca a la propia Facción. Lo cambia el Gobernador de ese asentamiento o el Rey, sin coste (decidido e implementado el 2026-10-02, Doc 1.10.5). La pérdida de materiales y tropas al perder la casa es la reubicación por conquista (Doc 5.15.5), no el exilio.
- **Identidad visual**: cada Facción tiene su propio sigilo/estandarte; una Liga puede tener uno colectivo (ver 2.8.1).

### 2.8.1 Sigilo, estandarte e identidad de imperios (decidido el 2026-10-05; implementado en el backend y en BronzeAgeClient, ver Doc 01 §23; Conquest lo dibuja con su CQ-009)
- **El sigilo es de la Facción** y lo compone su Rey con piezas de un catálogo cerrado: la forma del escudo, el fondo con dos colores, un emblema con su color y una orla opcional con su color. Es un identificador, nunca una imagen subida. Una Facción nueva nace con uno libre derivado de su id.
- **Único por servidor**: no puede haber dos Facciones con el mismo sigilo exacto; los parecidos sí (solo se rechaza el duplicado exacto). Hay sigilos reservados del servidor (sin Facción, bandidos, campamentos de mercenarios) que ninguna Facción elige.
- **Se elige al crear la Facción y no se cambia nunca**: queda guardado para toda la partida. Así los Aedas, la crónica y los ejércitos ya vistos no se confunden. Una Facción que se fusiona con otra (2.6) hereda el de la Facción A, la que propone.
- **La cultura no afecta al sigilo ni a los estandartes**: la Facción no tiene cultura (BA-006 D27). La cultura es del asentamiento y de sus edificios, y las escuadras no cambian de aspecto por ella.
- **La Liga y el Gran Rey no guardan nada**: el Gran Rey añade un marco o corona al estandarte de su Facción; una Liga por vasallaje se muestra con el sigilo de la señora, y una por alianza, con la orla de los de sus miembros. Se derivan de las relaciones y del Gran Rey, como la propia Liga.
- **Cada título de servidor (2.9) tiene una insignia fija**, no elegible, por id estable de título. Los Aedas cantan el traspaso con ella.
- **Cultura y audio quedan fuera de este apartado**: el audio y la música por cultura entran con la cultura (`Docs/Mecanicas a desarrollar.md` §43). Decisiones y plan del sigilo: `Consideraciones/Identidad_Visual_Definicion.md`.

## 2.9 Progresión sin condición de victoria
El juego es un SANDBOX de guerra persistente, SIN condiciones de victoria PARA EL JUGADOR. En vez de victoria personal, existen TÍTULOS DINÁMICOS de PRESTIGIO (sin beneficio mecánico, solo prestigio) que cambian de mano según el poder relativo, recalculados PERIÓDICAMENTE (no en tiempo real). Ejemplos de referencia (lista abierta, no cerrada): imperio/Facción más grande, general con más victorias, Facción con mayor poder económico, ejército más grande, "Gran Rey", "Mecenas de los Aedas" (más épicas cumplidas, Doc 6.7). El histórico de títulos se narra por los AEDAS/POETAS (ver Doc 6), consultable vía interfaz dedicada Y eventos in-game.

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
- **Ladrones:** información sobre acuerdos de comercio de OTRAS Facciones, información general de caravanas, e información de otras Facciones no visible de otra forma. NO revive el sistema de rumores/espionaje general (sigue descartado como mecánica base) — es un beneficio específico y acotado, diseñado para no ser demasiado diferenciador. **No se solapa con la taberna** (Doc 5.12.10): el gremio da, de forma pasiva y por tener su sede, acuerdos y caravanas ajenas; la taberna es un servicio de pago, abierto a cualquiera que la construya, para mirar zonas y pedir el layout y la defensa de una plaza.
