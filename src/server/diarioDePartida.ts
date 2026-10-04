// Diario de comandos de una partida (Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md §5.1): cada mutación que subió
// la versión se AÑADE como una línea, en el orden en que salió de la cola serial. El snapshot se guarda una vez
// por tick y el diario se vacía tras cada guardado; al cargar, se repasa el diario sobre el último snapshot.
//
// A diferencia de `eventosDePartida.ts` (historial derivado), esto SÍ es fuente de verdad: lo aceptado desde el
// último guardado solo existe aquí. Por eso un fallo al anexar revierte el comando (`RunnerDePartida`).
//
// El repaso es posible porque los comandos son deterministas dado el estado: `ContextoComando` trae el instante
// derivado del tick, el RNG y los ids de la sesión, y lo que no sube la versión no los mueve (`GameSession.ejecutar`).
import type { AlmacenDeObjetos } from './almacen/almacenDeObjetos';
import type { GameSession } from '../session/gameSession';
import { REGISTRO_DIARIO, type TipoDiario } from '../session/comandos/registro';
import type { ManejadorComando } from '../session/comandos/tipos';

/** Una mutación aceptada. `v` es la versión RESULTANTE: identifica la línea y verifica el repaso. */
export interface LineaDiario {
  v: number;
  t: TipoDiario;
  a: string;
  p?: unknown;
}

export function claveDeDiario(gameId: string): string {
  return `${gameId}.diario.jsonl`;
}

export async function anexarAlDiario(almacen: AlmacenDeObjetos, gameId: string, lineas: readonly LineaDiario[]): Promise<void> {
  if (lineas.length === 0) return;
  await almacen.anexar(claveDeDiario(gameId), lineas.map((l) => `${JSON.stringify(l)}\n`).join(''));
}

export async function vaciarDiario(almacen: AlmacenDeObjetos, gameId: string): Promise<void> {
  await almacen.escribir(claveDeDiario(gameId), '');
}

/** Se lanza cuando el diario no se puede repasar: línea ilegible en medio, tipo desconocido, o una línea que no
 * reproduce la versión que anotó (no-determinismo o diario de otra partida). La partida NO se abre: servir un
 * mundo distinto del que se aceptó sería peor que no servir ninguno. */
export class DiarioIrrepetibleError extends Error {
  constructor(gameId: string, motivo: string) {
    super(`partida '${gameId}': el diario de comandos no se puede repasar — ${motivo}`);
    this.name = 'DiarioIrrepetibleError';
  }
}

/**
 * Repasa sobre `sesion` las líneas del diario con versión mayor que la suya, en orden. Las de versión menor o
 * igual ya están en el snapshot (corte entre «guardar» y «vaciar») y se saltan. Una última línea ilegible es un
 * corte a mitad de escritura y se descarta; una ilegible antes de otra válida es un error.
 */
export async function repasarDiario(almacen: AlmacenDeObjetos, sesion: GameSession): Promise<void> {
  const gameId = sesion.gameId;
  const contenido = await almacen.leer(claveDeDiario(gameId));
  if (!contenido) return;

  const crudas = contenido.split('\n').filter((l) => l.trim() !== '');
  crudas.forEach((cruda, i) => {
    let linea: LineaDiario;
    try {
      linea = JSON.parse(cruda) as LineaDiario;
    } catch {
      if (i === crudas.length - 1) return; // corte a mitad de la última escritura
      throw new DiarioIrrepetibleError(gameId, `línea ${i + 1} ilegible`);
    }
    const version = sesion.getState().version;
    if (linea.v <= version) return;
    const manejador = REGISTRO_DIARIO[linea.t] as ManejadorComando<unknown, unknown> | undefined;
    if (!manejador) throw new DiarioIrrepetibleError(gameId, `tipo desconocido '${linea.t}' en la línea ${i + 1}`);
    const r = sesion.ejecutar(manejador, linea.p, { actor: linea.a });
    if (!r.ok || r.version !== linea.v) {
      throw new DiarioIrrepetibleError(
        gameId,
        `'${linea.t}' (línea ${i + 1}) anotó la versión ${linea.v} y al repasarlo dio ${r.ok ? `la ${r.version}` : `rechazo ${r.codigoError}`}`
      );
    }
  });
}
