// Sirve el mapa como ASSET, no como estado (Fase C11, doc 9). Compartido por las dos superficies —el mapa no
// es audiencia-dependiente, todos ven el mismo terreno— igual que `ejecutarComandoHttp` en `comandos.ts`
// comparte lo que no varía entre `/admin/*` y `/jugador/*`; cada ruta se registra por su lado porque la
// autorización de ENTRAR a la partida sí difiere (administración vs. membresía de jugador).
//
// `:mapaId` en la URL es puramente un cache-buster para el cliente: el handler lo IGNORA a propósito y sirve
// siempre el mapa actual de `runner`. No hay historial de mapas que servir —solo existe el vigente—, así que
// no hace falta validar el id contra nada; es exactamente el mismo patrón que un asset estático con hash en
// el nombre de archivo (`app.a3f9c2.js`): el hash no se interpreta, solo referencia una versión.
import type { FastifyReply } from 'fastify';
import type { RunnerDePartida } from '../runnerDePartida';
import { ESQUEMA_SESION_AUTH } from '../openapi';
import { PARAMS_GAME_ID_MAPA } from './esquemas';

const SEGURIDAD_MAPA = [{ [ESQUEMA_SESION_AUTH]: [] }];

export const ESQUEMA_MAPA = {
  description:
    'El mapa generado de la partida (terreno, recursos, ríos, bosques) — Fase C11. NUNCA cambia ' +
    'durante la partida (se deriva de la seed), así que la respuesta es cacheable para siempre: ' +
    '`Cache-Control: immutable`. El segmento `:mapaId` de la URL solo sirve para que el CLIENTE sepa cuándo ' +
    'invalidar su caché (cambia si `regenerarMundo` reemplaza la partida) — el servidor lo ignora y siempre ' +
    'devuelve el mapa vigente. Cuerpo no modelado en este esquema por su tamaño (Fase C6, doc 4).',
  tags: ['mapa'],
  security: SEGURIDAD_MAPA,
  params: PARAMS_GAME_ID_MAPA,
  response: { 401: { type: 'object', properties: { error: { type: 'string' } }, required: ['error'] } },
} as const;

/** Un año: sin caducidad real (el mapa no cambia jamás para un `mapaId` dado), pero un valor finito es más
 * conservador que `max-age` sin límite y sigue siendo, a efectos prácticos, "para siempre". */
const UN_ANIO_EN_SEGUNDOS = 60 * 60 * 24 * 365;

export function enviarMapa(reply: FastifyReply, runner: RunnerDePartida): void {
  reply.header('Cache-Control', `private, max-age=${UN_ANIO_EN_SEGUNDOS}, immutable`);
  reply.send(runner.getState().mapa);
}
