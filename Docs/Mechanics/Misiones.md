# Misiones

> **Diseño acordado el 2026-10-26 con el autor. Sin implementar.** Las recompensas y los umbrales son placeholders que se calibran con los batches y los bots. Contexto: `Docs/Mecanicas a desarrollar.md` entrada 49.

## 1. Para qué

Guiar al jugador al principio del juego para que vaya descubriendo sus mecánicas, con recompensas. Tres dueños: **individuales** (el héroe), **de Facción** y **de asentamiento**. Valen igual para bots (principio de `12_NPC_Fuera_Del_Motor.md` §2) y les dan un currículo ya hecho.

## 2. Qué se reutiliza

- Los **contadores en origen** de los logros de tecnología (`Tecnologia_Aedas.md` §2.1, 6B): cada sitio que produce el hecho suma al ocurrir, sin barrer y sin contar dos veces; aquí por dueño.
- Las **condiciones de estado** de los hitos (`CondicionHito`: edificio, tecnología, recurso en la capital, nivel): se comprueban al ocurrir el hecho que puede cumplirlas, nunca por minuto.
- Las **recompensas** ya existentes: oro al almacén personal del héroe, recursos al almacén de la plaza, experiencia de Facción (`NIVEL_FACCION.xp`).

## 3. Modelo

- Un **catálogo** en un bloque de parámetros nuevo (`MISIONES`, a crear al implementar): id, dueño (`heroe` | `faccion` | `asentamiento`), condición (contador o estado), recompensa, y la misión previa que la desbloquea (**cadenas**).
- Por dueño solo se guardan los contadores de las misiones **activas**; una misión cumplida queda fija.
- La recompensa se **reclama** con una orden (`ReclamarRecompensa`): así un almacén lleno no se la come y el cliente tiene su momento de «recompensa».
- Una vez por dueño, sin misiones repetibles al principio; solo cuentan objetivos ajenos donde corresponde (trueques y alianzas con otras Facciones).

## 4. Recompensas (decidido)

| Dueño | Recompensa |
|---|---|
| Héroe | oro, a su almacén personal |
| Asentamiento | recursos, a su almacén |
| Facción | experiencia de Facción |

Los bots las cobran igual; las recompensas meten recursos gratis en los batches y se calibran allí.

## 5. Ejemplos, contra hechos que el núcleo ya emite

| Misión | Dueño | Se cumple con |
|---|---|---|
| Fundar la aldea agrícola (al fundar el asentamiento) | asentamiento | `pueblo.completado` de tipo aldea ([`Aldea_Agricola_Y_Cria.md`](Aldea_Agricola_Y_Cria.md)) |
| Fundar asentamientos | Facción | la fundación de una plaza nueva |
| Conquistar asentamientos | Facción | `combate.asedio_conquista` o `batalla.aplicada` de asedio ganado |
| Cerrar trueques | Facción | trueque cumplido con otra Facción (los de la misma ya no dan XP ni reputación) |
| Ganar aliados | Facción | relación de alianza aceptada |
| Reclutar ciertas tropas | héroe | `reclutado` en plaza o `mercenarios.reclutado`, por tropa |

El catálogo completo de misiones está por escribir.

## 6. Qué toca en el motor (plan, sin hacer)

- Estado y foto nueva (hoy `BAN40`); orden `ReclamarRecompensa` tras la 178 con sus rechazos y su fila en la matriz de Proyecciones §5.1; hechos `mision.*`.
- Cada misión viaja en su ámbito (`HEROE`, `FACCION`, `ASENTAMIENTO`).
- Cliente: panel de misiones. Bots: seguir las misiones como guía.
- Encaja con la idea aparcada 11 (progresión de Liderazgo del jugador): una misión podría ser su primera fuente.
- Fuera de alcance: misiones que un humano coloca para otros humanos (`Docs/Ideas a diseñar.md` entrada 52).
