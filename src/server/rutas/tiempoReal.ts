// WebSocket único por jugador, con canales suscribibles (Fase C5, doc 6 §2): "una única conexión permanente
// por jugador, con canales lógicos multiplexados encima" en vez de una conexión por pantalla.
//
// Autenticación ANTES de completar el handshake (`preValidation`, en un `register` aparte para que el hook
// no se filtre a las demás rutas): igual que las superficies HTTP, sin sesión válida ni membresía de jugador
// no hay conexión — el rechazo llega como una respuesta HTTP normal (401/403) a la petición de upgrade, no
// como un socket que se abre y se cierra solo sin explicación.
//
// Protocolo (JSON sobre el socket):
//   cliente -> {"accion":"suscribir"|"desuscribir","canal":"mapa/general"|"asentamiento/<id>"}
//   servidor -> {"tipo":"suscrito"|"desuscrito","canal":...}
//             | {"tipo":"error","canal"?:...,"error":...}
//             | {"tipo":"evento","canal":...,"evento": EventoDominio}
//
// SIN mensaje sintético de "conectado": el evento `open` nativo de WebSocket ya se lo dice al cliente, y
// como la autenticación ocurre en `preValidation` —ANTES de que el *handshake* se complete—, para cuando
// `open` dispara la conexión ya está autenticada. Un frame de la app encima de eso no añadía información.
//
// Se descartó explícitamente por una razón más concreta que "es redundante": mandarlo de forma SÍNCRONA en
// el mismo tick en que el handler de la ruta arranca compite con que el cliente termine de engancharse al
// evento `message` — descubierto con `injectWS` en los tests (el primer mensaje se perdía siempre), pero es
// una carrera real del transporte, no una peculiaridad del arnés de pruebas. Evitarla del todo (no mandar
// nada hasta que el cliente inicie) es más simple y más robusto que retrasar el envío con un `setImmediate`.
//
// Al reconectar se pierden las suscripciones (doc 6 §2): el cliente debe re-suscribirse solo. No hay estado
// de suscripción que sobreviva al cierre del socket — más simple que reconstruir "qué tenía suscrito", y
// coherente con que las suscripciones describen QUÉ se quiere ver, no un historial que recuperar.
import type { FastifyInstance, FastifyRequest } from 'fastify';
import type WebSocket from 'ws';
import { puedeJugar } from '../../acceso/rolesDePartida';
import { puedeSuscribirseA } from '../../session/canales';
import { partidaNoAbierta, resolverActor, sinPermiso, sinSesion, type DependenciasDeRutas, type ParametrosGameId } from './contexto';

interface MensajeCliente {
  accion: 'suscribir' | 'desuscribir';
  canal: string;
}

function esMensajeCliente(valor: unknown): valor is MensajeCliente {
  if (typeof valor !== 'object' || valor === null) return false;
  const v = valor as Record<string, unknown>;
  return (v.accion === 'suscribir' || v.accion === 'desuscribir') && typeof v.canal === 'string';
}

/** El navegador no puede mandar cabeceras propias en el handshake de `WebSocket` — solo clientes no
 * navegador (o `injectWS` en los tests) pueden. Por eso se admite además `?sesion=<id>` como alternativa al
 * `Authorization: sesion <id>` habitual; ambas caminos llegan al mismo `resolverActor`. */
function conCredencialDeQuery(request: FastifyRequest): FastifyRequest {
  if (request.headers.authorization) return request;
  const sesionId = (request.query as Record<string, unknown> | undefined)?.sesion;
  if (typeof sesionId !== 'string') return request;
  return { ...request, headers: { ...request.headers, authorization: `sesion ${sesionId}` } } as FastifyRequest;
}

export function registrarRutaDeTiempoReal(app: FastifyInstance, deps: DependenciasDeRutas): void {
  const { hub } = deps;
  app.register(async (scoped) => {
    scoped.addHook('preValidation', async (request, reply) => {
      const { gameId } = request.params as ParametrosGameId;
      const resuelto = resolverActor(conCredencialDeQuery(request), deps, gameId);
      if (!resuelto) return sinSesion(reply);
      if (!puedeJugar(resuelto.actor)) return sinPermiso(reply, 'sin membresia de jugador en esta partida');
      if (!deps.partidas.obtener(gameId)) return partidaNoAbierta(reply, gameId);
    });

    scoped.get<{ Params: ParametrosGameId }>(
      '/jugador/partidas/:gameId/tiempo-real',
      { websocket: true },
      (socket: WebSocket, request) => {
        const { gameId } = request.params;
        // El hook ya validó sesión, rol y partida abierta; se vuelve a resolver aquí porque Fastify no ofrece
        // una costura directa para pasar el resultado de un hook al handler de WebSocket. Es barato (lookups
        // en memoria), no una segunda ronda de I/O.
        const resuelto = resolverActor(conCredencialDeQuery(request), deps, gameId)!;
        const jugadorId = resuelto.actor.membresia!.jugadorId!;

        hub.conectar(gameId, jugadorId, socket);

        socket.on('message', (data: Buffer) => {
          let mensaje: unknown;
          try {
            mensaje = JSON.parse(data.toString());
          } catch {
            socket.send(JSON.stringify({ tipo: 'error', error: 'mensaje no es JSON valido' }));
            return;
          }
          if (!esMensajeCliente(mensaje)) {
            socket.send(JSON.stringify({ tipo: 'error', error: 'se esperaba {accion, canal}' }));
            return;
          }

          if (mensaje.accion === 'desuscribir') {
            hub.desuscribir(gameId, socket, mensaje.canal);
            socket.send(JSON.stringify({ tipo: 'desuscrito', canal: mensaje.canal }));
            return;
          }

          // La partida pudo cerrarse entre la conexión y este mensaje (`forzar` la reemplaza) — sin estado
          // que consultar, no hay nada que autorizar.
          const runner = deps.partidas.obtener(gameId);
          if (!runner || !puedeSuscribirseA(runner.getState(), jugadorId, mensaje.canal)) {
            socket.send(JSON.stringify({ tipo: 'error', canal: mensaje.canal, error: 'no autorizado' }));
            return;
          }
          hub.suscribir(gameId, socket, mensaje.canal);
          socket.send(JSON.stringify({ tipo: 'suscrito', canal: mensaje.canal }));
        });

        socket.on('close', () => hub.desconectar(gameId, socket));
      }
    );
  });
}
