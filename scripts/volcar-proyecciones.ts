// Vuelca la proyección de jugador de TODOS los héroes de una partida a un archivo JSON, para comparar antes y después de tocar la proyección
// (`diff` de dos volcados): es el guardián de que un refactor no cambia lo que ve ningún jugador.
//
//   npx tsx scripts/volcar-proyecciones.ts <directorio> <gameId> <salida.json> [maxHeroes]
import { writeFileSync } from 'node:fs';
import { crearAlmacenEnDisco } from '../src/server/almacen/enDisco';
import { RunnerDePartida } from '../src/server/runnerDePartida';
import { proyectarParaJugador } from '../src/session/proyecciones/jugador';

const [directorio, gameId, salida, max] = process.argv.slice(2);
if (!directorio || !gameId || !salida) {
  console.error('uso: npx tsx scripts/volcar-proyecciones.ts <directorio> <gameId> <salida.json> [maxHeroes]');
  process.exit(2);
}
const runner = await RunnerDePartida.cargarOCrear(gameId, { seed: 1 }, { almacen: crearAlmacenEnDisco(directorio), ahora: () => new Date().toISOString() });
const estado = runner.getState();
const geometria = runner.geometriaAsentamientos();
const heroes = estado.heroes.slice(0, Number(max ?? '100000'));
const volcado = Object.fromEntries(heroes.map((h) => [h.id, proyectarParaJugador(estado, h.id, geometria)]));
writeFileSync(salida, JSON.stringify(volcado));
console.log(`${heroes.length} proyecciones de '${gameId}' (v${estado.version}) en ${salida}`);
