# simulaciones-batch

Carpeta fuera de `src/` para escenarios de batch desechables — ver `Consideraciones/Diarios_Simulaciones_Batch/*.md`
para los resultados que este tipo de script produce. El script estable vive en `scripts/run-batch-sim.ts`.

## El NPC de gobernanza vive ahora en `src/app/npcGobernanza.ts`

**Se movió** fuera de esta carpeta: el mismo comportamiento lo usa ahora también la
partida real, donde el jugador puede ceder una Facción al NPC desde la pestaña Facción y seguir jugando el
resto a mano (`GameState.faccionesNpcIds` → `GameStore.avanzarTick`). Sigue teniendo acoplamiento CERO con el
motor: vive en la capa de aplicación y solo consume funciones PÚBLICAS de `src/engine/*`, igual que `GameStore`
cuando el jugador pulsa un botón.

Diseño, decisiones y limitaciones: `Consideraciones/NPC_Gobernanza_Facciones_Controladas.md`.

Lo que cambia para un escenario de batch: **nada**, salvo la ruta del import. `ConfigNpcGobernanza.faccionesIds`
acota el NPC a un subconjunto de Facciones; omitirlo (lo que hacen los scripts de batch) es gobernar el mundo
entero, como siempre.

El comportamiento: un NPC que juega el rol de Gobernador/Tesorero/Rey (decisiones que en el juego real toma un jugador humano desde la UI, Doc 2.2/3.2).
No genera mundo, no funda nada inicial, no escribe archivos — solo decide, con las funciones PÚBLICAS del
motor (`src/engine/*`, nunca tocadas), qué haría ese NPC en un tick dado:

1. Gobernanza base — Gobernador+Tesorero para cualquier asentamiento, y reserva de 150 de madera vía
   `reservaManual` (protege esa madera de la auto-construcción, ver `RESERVA_MADERA_ANTES_DE_RECLUTAR`).
2. Infraestructura comercial (Mercado + caravana propia) para CUALQUIER asentamiento, sin esperar a que su
   Facción tenga 2+ asentamientos (a diferencia del paso 4).
3. Trueque de SUPERVIVENCIA — si a un asentamiento no le va a alcanzar para pagar Mantenimiento (madera
   siempre, +piedra desde nivel 2, +oro desde nivel 3), busca otro asentamiento con excedente de ese recurso
   y le propone un trueque, pagando con el mejor recurso propio de sobra — oro solo como último recurso si no
   hay otro. La idea es sobrevivir, no competir; ver `truequeDeSupervivencia` en `npcGobernanza.ts`. Con
   `faccionesIds` (partida real) el socio también tiene que ser NPC: `proponerTrueque` pacta sin pedir
   consentimiento al otro lado, y un NPC no puede comprometer recursos del jugador humano. En batch, sin ese
   filtro, el socio puede ser de cualquier Facción, como siempre.
4. Trueque de especialización — delega en `avanzarAutoComercioSimulado`
   (`src/engine/simulacionAutoComercio.ts`): equilibra minerales/livestock, pero solo DENTRO de la misma
   Facción y solo si esta tiene 2+ asentamientos propios.
5. Reclutamiento — cada residente recluta/repone su propio escuadrón (`reclutarTropa`), pero solo si el
   asentamiento tiene al menos 150 de madera en almacén (gate propio del NPC, no del motor — protege el
   margen de Mantenimiento). Mientras el asentamiento sigue en nivel 1, solo UN residente recluta por tick
   (no los 5) — 5 reclutando a la vez apenas se cruza el umbral de 150 volvería a vaciar la madera de golpe.
6. Ataque a campamentos de bandidos cercanos — con todos los escuadrones del asentamiento a la vez.
7. Expansión — lanza una Caravana de Fundación cuando el asentamiento cumple los requisitos reales del motor.

## Cómo usarlo desde un escenario

Un escenario es un test file TEMPORAL (normalmente `src/engine/__tests__/_tmp_<algo>.test.ts`, borrado al
terminar de extraer su diario) que construye el mundo/Facciones/asentamientos iniciales y, cada tick:

```ts
import { avanzarSimulacion } from '../../engine/simulation';
import { avanzarNpcGobernanza } from '../../app/npcGobernanza';

const resultado = avanzarSimulacion(estado, mapa, tick); // motor real, sin tocar
const npc = avanzarNpcGobernanza(resultado, mapa, tick, {}); // sin `faccionesIds` = gobierna el mundo entero
estado = npc.estado;
```

El destino de las Caravanas de Fundación es por defecto `buscarDestinoFundacionPorDefecto` (barrido radial,
la heurística que antes duplicaba cada escenario); un escenario puede pasar `buscarDestinoFundacion` propio si
quiere otra estrategia de posicionamiento.

`SIMULACION_AUTO_COMERCIO.activo` debe ponerse en `1` en memoria del proceso del escenario (nunca en
`src/constants.ts` — el repo lo mantiene apagado por defecto, ver el propio archivo).

Lo que NO vive en el NPC (queda en el escenario desechable de turno): generación de mundo, elección de las
posiciones de fundación INICIALES, el bucle de ticks en sí, fotos/snapshots, escritura de resultados a
JSON/diario.
