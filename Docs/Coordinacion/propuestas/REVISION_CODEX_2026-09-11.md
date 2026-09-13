# Revisión Codex de BA-001 a BA-004

**Fecha:** 2026-09-11  
**Autor:** Codex / Conquest  
**Alcance:** coherencia entre las cuatro propuestas y contraste con el código actual de BronzeAge.  
**Resultado:** diseño aceptable con correcciones obligatorias antes de implementar.

## Estado observado

| Propuesta | Estado escrito en el archivo | Resultado de esta revisión |
|---|---|---|
| BA-001 | ACEPTADO | Aceptable con correcciones de persistencia, seguridad y ciclo de estado |
| BA-002 | ACEPTADO | Aceptable con corrección de la frontera `Membresia`/`Jugador`/`Heroe` |
| BA-003 | LISTO_PARA_REVISION | No consta como aceptada; falta decisión humana sobre worldgen abierto |
| BA-004 | ACEPTADO | Aceptable como modelo conceptual; aún no define DTO v1 completo |

Los cuatro archivos están sin commit en el árbol observado. `ACEPTADO` significa diseño aprobado, no código
implementado: todavía no existen `Heroe`, `Batalla`, `src/contratos/v1`, `visualSeed` ni
`visualCatalogVersion` en `src/`.

## Correcciones obligatorias

### 1. No duplicar acceso dentro de `Jugador`

BA-002/BA-004 proponen dejar `Jugador` como vínculo `usuarioId + gameId + rol`. Esa forma ya existe en
`acceso/tipos.ts` como `Membresia`, que además tiene vigencia. Crear otra entidad con los mismos campos
duplicaría autoridad y contradice la separación actual entre `acceso/` y `domain/`.

Modelo recomendado:

- `Usuario`, `Sesion` y `Membresia` permanecen exclusivamente en `acceso/`.
- `Membresia.jugadorId` conserva el identificador de participación/cuenta dentro del mundo.
- `Heroe` vive en el dominio de juego y contiene `jugadorId` como dueño uno-a-uno.
- La restricción “un héroe por jugador y mundo” se valida por unicidad de `Heroe.jugadorId` dentro de la
  partida.
- Los comandos resuelven la cadena autenticada `Sesion -> Usuario -> Membresia -> jugadorId -> Heroe`.
- Las reglas de juego reciben `heroeId`; auditoría y autorización conservan también `usuarioId`/`jugadorId`.
- No hace falta persistir otro objeto `Jugador` de acceso dentro de `GameSessionState`. El actual
  `GameSessionState.jugadores` se reemplaza por `heroes` cuando se haga el corte de modelo.

Si se decide conservar un `Jugador` de dominio, debe tener una responsabilidad distinta y documentada; no
puede volver a guardar rol ni pertenencia que ya pertenecen a `Membresia`.

### 2. `squadId` no puede estar limitado al contenedor

BA-004 dice que el ámbito real de `squadId` es `Asentamiento.escuadrones` o `Ejercito.escuadrones`. Eso
confunde ubicación con identidad. La misma escuadra se mueve entre ambos contenedores y debe seguir siendo
referenciable por un ticket/result de batalla aun mientras cambia el estado del mundo.

`squadId` debe ser único y estable dentro de `gameId`. El contenedor actual es un campo/relación aparte con
invariante de exclusividad: una escuadra solo puede estar en guarnición, ejército, escolta o reserva de batalla
en un instante dado.

También debe revisarse el invariante “un `Escuadron` por `tropaId` por asentamiento”: si una escuadra sale y
se recluta otra del mismo tipo, ambas podrían regresar. La identidad estable no puede depender de que nunca
coincidan en la misma lista.

### 3. El resultado necesita idempotencia persistente

BA-001 compara la aplicación de `resultId` con la idempotencia actual de `RunnerDePartida`. El mapa
`RunnerDePartida.idempotencia` es una caché de promesas en memoria, limitada a 500 entradas; se pierde al
reiniciar y por tanto no sirve para una liquidación exactamente-una-vez que debe sobrevivir a reinicios.

La batalla persistida debe guardar como mínimo:

- `appliedResultId`;
- huella/hash del payload aplicado;
- instante y versión de estado resultante;
- estado terminal `aplicada`.

Repetir el mismo `resultId` y el mismo payload devuelve éxito previo sin mutar. Reutilizar el mismo
`resultId` con payload distinto se rechaza y audita. La aplicación de consecuencias y el cambio a `aplicada`
deben quedar en la misma mutación persistida, o existir un mecanismo explícito de recuperación de
`finalizada -> aplicada`.

### 4. Separar credenciales de servidor y tokens de jugador

`BattleServerAssignment` no debe contener una credencial única que BronzeAge reenvíe a los clientes.
Necesita dos clases de autorización:

- credencial servidor-a-servidor, capaz de registrar asignación y enviar `BattleResult`, nunca visible a un
  cliente;
- token de entrada por participante/batalla, de alcance limitado, duración corta y uso validado por el
  servidor Unity.

El contrato debe definir emisor, audiencia, expiración, rotación/revocación y protección frente a replay. La
elección concreta —token firmado, credencial opaca o mTLS— pertenece a infraestructura, pero la separación de
capacidades es un invariante del dominio.

### 5. Expiración no puede depender únicamente de una lectura

Marcar la batalla como fallida “al leer” no basta: un comando que intenta utilizar una escuadra reservada debe
normalizar primero las batallas expiradas, aunque nadie haya consultado su ficha. La expiración debe procesarse
por el tick/runner y también antes de validar acciones que compiten por reservas.

El fallo de infraestructura puede liberar sin penalización, como se decidió, pero debe producir un evento
persistente y auditable.

### 6. Aclarar inicio del ciclo y congelación del ticket

BA-001 conserva `propuesta -> convocando`, pero también afirma que `Batalla` solo nace cuando el combate se
compromete y el lobby termina. Debe elegirse una forma inequívoca:

- convocatoria/lobby fuera de `Batalla`; al comprometer, `Batalla` nace pendiente de asignación, o
- `Batalla` incluye realmente convocatoria y entonces sí persiste roster, aceptación y caducidad desde esa
  fase.

Recomendación: lobby separado y ticket inmutable al reservar. Después de emitir el ticket no se agregan
participantes/escuadras; cualquier sustitución genera una revisión nueva del ticket antes de iniciar, nunca
una mutación silenciosa que el servidor Unity no pueda detectar.

### 7. Validación semántica de `BattleResult`

JSON Schema solo valida forma. Antes de aplicar, BronzeAge debe comprobar:

- todos los participantes, héroes, escuadras y objetivos pertenecen al ticket;
- no hay IDs duplicados ni entidades nuevas;
- `survivors + deaths == deployed` por escuadra;
- ninguna cifra es negativa ni supera la reserva;
- ganador y razón son compatibles con las reglas del ticket;
- fechas y duración son coherentes;
- versión del contrato/build/balance está autorizada;
- la batalla está en un estado desde el que ese productor puede cerrarla.

BronzeAge sigue calculando XP, nivel, herida de héroe, conquista y ocupación. Unity reporta hechos, no deltas
de progresión.

## Ajustes al modelo de héroe y escuadra

### Héroe v1 mínimo

“Traer toda la estructura de Conquest” debe convertirse en una lista cerrada antes de programar. Propuesta:

- `id`, `jugadorId`, `displayName`, `classDefinitionId`, género y avatar;
- nivel, XP, puntos sin gastar y atributos base;
- perks desbloqueados;
- liderazgo, ubicación, memoria personal, exploración e `injuredUntil`;
- loadouts por ID;
- inventario por `itemInstanceId` y equipamiento por referencias.

No se persisten caches ni atributos calculados. Las monedas de Conquest se mantienen con nombres explícitos y
separadas del oro/recursos estratégicos hasta que exista una decisión de puente económico.

### Escuadra v1 mínima

- `id`, `heroeId`, `squadDefinitionId`/`tropaId`, nombre;
- `cantidad`, nivel y XP;
- moral;
- habilidades y formaciones desbloqueadas;
- formación seleccionada;
- ubicación/reserva estratégica.

Se elimina veteranía al adoptar nivel/XP. Las unidades de escuadra muertas reducen `cantidad` de forma
permanente y no existe `herido` para escuadra. El adaptador de Conquest debe dejar de interpretar
`SquadInstanceData.unitsInjured` como resultado persistente compartido.

El loadout referencia `squadId`; `totalLeadership` se calcula/valida en BronzeAge y no se acepta como verdad
del cliente.

## BA-003: worldgen y asentamientos

La parte de asentamientos está suficientemente encaminada, con estos matices:

- una `CeldaMuro` no tiene ID propio; su identidad estable debe definirse como `enclosureId + índice de
  recorrido` o añadirse explícitamente;
- calles/trazado son datos derivados y necesitan `layoutVersion`/seed si deben reconstruirse idénticos en un
  asedio;
- `visualSeed` y `visualCatalogVersion` deben estar en el snapshot que consume Unity;
- una variante 3D nunca cambia huella, acceso, producción ni colisión estratégica;
- si en el futuro un asedio destruye edificios/murallas, el resultado debe informar hechos y BronzeAge debe
  decidir el daño persistente.

La parte de mundo abierto no está cerrada. El propio archivo sigue como `LISTO_PARA_REVISION` y pide
confirmación humana. La recomendación híbrida necesita una frontera adicional: cualquier relieve que cambie
navegación, visibilidad o accesibilidad deja de ser cosmético y debe provenir de datos autoritativos. Unity
puede generar microdetalle solo cuando no altera reglas.

## Contratos antes de la migración masiva

La migración afecta al menos decenas de módulos que hoy usan `jugadorId`. Antes de renombrarlos debe existir:

1. un ADR o contrato aceptado de identidad con el flujo `Membresia -> Heroe`;
2. DTO/schema v1 exacto para héroe, escuadra y loadout;
3. fixtures dorados;
4. tests de unicidad/propiedad;
5. estrategia de corte de API, aunque se haya decidido no migrar snapshots viejos.

Esto evita completar una sustitución mecánica de `jugadorId -> heroeId` y descubrir después que la capa de
autorización perdió la identidad de cuenta que necesita para auditar.

## Conclusión de revisión

BA-001, BA-002 y BA-004 pueden pasar a diseño implementable una vez incorporadas estas correcciones. BA-003
requiere marcar explícitamente si se acepta la recomendación híbrida o si permanece abierta. Ninguna de las
cuatro está implementada todavía.
