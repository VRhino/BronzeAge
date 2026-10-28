# Misiones recurrentes y tablón de misiones

> **Diseño acordado el 2026-10-28 con el autor. Sin implementar** (vive en `Docs/Disenos_Por_Implementar/` hasta que se implemente; ver su README). Amplía las misiones de hoy (`Docs/Mechanics/Misiones.md`, mecánica 49). Las cifras (límites, cuántas a la vez, recompensas) son un borrador a calibrar. Entrada 55 de `Docs/Mecanicas a desarrollar.md`.

## 1. Qué cambia

Hoy una misión es de un dueño (héroe, plaza o Facción), es de una sola vez y se activa sola cuando se cumple su previa. Con esto, cada misión pasa a tener:

| Campo | Valores | Hoy |
|---|---|---|
| Ámbito | héroe, plaza, Facción | existe (el dueño) |
| Recurrencia | única o recurrente | solo única |
| Duración | 1 día o 7 días (solo las recurrentes) | — |
| Tipo | individual o grupal | — |
| Estado | disponible, aceptada, cumplida, reclamada, caducada | activa, cumplida, reclamada |
| Quién la acepta | cualquiera del ámbito o solo un cargo concreto | — |
| Texto | título, qué hacer y una explicación libre más larga | título y qué hacer |

Las misiones **únicas** siguen como hoy (con previa, cadenas y sin caducidad). Ejemplos de únicas de plaza: fundar un pueblo extractor, reclutar una tropa de un escalón cuando esté disponible. Ejemplos de recurrentes: defender pueblos, escoltar caravanas, participar en batallas.

## 2. Individual y grupal

- **Individual**: un héroe del ámbito la **acepta** y queda asignada a él; solo cuenta lo que hace ese héroe. Varios héroes pueden aceptar la misma misión, cada uno la suya.
  - **Límite semanal por tipo**: cada misión individual tiene un tope de aceptaciones por semana en ese ámbito (por ejemplo, «escoltar caravanas»: 20 por semana). El tope se reinicia con la renovación semanal (§3), también para las diarias.
  - Una aceptada que no se cumple antes de que termine su duración queda **caducada**.
- **Grupal**: no se acepta. Cuenta lo que hacen **todos** los héroes del ámbito. Al cumplirse, la recompensa de participación se reparte **a partes iguales** entre quienes aportaron algo.

## 3. Reloj: diarias y semanales

Hay un solo reloj, el del servidor, igual para todos los jugadores estén donde estén.

- Las **diarias** se renuevan cada día a las **00:00 UTC**.
- Las **semanales** se renuevan cada 7 días, a las 00:00 UTC, contando desde el primer día de la partida.
- Al renovarse, las del periodo que acaba caducan (las cumplidas por reclamar se pueden seguir reclamando) y salen las del siguiente.
- El cliente muestra la cuenta atrás hasta la renovación y la hora local.

(UTC no tiene cambios de hora; la hora de España sí, dos veces al año.)

## 4. El tablón de la plaza

Cada plaza tiene un **tablón** que ven todos sus residentes.

- **Catálogo por defecto** de recurrentes de plaza (por ejemplo, 20), de las que solo **7 a la vez** están en el tablón.
- Por defecto salen las 7 primeras del catálogo.
- El **Gobernador** elige qué 7 saldrán en la próxima renovación y puede cambiar su elección durante el periodo. Las que están en el tablón no cambian hasta la siguiente renovación a las 00:00; entonces salen las 7 elegidas.
- Algunas misiones solo las pueden aceptar ciertos **cargos** (por ejemplo, el Maestro de Obras).
- Las misiones del tablón **sustituyen** a las 11 misiones de plaza de hoy (A1-A11), que pasan al catálogo como únicas o se reescriben.

## 5. Recompensas

- Las misiones de **Facción** dan experiencia de Facción.
- Las del **tablón** de una plaza dan experiencia a la Facción y participación al héroe que la cumple ([`Participacion.md`](Participacion.md)).
- Las de **héroe** dan lo que diga su entrada en el catálogo (oro u otra recompensa del catálogo editable, §6).

## 6. Catálogo editable en caliente

El **contenido** de las misiones (el catálogo: cuáles hay, sus condiciones, umbrales, límites, recompensas y textos) se carga desde un fichero de datos externo al código del motor. Una herramienta de administración lo cambia **en caliente**, sin reiniciar el servidor ni tocar el código.

- El **funcionamiento** (estados, aceptar, contar, caducar, renovar, repartir) y los tiempos siguen en el motor.
- Cada cambio de catálogo entra en el motor como un **evento con su instante**, de modo que una partida se puede reproducir igual desde el diario.
- Un cambio no toca misiones ya aceptadas ni cumplidas: vale para las que salen desde la siguiente renovación.

## 7. Por decidir al implementar

- Las cifras: tope semanal de cada individual, recompensas, cuántas recurrentes en cada catálogo por defecto.
- Si también hay tablón de Facción (recurrentes de Facción) con un rol que elija, como el Gobernador en la plaza (propuesta: el Rey).
- El formato del fichero del catálogo y qué herramienta lo edita (el admin web o el panel).
