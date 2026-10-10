// Compresión `permessage-deflate` del WebSocket de tiempo real (contrato 03 §13.2). Es una extensión de transporte que negocia el propio
// protocolo: un cliente que no la ofrece recibe los mensajes sin comprimir, así que activarla no rompe a ninguno.
//
// Comprime sobre todo las FOTOS (admin ≈1,2 MiB con 500 héroes, mapa ≈70 KiB); un delta de unos pocos KiB no compensa la CPU de zlib, y por
// eso hay un umbral: por debajo de `umbralBytes` el mensaje viaja tal cual. Sin contexto compartido entre mensajes (`*NoContextTakeover`):
// con cientos de sockets el diccionario de 32 KiB por socket y sentido es memoria fija que no se quiere pagar, y los mensajes son
// independientes de todos modos.
import type { PerMessageDeflateOptions } from 'ws';

export interface OpcionesCompresionWs {
  /** Mensajes de menos de esto no se comprimen. */
  umbralBytes?: number;
  /** Nivel de zlib (1 rápido … 9 máximo). */
  nivel?: number;
}

export const UMBRAL_POR_DEFECTO = 1024;
export const NIVEL_POR_DEFECTO = 3;

/** Las opciones de `ws` para esta configuración, o `false` si la compresión está apagada. Sin configurar, activada con los valores por defecto. */
export function perMessageDeflateDe(compresion: OpcionesCompresionWs | false | undefined): PerMessageDeflateOptions | false {
  if (compresion === false) return false;
  return {
    threshold: compresion?.umbralBytes ?? UMBRAL_POR_DEFECTO,
    zlibDeflateOptions: { level: compresion?.nivel ?? NIVEL_POR_DEFECTO },
    serverNoContextTakeover: true,
    clientNoContextTakeover: true,
  };
}

/** Lee `WS_COMPRESION` (`off`/`0`/`false` la apaga), `WS_COMPRESION_UMBRAL` y `WS_COMPRESION_NIVEL` del entorno. */
export function compresionDeEntorno(env: Record<string, string | undefined>): OpcionesCompresionWs | false | undefined {
  const interruptor = env['WS_COMPRESION']?.trim().toLowerCase();
  if (interruptor === 'off' || interruptor === '0' || interruptor === 'false') return false;
  const umbral = env['WS_COMPRESION_UMBRAL'] ? Number(env['WS_COMPRESION_UMBRAL']) : undefined;
  const nivel = env['WS_COMPRESION_NIVEL'] ? Number(env['WS_COMPRESION_NIVEL']) : undefined;
  if (umbral === undefined && nivel === undefined) return undefined;
  return {
    ...(umbral !== undefined && Number.isFinite(umbral) && umbral >= 0 ? { umbralBytes: umbral } : {}),
    ...(nivel !== undefined && Number.isInteger(nivel) && nivel >= 1 && nivel <= 9 ? { nivel } : {}),
  };
}
