// Cuánto cuesta difundir los eventos de un tick (hub.difundir) con y sin un administrador suscrito a `admin/eventos` (contrato 03): sobre una partida
// grande, con los eventos que de verdad produce cada tick y con un puñado de jugadores suscritos a sus canales de siempre.
//   npx tsx scripts/medicion-eventos-admin.ts <directorio> <gameId> [ticks]
// SOLO LECTURA sobre una COPIA (el runner avanza ticks y los guarda).
import { performance } from 'node:perf_hooks';
import { crearAlmacenEnDisco } from '../src/server/almacen/enDisco';
import { HubDeDifusion, SIN_JUGADOR } from '../src/server/difusion/hub';
import { RunnerDePartida } from '../src/server/runnerDePartida';
import type { EventoDominio } from '../src/domain/eventos';

const [directorio, gameId, ticksArg] = process.argv.slice(2);
if (!directorio || !gameId) {
  console.error('uso: npx tsx scripts/medicion-eventos-admin.ts <directorio> <gameId> [ticks]');
  process.exit(2);
}
const TICKS = Number(ticksArg ?? '20');
const ticksDeEventos: EventoDominio[][] = [];
const runner = await RunnerDePartida.cargarOCrear(gameId, { seed: 1 }, { almacen: crearAlmacenEnDisco(directorio), ahora: () => new Date().toISOString(), alEmitir: (_id, eventos) => ticksDeEventos.push([...eventos]) });
const tickMs: number[] = [];
for (let i = 0; i < TICKS; i++) {
  const t0 = performance.now();
  await runner.avanzarTick();
  tickMs.push(performance.now() - t0);
}
// `avanzarTick` (administración) no pasa por `alEmitir`: se recogen los eventos de memoria de cada tick.
const estado = runner.getState();
const eventos = estado.eventosDominio.slice(0, 4000);
const porTick = Math.max(1, Math.round(eventos.length / TICKS));
const trozos = Array.from({ length: TICKS }, (_, i) => eventos.slice(i * porTick, (i + 1) * porTick));

const mediana = (xs: number[]) => [...xs].sort((a, b) => a - b)[Math.floor(xs.length / 2)] ?? 0;
const socket = (): never => ({ OPEN: 1, readyState: 1, send: () => undefined, close: () => undefined }) as never;

function medir(prepararHub: (hub: HubDeDifusion) => void): number {
  const hub = new HubDeDifusion();
  prepararHub(hub);
  const t: number[] = [];
  for (const lote of trozos) {
    const t0 = performance.now();
    hub.difundir(gameId, lote);
    t.push(performance.now() - t0);
  }
  return mediana(t);
}

const jugadores = estado.heroes.filter((h) => h.jugadorId).slice(0, 50);
const suscribirJugadores = (hub: HubDeDifusion) => {
  hub.conectar(gameId, 'nadie', socket()); // un socket más, sin suscripciones
  for (const h of jugadores) {
    const s = socket();
    hub.conectar(gameId, h.jugadorId!, s);
    hub.suscribir(gameId, s, 'mapa/general');
    hub.suscribir(gameId, s, `heroe/${h.id}`);
    for (const a of estado.asentamientos.filter((x) => x.heroesFundadoresIds.includes(h.id))) hub.suscribir(gameId, s, `asentamiento/${a.id}`);
  }
};
const suscribirAdmin = (hub: HubDeDifusion) => {
  const s = socket();
  hub.conectar(gameId, SIN_JUGADOR, s, () => true);
  hub.suscribir(gameId, s, 'admin/eventos');
};

console.log(`partida '${gameId}': ${estado.asentamientos.length} plazas, ${estado.heroes.length} héroes; ${eventos.length} eventos repartidos en ${TICKS} lotes (${porTick} por tick); un tick del mundo ≈ ${mediana(tickMs).toFixed(0)} ms\n`);
console.log(`hub.difundir por tick (mediana):`);
console.log(`  sin nadie suscrito a nada ........................ ${medir(() => undefined).toFixed(3)} ms`);
console.log(`  ${jugadores.length} jugadores en sus canales ........................ ${medir(suscribirJugadores).toFixed(3)} ms`);
console.log(`  + 1 administrador en admin/eventos ............... ${medir((h) => { suscribirJugadores(h); suscribirAdmin(h); }).toFixed(3)} ms`);
console.log(`  solo 1 administrador en admin/eventos ............ ${medir(suscribirAdmin).toFixed(3)} ms`);
