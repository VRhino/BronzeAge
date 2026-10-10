// Línea base del sondeo (`Docs/Coordinacion/03_Tiempo_real_por_segmentos.md` §6.4): cuánto cuesta construir y serializar el estado
// admin y la proyección de cada jugador, que es lo que hoy hace cada cliente cada 3-5 s. Después de la Fase 1 y la 2 se repite la
// medición con los canales de vista para comparar bytes y milisegundos.
//
//   npx tsx scripts/medicion-tiempo-real.ts <directorio-con-el-snapshot> <gameId> [ticks]
//
// SOLO LECTURA sobre una COPIA: abre el snapshot con un almacén en disco y no hace ninguna escritura, pero no lo apuntes al directorio de una
// partida viva que el servidor tenga abierta (copia `<gameId>.json` a otro directorio, sin los `.jsonl`).
import { performance } from 'node:perf_hooks';
import { crearAlmacenEnDisco } from '../src/server/almacen/enDisco';
import { RunnerDePartida } from '../src/server/runnerDePartida';
import { vistaAdminDeEstado } from '../src/session/estado';
import { proyectarParaJugador } from '../src/session/proyecciones/jugador';

const [directorio, gameId, ticksArg] = process.argv.slice(2);
if (!directorio || !gameId) {
  console.error('uso: npx tsx scripts/medicion-tiempo-real.ts <directorio> <gameId> [ticks]');
  process.exit(2);
}

const mediana = (xs: number[]): number => [...xs].sort((a, b) => a - b)[Math.floor(xs.length / 2)] ?? 0;
const kib = (bytes: number): string => `${(bytes / 1024).toFixed(1)} KiB`;

/** Tiempo (ms) y tamaño (bytes) de construir y serializar algo, mediana de `veces`. */
function medir(construir: () => unknown, veces = 10): { construirMs: number; serializarMs: number; bytes: number } {
  const c: number[] = [];
  const s: number[] = [];
  let bytes = 0;
  for (let i = 0; i < veces; i++) {
    const t0 = performance.now();
    const objeto = construir();
    const t1 = performance.now();
    bytes = Buffer.byteLength(JSON.stringify(objeto));
    const t2 = performance.now();
    c.push(t1 - t0);
    s.push(t2 - t1);
  }
  return { construirMs: mediana(c), serializarMs: mediana(s), bytes };
}

const runner = await RunnerDePartida.cargarOCrear(gameId, { seed: 1 }, { almacen: crearAlmacenEnDisco(directorio), ahora: () => new Date().toISOString() });
const ticks = Number(ticksArg ?? '0');
for (let i = 0; i < ticks; i++) await runner.avanzarTick();

const estado = runner.getState();
const jugadores = estado.heroes.filter((h) => h.jugadorId);
console.log(`partida '${gameId}' v${estado.version} tick ${estado.tick}: ${estado.asentamientos.length} asentamientos, ${estado.heroes.length} héroes (${jugadores.length} con cuenta), ${estado.ejercitos.length} columnas, ${estado.caravanas.length} caravanas, ${estado.facciones.length} Facciones\n`);

const admin = medir(() => ({
  ...vistaAdminDeEstado(runner.getState()),
  preciosReferencia: runner.preciosReferencia(),
  relojDeMundoIntervaloMs: runner.intervaloRelojDeMundoMs(),
  ...runner.geometriaAsentamientos(),
}));
console.log(`GET admin completo:      construir ${admin.construirMs.toFixed(1)} ms, serializar ${admin.serializarMs.toFixed(1)} ms, ${kib(admin.bytes)}`);

let totalMs = 0;
let totalBytes = 0;
const muestra = jugadores.slice(0, 20);
for (const h of muestra) {
  const m = medir(() => proyectarParaJugador(runner.getState(), h.id, runner.geometriaAsentamientos()), 5);
  totalMs += m.construirMs + m.serializarMs;
  totalBytes += m.bytes;
}
if (muestra.length > 0) {
  console.log(
    `GET proyección jugador:  media de ${muestra.length}: ${(totalMs / muestra.length).toFixed(1)} ms (construir+serializar), ${kib(totalBytes / muestra.length)}`
  );
  const por5s = (n: number) => `${kib((n * 60 * (totalBytes / muestra.length)) / 5)}/min por jugador a 1 sondeo/5 s`;
  console.log(`  → ${por5s(1)}`);
}
console.log(`  → admin a 1 sondeo/3 s: ${kib((admin.bytes * 60) / 3)}/min`);
