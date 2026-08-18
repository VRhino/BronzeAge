# simulaciones-batch

Carpeta fuera de `src/` (no forma parte del motor ni del build de la app) para código reutilizable entre
scripts de prueba de batch — ver `Consideraciones/Diarios_Simulaciones_Batch/*.md` para los resultados que
este tipo de script produce.

## `npcGobernanza.ts`

El único archivo persistente de esta carpeta: el comportamiento de un NPC que juega el rol de
Gobernador/Tesorero/Rey (decisiones que en el juego real toma un jugador humano desde la UI, Doc 2.2/3.2).
No genera mundo, no funda nada inicial, no escribe archivos — solo decide, con las funciones PÚBLICAS del
motor (`src/engine/*`, nunca tocadas), qué haría ese NPC en un tick dado:

1. Gobernanza base — Gobernador+Tesorero para cualquier asentamiento, y reserva de 150 de madera vía
   `reservaManual` (protege esa madera de la auto-construcción, ver `RESERVA_MADERA_ANTES_DE_RECLUTAR`).
2. Infraestructura comercial (Mercado + caravana propia) para CUALQUIER asentamiento, sin esperar a que su
   Facción tenga 2+ asentamientos (a diferencia del paso 4).
3. Trueque de SUPERVIVENCIA — si a un asentamiento no le va a alcanzar para pagar Mantenimiento (madera
   siempre, +piedra desde nivel 2, +oro desde nivel 3), busca CUALQUIER otro asentamiento de CUALQUIER
   Facción con excedente de ese recurso y le propone un trueque, pagando con el mejor recurso propio de
   sobra — oro solo como último recurso si no hay otro. La idea es sobrevivir, no competir; ver
   `truequeDeSupervivencia` en `npcGobernanza.ts`.
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
import { avanzarNpcGobernanza } from '../../../simulaciones-batch/npcGobernanza';

const resultado = avanzarSimulacion(estado, mapa, tick); // motor real, sin tocar
const npc = avanzarNpcGobernanza(resultado, mapa, tick, {
  buscarDestinoFundacion: (origen, mapa, asentamientos) => { /* el escenario decide el punto */ },
});
estado = npc.estado;
```

`SIMULACION_AUTO_COMERCIO.activo` debe ponerse en `1` en memoria del proceso del escenario (nunca en
`src/constants.ts` — el repo lo mantiene apagado por defecto, ver el propio archivo).

Lo que NO vive aquí (queda en el escenario desechable de turno): generación de mundo, elección de posiciones
de fundación, el bucle de ticks en sí, fotos/snapshots, escritura de resultados a JSON/diario.
