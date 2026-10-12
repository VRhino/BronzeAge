// Histórico de batallas de Unity de una partida, en un JSONL append-only hermano del snapshot.
//
// Una batalla cerrada se conserva en `estado.batallas` el tiempo que dice `BATALLA.conservarCerradasMinutos` (el
// resultado idempotente y el ticket recuperable lo piden); después el tick la saca del estado y `RunnerDePartida` la
// anexa aquí. Mismo motivo que `eventosDePartida.ts`: un historial no va dentro del snapshot, que se reescribe entero en
// cada guardado y aquí cada asedio lleva congelado el mapa de la plaza.
//
// Cada línea lleva el `gameId`: la batalla ya es única entre partidas (`idDeBatalla`), pero el histórico se puede juntar
// con el de otras partidas y entonces hace falta saber de cuál es.
//
// Es historial, no fuente de verdad: el runner lo escribe ANTES de anotar en el diario el tick que retira la batalla, así
// que un corte deja una línea de más (se dedupe al leer por `batalla.id`), nunca una de menos.
import type { AlmacenDeObjetos } from './almacen/almacenDeObjetos';
import type { Batalla } from '../session/batallas';

export interface RegistroHistoricoDeBatalla {
  gameId: string;
  /** Reloj de pared (ISO) del momento en que se anexó. */
  archivadaEn: string;
  batalla: Batalla;
}

export function claveDeHistoricoDeBatallas(gameId: string): string {
  return `${gameId}.batallas.jsonl`;
}

export async function anexarHistoricoDeBatallas(almacen: AlmacenDeObjetos, gameId: string, batallas: readonly Batalla[], archivadaEn: string): Promise<void> {
  if (batallas.length === 0) return;
  const lineas = batallas.map((batalla) => `${JSON.stringify({ gameId, archivadaEn, batalla } satisfies RegistroHistoricoDeBatalla)}\n`);
  await almacen.anexar(claveDeHistoricoDeBatallas(gameId), lineas.join(''));
}

/** Las batallas archivadas, en el orden en que se anexaron, sin repetir `batalla.id` (queda la primera). Una línea ilegible
 * (corte a mitad de escritura) se descarta. */
export async function leerHistoricoDeBatallas(almacen: AlmacenDeObjetos, gameId: string): Promise<RegistroHistoricoDeBatalla[]> {
  const contenido = await almacen.leer(claveDeHistoricoDeBatallas(gameId));
  if (contenido === null) return [];
  const vistas = new Set<string>();
  const registros: RegistroHistoricoDeBatalla[] = [];
  for (const linea of contenido.split('\n')) {
    if (linea.trim() === '') continue;
    try {
      const registro = JSON.parse(linea) as RegistroHistoricoDeBatalla;
      if (vistas.has(registro.batalla.id)) continue;
      vistas.add(registro.batalla.id);
      registros.push(registro);
    } catch {
      /* línea a medias: se descarta y el resto sigue siendo válido */
    }
  }
  return registros;
}
