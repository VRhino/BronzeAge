// Historial de eventos de dominio de una partida, en un JSONL append-only hermano del snapshot.
//
// MISMO patrón y mismos motivos que `auditoria.ts` (que ya dejó escrito por qué un historial no va DENTRO
// del estado: crece sin techo, se reescribe entero en cada guardado, viaja entero en cada lectura). De
// hecho `eventosDominio` era el caso ORIGINAL de ese problema y el que más pesaba —300 KB (66 %) de un
// snapshot de 35 000 ticks, reescritos en cada comando aceptado— y la razón por la que C13 tuvo que añadirle
// un cursor. Sacarlo del snapshot es terminar esa frase: aquí se añade una línea por evento y el snapshot
// vuelve a ser plano.
//
// Reparto de responsabilidad: el SNAPSHOT es la fuente de verdad del estado y de la `version`. Este archivo
// es el historial derivado de esas versiones — si una línea se pierde por un fallo de escritura, el juego no
// se rompe (se grita por `stderr`, y el cursor de `RunnerDePartida` reintenta desde donde iba), a diferencia
// del estado, cuyo fallo de escritura sí revierte el comando.
//
// APPEND, no reescritura, por lo mismo que la auditoría: copiar un log que crece sin límite para añadirle un
// renglón es justo lo que un append-only existe para evitar. JSONL y no un array JSON para que una última
// línea a medias (corte a mitad de escritura) se descarte sola sin llevarse el resto por delante.
import { appendFile, mkdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import type { EventoDominioConVersion } from '../session/estado';

/** Sufijo propio, como `.auditoria.jsonl`, para que un `readdir` del directorio de datos distinga de un
 * vistazo el snapshot (`<gameId>.json`) de sus dos hermanos append-only. */
export function rutaDeEventos(directorio: string, gameId: string): string {
  return join(directorio, `${gameId}.eventos.jsonl`);
}

/** Añade eventos al final del historial, uno por línea, EN EL ORDEN DADO (que `RunnerDePartida` pasa
 * cronológico — más viejo primero). No reescribe nada. Crea el directorio si no existe. */
export async function anexarEventos(directorio: string, gameId: string, eventos: readonly EventoDominioConVersion[]): Promise<void> {
  if (eventos.length === 0) return;
  await mkdir(directorio, { recursive: true });
  await appendFile(rutaDeEventos(directorio, gameId), eventos.map((e) => `${JSON.stringify(e)}\n`).join(''), 'utf-8');
}

/**
 * Lee el historial completo de una partida, MÁS NUEVO PRIMERO — el mismo orden en que vive
 * `GameSessionState.eventosDominio` (`exito()` antepone), para que `cargarPartida` lo devuelva listo para
 * asignar. `[]` si aún no hay archivo (partida sin comandos todavía, no un error).
 *
 * `hasta` descarta los eventos con `version` MAYOR: defensa contra líneas "del futuro" que quedaran de un
 * comando cuyo snapshot se revirtió después de haber anexado sus eventos (ver
 * `RunnerDePartida.aplicarYPersistir` — se anexa DESPUÉS de persistir justo para minimizar esta ventana, pero
 * un corte entre las dos escrituras aún la deja abierta). Una línea ilegible se descarta, como en
 * `auditoria.ts`.
 */
export async function leerEventos(directorio: string, gameId: string, hasta = Infinity): Promise<EventoDominioConVersion[]> {
  let contenido: string;
  try {
    contenido = await readFile(rutaDeEventos(directorio, gameId), 'utf-8');
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === 'ENOENT') return [];
    throw err;
  }

  const eventos: EventoDominioConVersion[] = [];
  for (const linea of contenido.split('\n')) {
    if (linea.trim() === '') continue;
    try {
      const evento = JSON.parse(linea) as EventoDominioConVersion;
      if (evento.version <= hasta) eventos.push(evento);
    } catch {
      /* línea a medias por un corte de escritura: se descarta, el resto del historial sigue siendo válido */
    }
  }
  return eventos.reverse(); // el archivo va cronológico; el estado va más-nuevo-primero
}
