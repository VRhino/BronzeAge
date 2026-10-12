// Convierte un punto de control del batch (`BATCH_CHECKPOINT_TICKS`) en una partida que el servidor, o `medicion-tiempo-real.ts`, pueden abrir.
//
//   npx tsx scripts/checkpoint-a-partida.ts <checkpoint.json> <directorio-destino> <gameId>
//
// Es la forma de tener una partida GRANDE para medir (p. ej. 500 bots al final de la Era I) sin esperar a jugarla. Escribe solo en el
// directorio destino.
import { readFileSync } from 'node:fs';
import { crearAlmacenEnDisco } from '../src/server/almacen/enDisco';
import { guardarPartida } from '../src/server/persistenciaPartida';
import { GameSession, type PartidaExportada } from '../src/session/gameSession';

const [checkpoint, destino, gameId] = process.argv.slice(2);
if (!checkpoint || !destino || !gameId) {
  console.error('uso: npx tsx scripts/checkpoint-a-partida.ts <checkpoint.json> <directorio-destino> <gameId>');
  process.exit(2);
}
const { partida } = JSON.parse(readFileSync(checkpoint, 'utf8')) as { partida: PartidaExportada };
partida.state.gameId = gameId;
const sesion = GameSession.importar(partida);
await guardarPartida(crearAlmacenEnDisco(destino), sesion, new Date().toISOString());
const e = sesion.getState();
console.log(`'${gameId}' escrita en ${destino}: v${e.version}, ${e.asentamientos.length} asentamientos, ${e.heroes.length} héroes, ${e.ejercitos.length} columnas, ${e.caravanas.length} caravanas`);
