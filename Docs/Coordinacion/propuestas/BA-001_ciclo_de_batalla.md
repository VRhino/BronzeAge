# BA-001 — Ciclo persistente de batalla jugable

**Destino:** `BronzeAgeFase0`  
**Estado:** ACEPTADO (revisión 2026-09-11, corregido con la revisión de Codex del mismo día — decisiones
cerradas, diseño listo para implementar del lado de BronzeAge)  
**Prioridad:** Alta  
**Solicitante:** Conquest / Codex

## Problema

El combate estratégico se resuelve dentro de la operación que ejecuta el enfrentamiento. Una partida real
puede durar minutos, sufrir reconexiones y terminar en otro proceso. Mantener abierto el comando o la cola del
mundo durante ese tiempo bloquearía la partida y no sobreviviría a reinicios.

## Comportamiento solicitado

Introducir una entidad persistente `Batalla` con apertura y liquidación separadas:

```text
propuesta -> convocando -> asignada -> en_curso -> finalizada -> aplicada
       \-> cancelada       \-----------------------> fallida
```

Invariantes:

- solo BronzeAge abre, reserva y aplica;
- solo un servidor de batalla Unity autenticado puede cerrar con resultado;
- un resultado se aplica como máximo una vez;
- jugadores, escuadras y recursos reservados no participan en otra acción incompatible;
- reiniciar el proceso no pierde batalla, reservas ni estado de aplicación;
- la cola del mundo no queda ocupada mientras se disputa la partida.

## Contratos mínimos propuestos

`BattleTicket` incluye `schemaVersion`, `battleId`, `gameId`, vigencia, contexto estratégico, dos bandos con
capacidad independiente, participantes (por `heroeId`, no `jugadorId` — ver BA-002/BA-004), escuadras,
efectivos, mapa, versiones de reglas y autorización limitada a la batalla.

`BattleResult` incluye `schemaVersion`, `battleId`, `resultId`, inicio/fin, ganador, razón, objetivos:

- **por escuadra:** efectivos desplegados/supervivientes/**muertos** — sin "heridos": una escuadra dañada
  pierde efectivos de forma permanente, nunca queda en un estado intermedio (decisión BA-004 #3, gana el
  modelo de BronzeAge).
- **por héroe:** participó/sobrevivió y, si aplica, **herido** (debuff temporal análogo al `heridoHasta`
  que hoy tiene `Escuadron`, pero movido a `Heroe` — el héroe sí combate directamente en la partida real de
  Unity, la escuadra no tiene ese estado). Dura 2 minutos de mundo (decisión del usuario, 2026-09-13).
- **XP ganada por héroe y por escuadra, ya calculada por Unity** (decisión del usuario, 2026-09-13): depende
  del desempeño en batalla, que solo conoce el servidor de batalla. Llega como delta; BronzeAge la valida,
  la suma y aplica la curva de nivel.

Más versión de servidor y autenticidad.

El resultado contiene hechos tácticos más la XP ganada, no órdenes como "conquistar asentamiento".
BronzeAge interpreta los hechos y aplica conquista, ocupación, herida de héroe, suministro y liberación de
reservas, y suma la XP recibida (nivel/XP de escuadra reemplaza a veteranía, decisión BA-004 #2).

## Criterios de aceptación

- La batalla y sus reservas sobreviven a reinicio.
- Abrirla y resolverla son operaciones distintas.
- Repetir `resultId` no duplica efectos.
- Se rechazan productor no autorizado, versión incompatible y estado inválido.
- Hay prueba con bandos asimétricos, cancelación y liberación de reservas.
- Se publican fixtures JSON de ticket y resultado para Conquest.

## Decisiones del propietario (resueltas, revisión 2026-09-11)

1. **Tiempo máximo y política ante fallo de infraestructura.** `Batalla` lleva `expiraEn: Instante` —mismo
   patrón que `AcuerdoTrueque.expiraEn`/`Caravana.preparaHasta`, ya usado en el dominio—, fijado al pasar a
   `convocando`/`asignada`. Si se alcanza sin recibir `BattleResult`, la batalla pasa a `fallida`: libera
   reservas (participantes, escuadras, suministro) sin aplicar bajas ni derrota a nadie — un fallo de
   infraestructura no debe penalizar al jugador. **Corrección de Codex incorporada:** comprobarlo solo "al
   leer" no basta — un comando que intenta usar una escuadra reservada podría nunca disparar esa lectura. La
   expiración se procesa (a) en cada tick/runner, igual que el resto de vencimientos de mundo (`Instante`),
   y (b) de forma defensiva antes de validar cualquier acción que compita por las mismas reservas. Cada
   transición a `fallida` por timeout emite un evento persistente/auditable — no es un efecto silencioso.
2. **Ubicación de la asignación de instancia.** No es responsabilidad de BronzeAge — el orquestador y las
   instancias de servidor Unity son de Conquest (tabla de propiedad de
   `02_Arquitectura_objetivo.md`). BronzeAge solo publica el `BattleTicket` con `battleId` y espera que
   Conquest reporte de vuelta un `BattleServerAssignment` que BronzeAge persiste y reenvía a los clientes
   autorizados. BronzeAge nunca decide QUÉ instancia física ejecuta la partida. **Corrección de Codex
   incorporada — separar credenciales:** `BattleServerAssignment` NO lleva una credencial única reenviada
   tal cual a los clientes. Dos clases de autorización distintas: (a) credencial servidor-a-servidor —
   registra la asignación y firma el `BattleResult`, nunca visible a un cliente—; (b) token de entrada por
   participante/batalla — alcance limitado a esa batalla, duración corta, validado por el servidor Unity. El
   contrato debe fijar emisor, audiencia, expiración y protección frente a replay de cada uno; la
   implementación concreta (token firmado, credencial opaca, mTLS) es infraestructura, pero la separación de
   capacidades es invariante de dominio.
3. **Relación entre convocatoria/lobby y `Batalla`.** **Corrección de Codex incorporada — se adopta
   explícitamente su recomendación** (la propuesta original dejaba dos lecturas posibles sin elegir):
   convocatoria/lobby vive FUERA de `Batalla`, con las entidades que ya existen (`Ejercito`, `participantes`)
   y sin reserva de recursos. `Batalla` nace cuando BronzeAge decide comprometer el combate real (mismo punto
   que los comandos ya existentes `iniciarAsedio`/`combateCampoAbierto`/`interceptarCaravana`), y **el ticket
   queda INMUTABLE en ese momento**: después de emitirlo no se agregan participantes ni escuadras. Cualquier
   sustitución (por desconexión, por ejemplo) genera una revisión NUEVA del ticket antes de que la partida
   empiece a jugarse — nunca una mutación silenciosa que el servidor Unity no pueda detectar.
4. **Datos tácticos adicionales necesarios para consecuencias estratégicas.** Cubiertos arriba en
   "Contratos mínimos propuestos": bajas por escuadra (muertos, sin heridos), estado de herida por héroe,
   objetivos capturados (para conquista/ocupación), duración real y ganador/razón. **Corrección de Codex
   incorporada — validación semántica antes de aplicar** (el JSON Schema solo valida forma): BronzeAge
   comprueba, antes de aplicar cualquier `BattleResult`, que (a) todos los participantes/héroes/escuadras/
   objetivos pertenecen al ticket congelado; (b) no hay IDs duplicados ni entidades nuevas; (c)
   `survivors + deaths == deployed` por escuadra; (d) ninguna cifra es negativa ni supera lo reservado; (e)
   ganador y razón son compatibles con las reglas del ticket; (f) fechas/duración son coherentes; (g) la
   versión de contrato/build/balance está autorizada; (h) la batalla está en un estado desde el que ESE
   productor puede cerrarla. BronzeAge sigue decidiendo herida de héroe, conquista y ocupación a partir de
   los hechos. **Excepción, decisión del usuario 2026-09-13:** la XP sí llega ya calculada por Unity (solo
   el servidor de batalla ve el desempeño); BronzeAge la valida, la suma y calcula el nivel.

## Plan de implementación (listo del lado de BronzeAge)

Sin migración de partidas existentes (decisión general del usuario, 2026-09-11) — el snapshot con `Batalla`
nace desde cero, no hay que convivir con partidas guardadas sin ella.

1. **`domain/types.ts`:** nuevas interfaces `Batalla`, `BatallaBando`, `BatallaParticipante` (por `heroeId`),
   `BatallaReserva`. Estado como unión cerrada (mismo patrón que `Caravana.estado`/`AcuerdoTrueque.estado`):
   `'propuesta' | 'convocando' | 'asignada' | 'en_curso' | 'finalizada' | 'aplicada' | 'cancelada' | 'fallida'`.
   `Batalla` guarda, para la idempotencia del punto 2: `appliedResultId?`, huella/hash del payload aplicado,
   instante y versión de estado resultante.
2. **`engine/batalla.ts`** (nuevo): `abrirBatalla` (crea `Batalla`, reserva participantes/escuadras/
   suministro, publica `BattleTicket` inmutable), `registrarAsignacion` (recibe `BattleServerAssignment` de
   Conquest, con las dos clases de credencial separadas del punto 2 de arriba), `aplicarResultadoBatalla`.
   **Corrección de Codex incorporada — idempotencia persistida, no en memoria:** la caché
   `RunnerDePartida.idempotencia` es en memoria (máx. 500 entradas, se pierde al reiniciar) y NO sirve para
   una liquidación exactamente-una-vez que debe sobrevivir a reinicios — es un mecanismo distinto, para otra
   cosa. `aplicarResultadoBatalla` debe: autenticar productor, correr la validación semántica del punto 4 de
   arriba, y luego — repetir el mismo `resultId` con el mismo payload devuelve éxito previo sin mutar;
   repetirlo con payload distinto se rechaza y se audita; aplicar consecuencias (bajas/XP/herida/conquista/
   liberación de reservas) y pasar a `aplicada` deben quedar en la MISMA mutación persistida, o existir un
   mecanismo explícito de recuperación de `finalizada -> aplicada` si el proceso muere entre medias.
3. **`src/contratos/v1/`** (ubicación acordada en BA-004): `battleTicket.schema.json`,
   `battleResult.schema.json` + fixtures dorados de cada uno, más DTO TS que Conquest pueda deserializar en
   C# sin depender de `domain/types.ts`.
4. **Pruebas:** batalla que sobrevive a reinicio (fixture + reload de snapshot), bandos asimétricos,
   cancelación con liberación de reservas, `resultId` repetido con mismo payload no duplica efectos,
   `resultId` repetido con payload distinto se rechaza y audita, productor no autorizado y versión
   incompatible se rechazan, timeout de infraestructura pasa a `fallida` sin penalizar (vía tick, no solo al
   leer), recuperación de una batalla que murió justo entre `finalizada` y `aplicada`.

Con esto los criterios de aceptación de la propuesta quedan cubiertos por diseño; falta solo escribir el
código de los puntos 1-4.

