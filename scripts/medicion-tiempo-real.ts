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
import { DifusorDeVistas } from '../src/server/difusion/difusorDeVistas';

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
const conCuenta = estado.heroes.filter((h) => h.jugadorId);
const jugadores = conCuenta.length > 0 ? conCuenta : estado.heroes; // en un checkpoint del batch los héroes son bots, sin cuenta
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

// --- Canales de vista (contrato 03): lo mismo, empujado ---
const difusor = new DifusorDeVistas({ fuenteDe: () => runner, coalescerMs: 0 });
const enviados: number[] = [];
const socket = { OPEN: 1, readyState: 1, bufferedAmount: 0, send: (m: string) => enviados.push(Buffer.byteLength(m)), close: () => undefined } as never;
const conexion = { audiencia: { tipo: 'admin' as const }, derecho: () => true };
difusor.suscribir(gameId, socket, conexion, 'admin/partida');
const bytesFoto = enviados.at(-1)!;
const TICKS_MEDIDOS = 20;
enviados.length = 0;
for (let i = 0; i < TICKS_MEDIDOS; i++) {
  await runner.avanzarTick();
  difusor.notificar(gameId);
}
const m = difusor.metricas().porCanal['admin/partida']!;
const mediaDelta = enviados.reduce((a, b) => a + b, 0) / Math.max(1, enviados.length);
console.log(`
canal admin/partida:     foto ${kib(bytesFoto)}, delta medio ${kib(mediaDelta)} (${enviados.length} deltas en ${TICKS_MEDIDOS} ticks, máx ${kib(Math.max(0, ...enviados))})`);
console.log(`  proyección p50 ${m.ms.proyeccion.p50.toFixed(1)} ms · diff p50 ${m.ms.diff.p50.toFixed(1)} ms · serialización p50 ${m.ms.serializacion.p50.toFixed(2)} ms`);
console.log(`  cambios por op: ${JSON.stringify(m.cambiosPorOp)}`);
console.log(`  → a 1 tick/min: ${kib(mediaDelta)}/min por cliente admin (el sondeo a 3 s: ${kib((admin.bytes * 60) / 3)}/min → ${((mediaDelta / ((admin.bytes * 60) / 3)) * 100).toFixed(2)} %)`);

// --- Canales de jugador: vista/mapa + vista/heroe de una muestra de héroes (grupo por héroe) ---
const heroesMuestra = jugadores.slice(0, Math.min(jugadores.length, 50));
const porCanal: Record<string, { fotos: number[]; deltas: number[] }> = { 'vista/mapa': { fotos: [], deltas: [] }, 'vista/heroe': { fotos: [], deltas: [] } };
let enFoto = true;
for (const h of heroesMuestra) {
  const sock = {
    OPEN: 1,
    readyState: 1,
    bufferedAmount: 0,
    send: (m: string) => {
      const msg = JSON.parse(m) as { tipo: string; canal?: string };
      if (msg.canal && porCanal[msg.canal]) (enFoto ? porCanal[msg.canal]!.fotos : porCanal[msg.canal]!.deltas).push(Buffer.byteLength(m));
    },
    close: () => undefined,
  } as never;
  const c = { audiencia: { tipo: 'jugador' as const, jugadorId: h.jugadorId ?? '' }, derecho: () => true };
  if (!h.jugadorId) continue;
  difusor.suscribir(gameId, sock, c, 'vista/mapa');
  difusor.suscribir(gameId, sock, c, 'vista/heroe');
}
enFoto = false;
const t0 = performance.now();
for (let i = 0; i < 5; i++) {
  await runner.avanzarTick();
  difusor.notificar(gameId);
}
const msPorTick = (performance.now() - t0) / 5;
const media = (xs: number[]) => (xs.length ? xs.reduce((a, b) => a + b, 0) / xs.length : 0);
const suscritos = Math.max(1, heroesMuestra.filter((h) => h.jugadorId).length);
console.log(`
canales de jugador (${suscritos} héroes suscritos a vista/mapa y vista/heroe), 5 ticks:`);
for (const [canal, d] of Object.entries(porCanal)) console.log(`  ${canal}: foto media ${kib(media(d.fotos))}, delta medio ${kib(media(d.deltas))}`);
const vm = difusor.metricas().porCanal;
console.log(`  proyección p50/p95 vista/mapa ${vm['vista/mapa']?.ms.proyeccion.p50.toFixed(2)}/${vm['vista/mapa']?.ms.proyeccion.p95.toFixed(2)} ms · diff p50 ${vm['vista/mapa']?.ms.diff.p50.toFixed(2)} ms`);
console.log(`  coste de un tick (avanzar + flush de ${suscritos} héroes): ${msPorTick.toFixed(0)} ms`);
const deltaJugador = media(porCanal['vista/mapa']!.deltas) + media(porCanal['vista/heroe']!.deltas);
console.log(`  → delta de un jugador por tick: ${kib(deltaJugador)} (el sondeo a 5 s: ${kib((totalBytes / Math.max(1, muestra.length)) * 12)}/min → ${((deltaJugador / ((totalBytes / Math.max(1, muestra.length)) * 12)) * 100).toFixed(2)} %)`);
