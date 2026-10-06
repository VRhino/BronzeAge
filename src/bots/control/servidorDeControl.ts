// El canal de control del servicio de bots: un WebSocket al que se conecta el cliente admin (protocolo en `contrato.ts`).
// Sin sesiones ni usuarios: un token compartido (`BOTS_TOKEN`) que el cliente manda como primera orden. Hasta entonces no se
// le cuenta nada; a los 5 s sin autenticarse se le cierra. También contesta `GET /salud` con 200, para el health check del host.
import { createServer } from 'node:http';
import { timingSafeEqual } from 'node:crypto';
import { WebSocketServer, type WebSocket } from 'ws';
import type { AddressInfo } from 'node:net';
import type { ComandoConId, MensajeDeServicio } from './contrato';
import type { ServicioDeBots } from './servicio';

const ESPERA_AUTENTICACION_MS = 5_000;
/** Si un cliente no lee, no se le sigue mandando estado: se le salta. */
const MAX_PENDIENTE = 1_000_000;

export interface ServidorDeControl {
  puerto: number;
  cerrar(): Promise<void>;
}

const igual = (a: string, b: string): boolean => a.length === b.length && timingSafeEqual(Buffer.from(a), Buffer.from(b));

export async function servidorDeControl(servicio: ServicioDeBots, opciones: { token: string; puerto: number; host?: string }): Promise<ServidorDeControl> {
  const http = createServer((req, res) => {
    res.writeHead(req.url === '/salud' ? 200 : 404, { 'content-type': 'application/json' }).end(req.url === '/salud' ? JSON.stringify({ fase: servicio.estado().fase }) : '{}');
  });
  const ws = new WebSocketServer({ server: http, path: '/control', maxPayload: 64 * 1024 });

  ws.on('connection', (socket: WebSocket) => {
    let desuscribir: (() => void) | undefined;
    const enviar = (m: MensajeDeServicio) => {
      if (socket.readyState === socket.OPEN && socket.bufferedAmount < MAX_PENDIENTE) socket.send(JSON.stringify(m));
    };
    const espera = setTimeout(() => socket.close(1008, 'sin autenticar'), ESPERA_AUTENTICACION_MS);
    socket.on('close', () => {
      clearTimeout(espera);
      desuscribir?.();
    });

    socket.on('message', async (datos) => {
      let orden: ComandoConId;
      try {
        orden = JSON.parse(String(datos)) as ComandoConId;
        if (typeof orden.id !== 'number' || typeof orden.accion !== 'string') throw new Error();
      } catch {
        enviar({ tipo: 'respuesta', id: -1, ok: false, error: 'se esperaba {id, accion, ...}' });
        return;
      }
      if (orden.accion === 'autenticar') {
        if (typeof orden.token !== 'string' || !igual(orden.token, opciones.token)) {
          console.warn(`[bots] token de control rechazado (recibidos ${typeof orden.token === 'string' ? orden.token.length : 0} caracteres, esperados ${opciones.token.length})`);
          enviar({ tipo: 'respuesta', id: orden.id, ok: false, error: 'token incorrecto' });
          socket.close(1008, 'token incorrecto');
          return;
        }
        clearTimeout(espera);
        if (!desuscribir) {
          enviar({ tipo: 'respuesta', id: orden.id, ok: true });
          enviar({ tipo: 'historial', ...servicio.historial() });
          enviar({ tipo: 'estado', estado: servicio.estado() });
          desuscribir = servicio.suscribir(enviar);
        }
        return;
      }
      if (!desuscribir) {
        enviar({ tipo: 'respuesta', id: orden.id, ok: false, error: 'hay que autenticarse primero' });
        return;
      }
      try {
        enviar({ tipo: 'respuesta', id: orden.id, ok: true, datos: await servicio.manejar(orden) });
      } catch (err) {
        enviar({ tipo: 'respuesta', id: orden.id, ok: false, error: err instanceof Error ? err.message : String(err) });
      }
    });
  });

  await new Promise<void>((resolver) => http.listen(opciones.puerto, opciones.host ?? '0.0.0.0', resolver));
  return {
    puerto: (http.address() as AddressInfo).port,
    cerrar: () =>
      new Promise<void>((resolver) => {
        for (const cliente of ws.clients) cliente.terminate();
        ws.close(() => http.close(() => resolver()));
      }),
  };
}
