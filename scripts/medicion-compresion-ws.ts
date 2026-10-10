// Cuánto ahorra comprimir (permessage-deflate, contrato 03 §13.2) las fotos y los deltas de los canales de vista, y cuánta CPU cuesta: toma los
// mensajes que de verdad produce el difusor sobre una partida y los pasa por zlib (deflate crudo, lo que usa la extensión) a varios niveles.
//
//   npx tsx scripts/medicion-compresion-ws.ts <directorio-con-el-snapshot> <gameId>
// SOLO LECTURA sobre una COPIA, como `medicion-tiempo-real.ts`.
import { performance } from 'node:perf_hooks';
import { deflateRawSync } from 'node:zlib';
import { crearAlmacenEnDisco } from '../src/server/almacen/enDisco';
import { DifusorDeVistas } from '../src/server/difusion/difusorDeVistas';
import { RunnerDePartida } from '../src/server/runnerDePartida';

const [directorio, gameId] = process.argv.slice(2);
if (!directorio || !gameId) {
  console.error('uso: npx tsx scripts/medicion-compresion-ws.ts <directorio> <gameId>');
  process.exit(2);
}
const kib = (b: number) => `${(b / 1024).toFixed(1)} KiB`;
const mediana = (xs: number[]) => [...xs].sort((a, b) => a - b)[Math.floor(xs.length / 2)] ?? 0;

const runner = await RunnerDePartida.cargarOCrear(gameId, { seed: 1 }, { almacen: crearAlmacenEnDisco(directorio), ahora: () => new Date().toISOString() });
const difusor = new DifusorDeVistas({ fuenteDe: () => runner, coalescerMs: 0 });
const mensajes: { canal: string; tipo: string; texto: string }[] = [];
const socket = {
  OPEN: 1,
  readyState: 1,
  bufferedAmount: 0,
  send: (m: string) => {
    const x = JSON.parse(m) as { tipo: string; canal?: string };
    if (x.canal && (x.tipo === 'foto' || x.tipo === 'delta')) mensajes.push({ canal: x.canal, tipo: x.tipo, texto: m });
  },
  close: () => undefined,
} as never;

const estado = runner.getState();
const jugador = estado.heroes.find((h) => h.jugadorId)!;
difusor.suscribir(gameId, socket, { audiencia: { tipo: 'admin' }, derecho: () => true }, 'admin/partida');
for (const canal of ['vista/mapa', 'vista/heroe']) difusor.suscribir(gameId, socket, { audiencia: { tipo: 'jugador', jugadorId: jugador.jugadorId! }, derecho: () => true }, canal);
for (let i = 0; i < 3; i++) {
  await runner.avanzarTick();
  difusor.notificar(gameId);
}

console.log(`partida '${gameId}' v${runner.getState().version}: ${estado.asentamientos.length} plazas, ${estado.heroes.length} héroes\n`);
console.log('mensaje'.padEnd(26) + 'bruto'.padStart(12) + '   nivel 1'.padEnd(28) + 'nivel 3'.padEnd(28) + 'nivel 6');
for (const tipo of ['foto', 'delta'] as const) {
  for (const canal of ['admin/partida', 'vista/mapa', 'vista/heroe']) {
    const m = mensajes.filter((x) => x.canal === canal && x.tipo === tipo).at(-1);
    if (!m) continue;
    const buf = Buffer.from(m.texto);
    const celdas = [1, 3, 6].map((level) => {
      const tiempos: number[] = [];
      let largo = 0;
      for (let i = 0; i < 15; i++) {
        const t0 = performance.now();
        largo = deflateRawSync(buf, { level }).length;
        tiempos.push(performance.now() - t0);
      }
      return `${kib(largo)} (${((largo / buf.length) * 100).toFixed(0)} %) ${mediana(tiempos).toFixed(1)} ms`.padEnd(28);
    });
    console.log(`${tipo} ${canal}`.padEnd(26) + kib(buf.length).padStart(12) + '   ' + celdas.join(''));
  }
}
