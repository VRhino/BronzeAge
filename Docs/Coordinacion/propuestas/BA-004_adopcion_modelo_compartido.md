# BA-004 — Revisión y adopción del modelo compartido de entidades v0

**Destino:** `BronzeAgeFase0`  
**Estado:** ACEPTADO (revisión 2026-09-11, corregido con la revisión de Codex del mismo día — ver abajo)  
**Prioridad:** Alta  
**Documento origen:** `Conquest_prototype/Docs/Coordinacion/03_Modelo_compartido_entidades_v0.md`

## Solicitud

Revisar el inventario y las decisiones propuestas en el documento origen desde la perspectiva del dominio de
BronzeAge. En particular, confirmar o corregir:

- ámbitos y cardinalidad de `userId`, `playerId`, `heroId` y `squadId`;
- propiedad de escuadras por jugador y asignación por loadout;
- escuadra agregada persistente frente a unidad individual temporal;
- campos del asentamiento necesarios para reconstrucción 3D y asedio;
- separación entre entidad persistente, DTO de red y entidad ECS;
- lista de entidades existentes que el inventario haya omitido.

## Resultado esperado de la revisión

1. Comentarios o estado actualizado en esta propuesta.
2. Decisiones abiertas respondidas o marcadas para decisión humana.
3. Una ubicación propuesta dentro de BronzeAge para schemas y fixtures canónicos.
4. Una secuencia de implementación que no rompa snapshots actuales ni la API del cliente existente.

Codex no solicita todavía código del backend. Primero hace falta acordar el modelo y publicar una versión de
contrato que pueda probarse desde C#.

## Revisión desde BronzeAge (2026-09-11)

Contrastado contra `src/domain/types.ts`, `src/acceso/tipos.ts` y
`Docs/Arquitectura/5_Contratos_Identidad_Permisos.md` (código y diseño real, no solo intención).

### 1. Ámbitos y cardinalidad de `userId`, `playerId`, `heroId`, `squadId`

- **`userId`/`playerId` — CONFIRMADO, ya implementado.** `Usuario.id` (userId) y `Jugador.id` (playerId,
  ámbito `gameId`) existen exactamente como los propone el modelo compartido: un `Usuario` tiene como máximo
  un `Jugador` por `gameId`, y el mismo `Usuario` puede jugar varias partidas en paralelo, cada una con su
  propio `Jugador` (Doc 5, sección "Jugador"). Nada que migrar de este lado.
- **`heroId` — CONFIRMADO por decisión de producto del usuario (2026-09-11).** `Heroe` se crea en BronzeAge
  con toda su estructura proveniente de Conquest, acotada a una lista cerrada (ver "Héroe v1 mínimo" abajo,
  corrección de Codex). Cardinalidad: como máximo un `Heroe` por `jugadorId`/`gameId`. **Corrección de Codex
  incorporada:** NO se crea una entidad `Jugador` nueva en `domain/` para sostener esto — duplicaría
  `Membresia` (`acceso/tipos.ts`), que ya une `usuarioId`+`gameId`+`jugadorId`+rol con vigencia. `Heroe`
  lleva `jugadorId` como dueño uno-a-uno (mismo valor que `Membresia.jugadorId`), y la unicidad "un héroe por
  jugador y mundo" se valida por unicidad de `Heroe.jugadorId` dentro de la partida, no por una entidad de
  acceso adicional. Todo el estado que hoy carga `domain.Jugador` (`liderazgoBase`, `ubicacion`,
  `plazasRecordadas`, `exploracionPersonal`) se muda a `Heroe`; `GameSessionState.jugadores` se reemplaza por
  `heroes`. Los comandos resuelven `Sesion -> Usuario -> Membresia -> jugadorId -> Heroe`; las reglas de
  juego reciben `heroeId`, auditoría/autorización conservan también `usuarioId`/`jugadorId`. Detalle completo
  en BA-002.
- **`squadId` — CONFIRMADO con una corrección de identidad (Codex).** `Escuadron.id` ya existe; su dueño
  pasa de `jugadorId` a `heroeId`. La primera lectura de esta revisión confundía ubicación con identidad al
  decir que el ámbito de `squadId` era "dentro del contenedor" — es único y estable dentro de `gameId` con
  independencia de en qué contenedor esté en cada momento (`Asentamiento.escuadrones`/`Ejercito.escuadrones`/
  escolta/reserva de batalla), con invariante de exclusividad: solo en uno a la vez. Unicidad
  (decisión del usuario, 2026-09-13): como mucho una `Escuadron` por (`heroeId`, `tropaId`) en toda la
  partida, comprobada contra todas las escuadras del héroe y no solo contra la lista del asentamiento — ver
  `01_Modelo_de_datos_compartido.md` §13.

### 2. Propiedad de escuadras por jugador y asignación por loadout

Propiedad: confirmada, pero el dueño es `Heroe`, no `Jugador` (ver punto 1 — decisión de producto que
corrige la lectura original de esta revisión). **Loadout — CONFIRMADO, se trae de Conquest tal cual
(decisión de producto del usuario, 2026-09-11).** Es la preselección de escuadrones de un `Heroe` para salir
a mundo abierto o unirse a un `Ejercito`; encaja bien porque el límite que ya la restringe —liderazgo— ya
existe en BronzeAge (`liderazgoBase`, hoy en `Jugador`, se muda a `Heroe` con el resto de su estado). No es
un concepto nuevo de reglas, es una mejora de calidad de vida ya resuelta en Conquest que solo falta traer.

### 3. Escuadra agregada persistente frente a unidad individual temporal

Confirmado sin reservas — es exactamente como ya funciona BronzeAge. `Escuadron.cantidad` es un contador
agregado; no hay unidades individuales persistentes. El servidor de batalla Unity puede generar unidades
efímeras a partir de `BattleSquadSnapshot.cantidad` sin que BronzeAge necesite modelarlas — coincide con la
propuesta del doc origen §5 ("Unidad individual… no es persistente en v1").

### 4. Campos del asentamiento para reconstrucción 3D y asedio

Ya existen y son reutilizables directamente, sin inventar un segundo modelo:

- `Edificio`: `id`, `tipo`, `posicion` (local al espacio del asentamiento, origen en el Centro Urbano),
  `estado`, `nivelInterno`, `rotado` (orientación intercambiable ancho↔alto), `ambito`
  (`'asentamiento'|'mapa'`).
- Footprint por tipo/nivel: `EDIFICIO_TAMANO` / `EDIFICIO_CATALOGO[tipo].niveles[n].tamano` (`ancho`×`alto`
  en celdas, `constants.ts`) — la única huella que cambia con el nivel es Granja (1×1→6×6 según nivel
  interno); el resto es fija por tipo.
- `Recinto`/`CeldaMuro`: anillo de celdas (`col`, `row`, `clase: 'muro'|'puerta'|'torre'`) **en orden de
  recorrido**, con `avance` (progreso de construcción) y `mejorandoA` (mejora en curso). Las puertas se
  fijan para siempre al trazar el recinto — esto ya cubre "identidades y coordenadas estables" que pide
  `02_Arquitectura_objetivo.md` para murallas/puertas.
- Falta (campo nuevo, no ruptura): `visualSeed`/`visualCatalogVersion` que propone el doc §8 para elegir
  variante 3D determinística por `buildingId` — no existe todavía, se añade como opcional.
- Falta definir explícitamente: sistema de coordenadas y factor de conversión celda→unidad Unity (pedido en
  los criterios de aceptación de BA-003).

### 5. Separación entidad persistente / DTO de red / entidad ECS

Principio correcto y ya coincide con cómo está construido el motor: `domain/types.ts` es interno del
backend. **No existe todavía una capa DTO** pensada para un cliente ajeno — hoy `server/`/`GameStore` sirven
el estado casi directo. Adoptar el modelo implica **crear esa capa** (trabajo nuevo), no arreglar algo roto.
Ver ubicación propuesta abajo.

### 6. Entidades existentes que el inventario del doc origen omite

- `Recinto`/`CeldaMuro` con su construcción incremental (`avance`/`mejorandoA`) — el doc origen menciona
  `enclosureId` de forma genérica pero no captura que las murallas se levantan celda a celda y las puertas
  quedan congeladas para siempre.
- `CargosAsentamiento` (gobernador/tesorero/general/maestroObras/sacerdote) y los cargos de Facción
  (`reyId`/`embajadorId`) — dominio político, ausente del grafo conceptual §2.
- `Usuario`/`Sesion`/`Membresia` (capa `acceso/`, Doc 5) — es la base real de `userId`/`playerId` y no
  aparece en el grafo, aunque el doc los usa como si ya estuvieran ahí.
- `Titulo` (prestigio, derivado) y `ZonaInfluencia`/`ZonaFaccion` (territorio derivado) — relevantes solo si
  Unity dibuja fronteras; no forman parte del contrato de batalla.
- El resto del inventario del mundo estratégico (§7/§9) está correcto y completo.

## Modelo de Héroe y Escuadra v1 (cerrado, revisión Codex 2026-09-11)

"Traer toda la estructura de Conquest" no es una lista cerrada por sí sola — Codex fijó una antes de que se
programe nada, para no descubrir el alcance a mitad de implementación:

**`Heroe`:** `id`, `jugadorId` (dueño, ver arriba), `displayName`, `classDefinitionId`, género, avatar;
nivel, XP, puntos sin gastar, atributos base; perks desbloqueados; `liderazgoBase`, `ubicacion`,
`plazasRecordadas`, `exploracionPersonal`, `injuredUntil` (el `heridoHasta` movido desde `Escuadron`);
loadouts por ID; inventario por `itemInstanceId` y equipamiento por referencia. No se persisten caches ni
atributos calculados. Las monedas de Conquest se mantienen con nombres explícitos, separadas del oro/
recursos estratégicos (decisión #4 abajo).

**`Escuadron`:** `id`, `heroeId` (dueño), `squadDefinitionId`/`tropaId`, nombre; `cantidad`, nivel y XP
(reemplaza `veterania`, decisión #2); moral; habilidades y formaciones desbloqueadas; formación seleccionada;
ubicación/reserva estratégica. Las bajas reducen `cantidad` de forma permanente — no existe `herido` para
escuadra (decisión #3). El adaptador de Conquest debe dejar de interpretar `SquadInstanceData.unitsInjured`
como resultado persistente compartido: eso es ruido de UI de Conquest, no un hecho que BronzeAge persista.

El loadout referencia `squadId` (no una copia de la escuadra); `totalLeadership` del loadout se
calcula/valida en BronzeAge y nunca se acepta como verdad enviada por el cliente.

## Decisiones abiertas — resueltas o marcadas para decisión humana

1. **¿Se crea `Heroe` como entidad nueva?** **Resuelto — sí, decisión de producto confirmada por el usuario
   (2026-09-11).** Ámbito `gameId`, uno por `Jugador`/mundo, nunca cuenta global ni varios por mundo. Sin
   cambio de héroe dentro de una partida (no hace falta diseñar sustitución, al ser único por mundo).
2. **Veteranía BronzeAge vs nivel/XP Conquest — RESUELTO, decisión del usuario (2026-09-11): gana
   Conquest.** El modelo de nivel/XP es más completo; `Escuadron.veterania` (un solo número que sube
   combatiendo) se **reemplaza** por nivel + experiencia al estilo Conquest. **Actualizado 2026-09-13
   (decisión del usuario, sustituye lo que decía aquí):** la XP ganada la calcula el servidor de batalla
   Unity, porque depende del desempeño (bajas, héroes abatidos, capturas, daño, MVP, puesto en la tabla) y
   eso solo lo ve Unity. Llega como delta en el `BattleResult`; BronzeAge la valida, la suma y aplica la
   curva de nivel. Detalle en `01_Modelo_de_datos_compartido.md` §15.
3. **Herido vs muerto — RESUELTO, decisión del usuario (2026-09-11): gana BronzeAge.** El `heridoHasta` que
   mencionaba la revisión anterior es un estado que debe aplicar al **héroe** (que ahora sí combate
   directamente en la partida real de Unity), no a la escuadra. **Las escuadras no tienen estado de
   "herida"**: una baja de escuadra es directamente una muerte permanente (reduce `cantidad`), como ya
   funciona hoy — Conquest no impone su distinción herido/muerto sobre las escuadras. El `BattleResult`
   reporta por escuadra solo supervivientes/muertos (sin heridos), y por separado puede indicar si el héroe
   de un lado quedó herido (debuff temporal, análogo al actual `heridoHasta` pero movido de `Escuadron` a
   `Heroe`). Duración: 2 minutos de mundo (decisión del usuario, 2026-09-13).
4. **Economía/monedas de héroe.** `Heroe` sí existe (decisión #1), así que sí hay que resolverlo: las
   monedas de héroe de Conquest (`bronze/silver/gold` de `HeroData`) y el oro de Facción/asentamiento de
   BronzeAge se mantienen como economías DISTINTAS, sin fusión automática por nombre — mismo criterio que ya
   marca el doc origen §6. Pendiente de decisión humana solo si en el futuro se quiere puentear una con otra.
5. **Equipo de escuadra agregado o por pieza.** BronzeAge no modela equipo de escuadra hoy — el poder viene
   de `tropaId`/catálogo versionado. Recomendación: mantenerlo agregado, no introducir inventario de pieza
   que no existe. Decisión humana solo si se quiere diferenciar escuadras del mismo `tropaId` por equipo.
6. **Reglas tácticas auditables sin reejecutar la batalla.** Cubierto por el `BattleResult` de BA-001:
   efectivos iniciales/finales por escuadra, ganador, objetivos, duración.
7. **Política de héroe activo/sustitución.** No aplica — al ser un solo héroe por mundo no hay sustitución
   que diseñar (ver decisión #1). La sucesión de liderazgo de `Ejercito` sigue igual (`liderId` pasa al
   participante más antiguo), solo que el participante ahora se identifica por `heroeId`.
8. **Formato de worldgen 3D y mapas de asedio.** Sigue abierto — es el alcance de BA-003, ver esa propuesta.

## Ubicación propuesta para schemas y fixtures canónicos

Nuevo directorio de contrato versionado, separado de `domain/` (que sigue siendo interno) y de
`Docs/Coordinacion/` (que es solo para propuestas de coordinación, no para el contrato real con tests):

```text
src/contratos/v1/
  *.schema.json       — JSON Schema versionado (identidad, escuadra, asentamiento, batalla)
  fixtures/*.json      — fixtures dorados, uno por entidad de contrato
  dto.ts                — tipos TS derivados, consumidos por server/ al serializar
```

Cumple la regla §5.8 del modelo de cooperación: "cada contrato publicado incluye al menos un fixture JSON
válido y una prueba que lo lea en ambos lados."

## Secuencia de implementación

**Sin migración ni retrocompatibilidad de snapshots — decisión del usuario (2026-09-11): las partidas
actuales no son relevantes para el producto final.** Si un snapshot viejo no carga con el modelo nuevo, se
descarta y se crea uno nuevo.

**Corrección de Codex incorporada — "sin migrar snapshots" no es lo mismo que "sin contrato antes de
tocar código".** La sustitución `jugadorId -> heroeId` afecta al menos decenas de módulos. Antes de
renombrar mecánicamente hace falta, en este orden:

0. Un ADR/contrato aceptado del flujo de identidad `Membresia -> Heroe` (ya escrito arriba en esta
   propuesta, falta solo el documento formal si el proceso lo exige).
1. DTO/schema v1 exacto para `Heroe`, `Escuadron` y `Loadout` (las listas cerradas de la sección anterior).
2. Fixtures dorados de cada uno.
3. Tests de unicidad (`Heroe.jugadorId` único por partida) y de propiedad (`Escuadron.heroeId` resuelve al
   héroe correcto).
4. Estrategia de corte de API explícita — aunque no haya que migrar snapshots viejos, sí hay que decidir el
   momento en que `server/`/`GameStore` dejan de aceptar `jugadorId` como dueño y empiezan a exigir
   `heroeId`, para no descubrir después que la capa de autorización perdió la identidad de cuenta que
   necesita para auditar.

Solo entonces:

5. Crear `Heroe` en `domain/types.ts` con los campos que hoy tiene `Jugador` (`liderazgoBase`, `ubicacion`,
   `plazasRecordadas`, `exploracionPersonal`) más los nuevos de Conquest (lista cerrada arriba).
6. Renombrar el campo dueño en `Escuadron`, `Ejercito.participantes`/`liderId`, `CargosAsentamiento.*Id`,
   `Faccion.reyId`/`embajadorId`/`ciudadanosIds` y `Asentamiento.jugadoresFundadoresIds`/`casasCompradas` de
   `jugadorId` a `heroeId`. Reemplazar `Escuadron.veterania` por nivel/XP (decisión #2) y eliminar
   `Escuadron.heridoHasta` (decisión #3 — ese debuff pasa a vivir en `Heroe`).
7. Crear `src/contratos/v1/` con los DTO/fixtures del paso 1-2 ya escritos.
8. Publicar DTO de `Asentamiento` como proyección de solo lectura derivada de `domain` (no una segunda
   fuente de verdad).
9. Añadir campos nuevos a `domain` (`visualSeed`, `visualCatalogVersion`) para variantes visuales 3D.
10. Implementar `Battle`/`BattleTicket`/`BattleResult` (BA-001) sobre los DTO anteriores, con `heroeId` como
    identidad de participante — detalle completo en la revisión de BA-001.

