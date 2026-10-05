# Aedas — decisiones y plan de implementación

**Reglas:** `Docs/Game/6_Sistema_de_Tecnologia_y_Aedas.md` §6.1, 6.4 y 6.7 (canon). Este documento solo guarda las
decisiones, el modelo propuesto y los pasos. Cerrado con el usuario el 2026-10-05; origen: `Docs/Mecanicas a
desarrollar.md` §42 (ya borrado al cerrarse), BA-006 (D31, D55).

## 1. Decisiones (2026-10-05)

| # | Decisión | Elegido | Alternativas descartadas |
|---|---|---|---|
| A1 | Retraso de la ventaja del primero | **24 horas** desde que alguien desbloquea la tecnología | 48 h, 72 h (igual que los campamentos de mercenarios, Doc 6.5b), 7 días |
| A2 | Cuántos itinerantes y cómo se mueven | Uno por cada 3 Facciones vivas (mín. 3); por la **red de caminos** (grafo de navegación); el exilio de puertas los deja fuera; se ven con la regla de avistamiento de ejércitos; ni atacables ni retenibles | Solo rutas de caravana (no hay Aedas sin comercio); al azar y atacables (abre robo/retención sin diseñar) |
| A3 | Venta | 2 × oro de la tarifa de adopción de su Era (200/600/1.200), Eras I-III, logro cumplido y Era abierta; paga el Rey o el Gobernador desde el almacén de la plaza; la adopción se paga igual | 3 × en todas las Eras; precio fijo solo para Eras I-II |
| A4 | Épica y modelo logro+hito (D30) | La épica es un **hito alternativo**: sustituye al hito, nunca al logro del servidor; sin coste de oro | Épica como vía exclusiva de tecnologías propias (obligaría a reabrir el catálogo) |
| A5 | Cupo de residentes | Nivel 2: 1, nivel 3: 2, nivel 4: 3, nivel 5: 4 (con Palacio y Nobleza); reputación ≤ −50 los atrae ×3 más despacio | 1/1/2/3 |

Decididas de paso (derivan de las anteriores y no estaban pedidas): llegada de un residente cada `llegadaHoras`;
los Aedas no combaten; el estado «revelada» de una tecnología (la Facción sabe qué le falta, aún no puede adoptarla);
la crónica publica un descubrimiento cuando el retraso ha pasado.

## 2. Qué toca en el código (propuesta)

- **`EstadoTecnologia`** (`domain/types.ts`): sin cambios en lo existente; `TecnologiasFaccion` gana `reveladas`
  (lo que un Aeda le mostró). `primeros[id].en + retraso` es cuándo la conocen los Aedas.
- **Entidad `Aeda`** (`domain/types.ts`): `{ id, clase: 'itinerante' | 'residente', … }`. Itinerante: posición, ruta
  por el grafo, destino y llegada, estancia hasta. Residente: `asentamientoId`, épica en curso
  (`tecnologiaId`, capítulo, progreso, último hecho contado). Vive en el estado de sesión, no en `Faccion` (viaja a
  todos los jugadores).
- **`engine/aedas.ts`** (motor puro, determinista con el RNG de la sesión): movimiento por caminos, difusión al llegar
  (`revelar`), compra (`comprarTecnologiaAeda`, reutiliza la aparición de `engine/tecnologia.ts`), residentes (llegada,
  marcha, cupo), épicas. `engine/tecnologia.ts` expone la aparición por una vía que se salta el hito.
- **`engine/cronica.ts`**: convierte los eventos públicos (`tecnologia.logro`, `era.comienza`, `titulo.*`,
  descubrimientos, caídas, guerras) en entradas persistentes del registro.
- **Contrato v1** (`src/contratos/v1/`): `Aeda` avistado en la proyección, `reveladas` en `ProyeccionJugador.tecnologia`,
  comando `comprarTecnologiaAeda`, consulta de la crónica. Aviso a Conquest / al cliente cuando se toque.
- **Constantes** (`constants.ts`, `AEDAS`, placeholders): `retrasoConocimientoHoras: 24`, `itinerantes`,
  `estanciaHoras`, `velocidad`, `factorVentaOro: 2`, `cupoResidentes`, `llegadaHoras`,
  `factorLlegadaReputacionBaja`, `umbralReputacionBaja: -50`, `epica`. Calibración en `Docs/Mecanicas a balancear.md`.

## 3. Pasos

Cada paso deja los tests en verde y se puede fusionar por separado.

1. **Itinerantes difusores + venta.** — **Hecho (2026-10-05).** Ver 3.1.
2. **Residentes con épica.** — **Hecho (2026-10-05).** Ver 3.2.
3. **Crónica y lore.** — **Hecho (2026-10-05).** Ver 3.3.

### 3.1 Paso 1, tal como quedó

- `engine/aedas.ts` (movimiento, sin RNG) y, en `engine/tecnologia.ts`, `revelarTecnologias`, `venderTecnologia` y
  `conocidaPorAedas`. `AedaItinerante` y `TecnologiasFaccion.reveladas?` en `domain/types.ts`. `AEDAS` en `constants.ts`.
- `GameSessionState.aedas?` y `EstadoSimulacion.aedas?` **opcionales**: los crea el tick y no hace falta subir
  `FORMATO_SNAPSHOT_VERSION` (el riesgo de checkpoints del §4 no se materializa en este paso).
- Comando `comprarTecnologiaAeda { asentamientoId, tecnologiaId }` (Rey o Gobernador de la plaza, presente; código de error
  `aedas.venta_invalida`). Eventos `aedas.revela` y `aedas.venta`, atribuidos a la plaza: solo los ve su Facción.
- Proyección del jugador: `aedasAvistados` (posición y plaza, con la regla de avistamiento de los ejércitos) y
  `tecnologia.reveladas` (quién la desbloqueó y el **hito completo**; los requisitos que faltan, ya filtrados, solo van en
  el mensaje del evento de revelación porque la proyección no tiene el mapa para evaluarlos).
- Puerta: nuevo grupo `aedas` en `GrupoPuerta` (el cierre por defecto no lo incluye). Primera versión: solo respetaba un cierre
  explícito a neutrales; la revisión de código lo cambió porque el resultado dependía de si el campo existía.
- Fuera del contrato v1: `ProyeccionJugador` no está en él y los Aedas no llegan a Conquest.
- Sin hacer: cliente de administración y NPC de bots (no compran).

### 3.2 Paso 2, tal como quedó (residentes y épica)

- `engine/aedasResidentes.ts`: ciclo de vida (`avanzarResidentes`, en el tick), épicas (`empezarEpica`, `abandonarEpica`,
  `avanzarEpicas`) y `hechosDeEventos`. Estado opcional `GameSessionState.aedasResidentes` (residentes, plazos de llegada y
  épicas cumplidas por Facción). Catálogo `EPICAS` en `constants.ts`: 24 épicas de plantilla (3/4/5 capítulos).
- Las épicas avanzan en `exito()` (`session/comandos/tipos.ts`), junto a los contadores de los logros: por ahí pasan los
  eventos del tick, de los comandos y del NPC. Los hechos salen de eventos que ya existían; no hizo falta enriquecer payloads.
- Comandos `empezarEpica` y `abandonarEpica` (Rey, Gobernador o Sacerdote de la plaza, presente; error `aedas.epica_invalida`).
  Eventos privados `aedas.residente_llega`, `_se_va`, `epica_empieza`, `_capitulo`, `_tecnologia`, `_abandonada`; público
  `aedas.epica_cumplida`. La proyección trae `aedasResidentes` de la Facción propia.
- Felicidad y nobleza: parámetro nuevo de `crecerPoblacion`. Título «Mecenas de los Aedas» en `calcularTitulos`.
- **Cambios frente al diseño:** se descartó el tope de una batalla por Facción y periodo: basta el enfriamiento de 6 horas por
  épica (con N residentes, N épicas a la vez). El Aeda de una plaza conquistada se queda con su Facción nueva sin épica. Las
  batallas de Unity cuentan solo si emiten los mismos eventos que el motor (`combate.encuentro`/`asedio_*`).

### 3.3 Paso 3, tal como quedó (crónica)

- `engine/cronica.ts`: vista derivada de eventos. Los ya públicos (`CODIGOS_CRONICA_PUBLICOS`) cuentan tal cual; de los demás
  (conquistas, ruinas, fundaciones, guerras, rebeliones, anexiones, fusiones, primera ciudad en nivel 3-5) se emite un
  evento público `cronica.entrada` desde `exito()`. Sin estado nuevo. `PayloadAsentamientoRuinas` gana `faccionId` y `nombre`.
- Nuevo `aedas.canta_descubrimiento` (tick de tecnología, con `EstadoTecnologia.cantadas?`) cuando pasa el retraso.
- `GET /jugador/partidas/:gameId/cronica?desde=&limite=`: las más recientes, orden cronológico, igual para todos. Sin vista
  en el cliente todavía (cliente de administración y BronzeAgeClient). Sin hecho de «paz» ni «guerra terminada»: no hay evento.

## 4. Riesgos

- **Contratos y snapshots**: entidad nueva en el estado → `FORMATO_SNAPSHOT_VERSION` sube y los checkpoints de batch
  dejan de valer; avisar a la sesión de balance.
- **Bots**: los bots (adaptador remoto, `npm run bots`) no compran a los Aedas en el paso 1; las Facciones NPC del
  motor ganan el estado «revelada» y la compra solo si el NPC lo implementa.
- **Eras IV-V**: la venta excluye las Eras IV-V; el catálogo de esas Eras sigue en `Docs/Mecanicas a desarrollar.md` §20.
