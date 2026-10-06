# Ideas a diseñar

Ideas **aparcadas a propósito**: no se van a desarrollar ahora, pero no se quieren perder. No tienen plan ni código. Cuando
una se retoma, pasa a `Docs/Mecanicas a desarrollar.md` (con su diseño) o se descarta aquí. Los números de entrada se conservan
de `Mecanicas a desarrollar.md`. Aparcadas el 2026-10-05 (la cultura, el 2026-10-06).

| # | Idea |
|---|---|
| 9 | Eventos de asentamiento |
| 11 | Progresión de Liderazgo del jugador |
| 19 | El mapa político como entidad |
| 21 | Los 4 gremios escasos a nivel de servidor |
| 23 | Materiales exóticos |
| 24 | Ciclo de servidor de 12 meses + Maravilla + legado NPC (pospuesto con las Eras IV-V) |
| 43 | Cultura del asentamiento (y audio) |

## 9. Eventos de asentamiento

Los asentamientos tienen eventos propios como, por ejemplo, ser sitiados por bandidos (y los jugadores tienen
cierta cantidad de horas para formular la defensa y jugar la defensa). Pensar en otro tipo de eventos que
mantengan entretenido el juego.

*Base ya disponible:* los campamentos de bandidos existen y atacan caravanas cada tick
(`engine/bandidos.ts`), pero nunca asedian un asentamiento.

*Incluye:* **NPCs hostiles más allá de los bandidos** — hoy los bandidos son la única amenaza no-jugador del
mundo. Fauna peligrosa, incursores estacionales, u otros agresores ambientales caben aquí.

## 11. Progresión de Liderazgo del jugador

`Jugador` tiene hoy `liderazgoBase` y nada más. El propio código lo anota: *"el efectivo es base + progresión,
pero la progresión todavía no está diseñada, así que hoy coinciden"* (`domain/types.ts`, `constants.ts` §1587).
Un jugador sin registro usa `LIDERAZGO.base`.

Falta decidir qué hace subir el liderazgo (combatir, ganar, tiempo al mando, cargo militar…) y con qué curva.
Los **escuadrones** progresan por nivel y experiencia (Doc 5.16.3), sin cambiar nunca de tropa (Doc 5.8); el
motor aún usa veteranía (§31). La mecánica de Liderazgo ya admite un efectivo > base sin tocar nada — solo falta la
fuente.

## 19. El mapa político como entidad

**Estado: idea, sin diseñar.** Las fronteras "de dónde a dónde llegan" como una entidad de primera clase, no
algo que se recalcula al vuelo. Hoy las zonas de influencia y las fronteras ajenas se derivan
(`engine/zones.ts`, fronteras ajenas 2026-09-05); no existe un "mapa político" consultable como objeto.

## 43. Cultura del asentamiento

**Estado: `código: ✘` — solo decisiones tomadas en BA-006, sin documento de canon ni una línea de código.** Hasta el
2026-10-05 estaba repartida entre `Consideraciones/BA-006_Revision_Tecnologia_Eras.md` (sección «Culturas», D12-D27),
Doc 4 (Sala del Consejo) y Doc 6.1 (roster universal) y no figuraba en esta lista. Se separó de la identidad visual del
sigilo (`Consideraciones/Identidad_Visual_Definicion.md`, cerrada): **la cultura es del asentamiento y de sus edificios, y no afecta a los sigilos ni a los estandartes de la
Facción.**

**Lo ya decidido** (BA-006, que sigue siendo su fuente hasta que exista el documento propio):
- **D12** — La cultura es del ASENTAMIENTO: una puntuación por cultura, por debajo, que el jugador no elige ni
  modifica directamente; la moldean sus decisiones. **D27**: la Facción no tiene cultura de conjunto.
- **D13 / Doc 6.1** — El roster es universal: la cultura no da ni quita tropas ni tecnologías, solo aspecto.
- **D14** — Suman puntos a la cultura de un asentamiento: los soldados reclutados allí de tropas propias de una
  cultura (las reposiciones cuentan; las neutras no suman); las tropas más usadas en batalla por los héroes
  residentes (D20); políticas propias de cada cultura (con beneficio); las tecnologías que adopta la Facción. A
  estudiar: marcadores del mapa (costa, bioma, cercanía a elementos del generador).
- **D15** — Eras y cultura son independientes; el generador de mapas no asigna cultura.
- **D16 / D21** — Cada edificio se construye con el estilo de la cultura dominante en ese momento y lo conserva;
  mejorar un edificio (nivel interno) le da el estilo de la cultura dominante del momento. **D17**: manda la
  cultura con más puntos.
- **D18** — De momento la cultura es SOLO aspecto visual, sin efecto mecánico propio (las políticas culturales sí
  dan su beneficio).
- **D19** — Al fundar, el asentamiento es NEUTRO, sin puntos (neutro también tiene estilo visual). Al conquistarlo
  no cambia: conserva sus puntos.
- **D20** — «Miembros» de un asentamiento son sus héroes residentes. **D22** — Las escuadras no cambian de aspecto
  por cultura: solo los edificios.
- **Catálogo propuesto (sin confirmar):** Neutra, Micénica, Hitita/Anatolia, Egipcia, Mesopotámica,
  Fenicia/Levantina, Helénica (incluye Macedonia y Tracia, 2026-09-16), Persa.
- Las tropas llevan afinidad cultural (las de nombre histórico ya la llevan: roster de BA-006). La Sala del Consejo
  (Doc 4) toma el aspecto de la cultura dominante; su nombre es neutro.
- **Campamentos de mercenarios**: la nota D26 original (tropas con cultura propia, puntos a la plaza donde reside el
  héroe) quedó superada el 2026-10-02 (§40 y canon Doc 1.9b/5.8/6.5b): sin roster ni cultura propios.

**Pendiente de diseñar:**
- **Documento de canon propio** (D23) y confirmar el catálogo.
- Afinidad cultural de cada tecnología, y a qué asentamientos suman sus puntos al adoptarla la Facción.
- Catálogo de políticas culturales y si las tiene cualquiera o se desbloquean con puntos.
- Desempate cuando dos culturas tienen los mismos puntos; si los puntos decaen con el tiempo; si el estilo de un
  edificio salta de golpe al cambiar la cultura dominante o hay histéresis (**salió del consejo**, 2026-10-05).
- Qué pasa con los edificios ya construidos cuando la cultura dominante cambia (D16 dice que conservan el estilo).
- **Contrato con Conquest**: cada `Edificio` llevaría su estilo (Doc 01 §17, `visualSeed`/`visualCatalogVersion`);
  propuesta CQ para los modelos por cultura. Id de cultura dominante por asentamiento, estable en el contrato.
- **Referencia estética concreta** de cada cultura (paleta, motivos de edificio) — antes `Preguntas_Abiertas` §9.
- **Audio** (sacado de la identidad visual): música por cultura, música ambiental que refleje la cultura dominante del asentamiento y
  sonidos por evento de juego. Es trabajo de los clientes; este repo solo garantizaría ids estables de cultura y de
  evento, con una lista cerrada y un test de contrato.

## 21. Los 4 gremios escasos a nivel de servidor

**Estado: diseño parcial en el canon (Doc 2.10), `código: ✘` — nada.** Los 4 gremios (Comerciantes,
Artesanos, Constructores, Ladrones), escasos a nivel de servidor, con una tirada periódica sujeta a cuatro
requisitos simultáneos (reputación > 90, título de servidor específico, nivel de asentamiento en el máximo,
mantenimiento > 90 %), y su mecanismo de pérdida. El `patioDeGremios` de `constants.ts` es solo una parcela
decorativa del trazado urbano, sin relación con esta mecánica.

Pendiente de decidir (Preguntas_Abiertas §14): valores numéricos de cada requisito por gremio, el título de
servidor asociado a cada uno, el detalle de beneficios de Comerciantes/Artesanos/Constructores, y si hay
margen de gracia antes de perder el gremio.

## 23. Materiales exóticos

**Estado: `código: ✘` — no existe ese tipo de recurso.** Los pide el diseño de la Maravilla (Doc 6 / §24) y
posiblemente el catálogo de commodities de Nobleza. Hoy la Maravilla se paga con un coste placeholder de
recursos ya existentes. Falta: qué materiales son, de dónde salen (¿nodos raros? ¿bioma? ¿solo comercio de
larga distancia?), y qué los consume además de la Maravilla.

## 24. Ciclo de servidor de 12 meses + Maravilla + legado NPC

**Estado: el EDIFICIO Maravilla implementado (único, nivel de asentamiento máximo, vía cola manual, coste
placeholder); el CICLO no.** Diseño cerrado en `Consideraciones/Roadmap_Escalado.md` Eje 4 y
`Preguntas_Abiertas.md` §14d: 12 meses de servidor, cierre anticipado por la primera Facción que complete la
Maravilla, y la Facción ganadora persiste como Facción-legado NPC de solo mantenimiento y comercio.

Requiere infraestructura de servidor / multi-instancia (reset, generación del nuevo mapa, destino de las
Facciones no ganadoras, si la legado es atacable). Nada de eso tiene código. Ver la lista completa en el
Roadmap.
