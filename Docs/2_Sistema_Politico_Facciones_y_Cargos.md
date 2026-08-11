# 2. Sistema Político: Facciones, Cargos y Diplomacia

## 2.1 Glosario rápido (ver página "Glosario de Entidades Políticas" para el detalle completo)
- **Jugador**: pertenece a 1 y solo 1 Facción.
- **Asentamiento**: pertenece a 1 y solo 1 Facción.
- **Facción**: entidad política soberana, agrupa jugadores y asentamientos. Tiene cargos de nivel Facción (Rey, Embajador).
- **Liga**: red de Facciones conectadas entre sí (por vasallaje y/o alianza). NO tiene cargos ni ciudadanía propia.
- **Vasallo/Señor**: relación ATADA entre 2 Facciones.
- **Aliado**: relación LIBRE y revocable entre 2 Facciones.
- **Gran Rey**: título de PRESTIGIO (no cargo mecánico) del Rey cuya Facción domina otras Facciones enteras.

## 2.2 Cargos

### Nivel Facción
- **Rey**: autoridad sobre vasallaje, políticas superiores hacia asentamientos de la Facción (sugerencia u obligatorias), y todas las funciones del Embajador. Automático si la Liga se formó por vasallaje (Rey de la Facción señora); ELECTO por voto entre representantes si se formó por alianza entre iguales (sin empates permitidos, se repite la votación si hay).
- **Embajador**: crea alianzas y declara guerras. Designado directamente por el Rey.

### Nivel asentamiento (uno de cada por asentamiento, designados por el Gobernador salvo este)
- **Gobernador**: ELECTO por ciudadanos, máxima autoridad del asentamiento, designa al resto de cargos. Ve y controla la cola de auto-construcción (ver Doc 4.2).
- **Tesorero**: gestión económica completa (trueque + Mercado).
- **General**: mando militar del asentamiento.
- **Maestro de Obras**: gestiona prioridades de auto-construcción, bonus a tiempos de construcción. Puede reordenar, añadir y quitar proyectos de la cola — nunca elegir su ubicación (ver Doc 4.2).
- **Sacerdote**: acelerador de aparición de Nobleza (no requisito) + bonificación de felicidad.

### Reglas generales de cargos
- Un jugador PUEDE ejercer varios cargos a la vez.
- Cargo se LIBERA tras 1 semana de inactividad del jugador que lo ocupa.
- PENDIENTE: mecanismo exacto de sucesión de un Rey-por-vasallaje si abandona el juego.

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
- Obtención: (1) fundar un asentamiento — cada fundador recibe automáticamente una casa en el asentamiento recién fundado, ocupando un espacio del mismo cupo de vivienda que (2), y con ella ciudadanía inmediata; (2) comprar una casa en un asentamiento de la propia Facción (espacios limitados según nivel/tamaño). El cupo base de un asentamiento recién fundado es de 5 casas — igual al máximo de jugadores que pueden fundar juntos (Doc 1.2/1.3) — así que siempre hay sitio para todos los fundadores; si fundan menos de 5, los espacios restantes quedan libres para que otros jugadores compren casa después (p. ej. 2 fundadores dejan 3/5 libres).
- Beneficios: ejercer cargo, iniciar caravanas en Mercados de la Facción, votar políticas, reclutar tropas (solo donde la ciudadanía lo permite), comisiones de comercio más bajas dentro de la misma Facción.
- PENDIENTE: residencia en cualquier asentamiento de la Facción, protección militar, voz en política exterior; nivel intermedio de comisiones para Facciones aliadas/vasallas.

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

**Requisitos (distintos por gremio, misma naturaleza de 3 ejes):**
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
