// WebSocket de tiempo real (Fase C5, doc 6 §2): una única conexión permanente por cliente, con canales lógicos multiplexados encima.
//
// El protocolo (mensajes, canales de eventos y de vista, foto/delta/resync, errores y migración) está en
// `Docs/Coordinacion/03_Tiempo_real_por_segmentos.md` y, para los canales de eventos, en `02_Contrato_Comunicacion_Servidor_Cliente.md` §2:
// ahí es donde se mantiene, para que no viva duplicado en prosa aquí. Este archivo es el transporte:
//
//  - `/jugador/partidas/:gameId/tiempo-real`: canales de eventos (`mapa/general`, `asentamiento/<id>`, `batalla/<id>`, `heroe/<id>`), el
//    mensaje `tick` y los canales de vista de jugador (`vista/mapa`, `vista/heroe`; `vista/asentamiento/<id>` es de la fase 3 y hoy se
//    anuncia como `canal_desconocido`).
//  - `/admin/partidas/:gameId/tiempo-real`: canales de vista de administración (`admin/*`); no tiene canales de eventos.
//
// Autenticación ANTES de completar el handshake (`preValidation`, en un `register` aparte para que el hook no se filtre a las demás rutas):
// sin sesión válida ni el permiso que toque no hay conexión — el rechazo llega como una respuesta HTTP normal (401/403) a la petición de
// upgrade, no como un socket que se abre y se cierra solo sin explicación.
//
// SIN mensaje sintético de "conectado": el evento `open` nativo de WebSocket ya se lo dice al cliente, y la autenticación ocurre en
// `preValidation`, ANTES de que el handshake se complete. Mandarlo de forma síncrona compite con que el cliente termine de engancharse a
// `message` (se perdía el primer mensaje en los tests, y es una carrera real del transporte).
//
// Al reconectar se pierden las suscripciones (doc 6 §2): el cliente debe re-suscribirse solo.
import type { FastifyInstance, FastifyRequest } from 'fastify';
import type WebSocket from 'ws';
import { puedeAdministrar, puedeJugar } from '../../acceso/rolesDePartida';
import { puedeSuscribirseA } from '../../session/canales';
import { analizarCanal } from '../../session/vistas/segmentos';
import type { CodigoDeError } from '../difusion/difusorDeVistas';
import { partidaNoAbierta, resolverActor, sinPermiso, sinSesion, type DependenciasDeRutas, type ParametrosGameId } from './contexto';

type Accion = 'suscribir' | 'desuscribir' | 'resincronizar';

interface MensajeCliente {
  accion: Accion;
  canal: string;
  /** Solo en `suscribir` a un canal de vista (03 §13.5): entregar un delta cada N ticks. El servidor lo recorta a su tope. */
  ritmo?: number;
}

function esMensajeCliente(valor: unknown): valor is MensajeCliente {
  if (typeof valor !== 'object' || valor === null) return false;
  const v = valor as Record<string, unknown>;
  return (
    (v.accion === 'suscribir' || v.accion === 'desuscribir' || v.accion === 'resincronizar') &&
    typeof v.canal === 'string' &&
    (v.ritmo === undefined || typeof v.ritmo === 'number')
  );
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

const CANAL_DE_VISTA = /^(vista|admin)\//;

function enviarError(socket: WebSocket, canal: string | undefined, error: string, codigo: CodigoDeError): void {
  socket.send(JSON.stringify({ tipo: 'error', ...(canal === undefined ? {} : { canal }), error, codigo }));
}

/** Parsea y valida el mensaje del cliente; contesta el error por su cuenta y devuelve `undefined` si no vale. */
function leerMensaje(socket: WebSocket, data: Buffer): MensajeCliente | undefined {
  let mensaje: unknown;
  try {
    mensaje = JSON.parse(data.toString());
  } catch {
    enviarError(socket, undefined, 'mensaje no es JSON valido', 'formato');
    return undefined;
  }
  if (!esMensajeCliente(mensaje)) {
    enviarError(socket, undefined, 'se esperaba {accion, canal, ritmo?}', 'formato');
    return undefined;
  }
  return mensaje;
}

export function registrarRutaDeTiempoReal(app: FastifyInstance, deps: DependenciasDeRutas): void {
  const { hub, difusorDeVistas } = deps;
  // Apagar el servidor (un despliegue) cierra todos los sockets, y eso no es que los jugadores cierren el cliente: no se
  // desconecta a nadie. Sin esto, cada despliegue los sacaría del mundo, y la escritura llegaría con la partida cerrada.
  let cerrando = false;
  app.addHook('preClose', async () => {
    cerrando = true;
  });
  /** Manda el comando de presencia del héroe de ese jugador, si tiene héroe. Un fallo al anotarlo se grita y no tumba el
   * socket: la presencia se corrige sola en la siguiente conexión. */
  const presencia = (gameId: string, jugadorId: string, tipo: 'conectarse' | 'desconectarse') => {
    if (cerrando) return;
    const runner = deps.partidas.obtener(gameId);
    const heroe = runner?.getState().heroes.find((h) => h.jugadorId === jugadorId);
    if (!runner || !heroe) return;
    runner
      .ejecutar(tipo, { heroeId: heroe.id }, heroe.id)
      .then((r) => hub.difundir(gameId, r.eventos))
      .catch((err) => console.error(`[presencia] ${tipo} de ${heroe.id} en '${gameId}':`, err));
  };

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
        // La membresía puede revocarse mientras el socket está abierto: se vuelve a mirar en cada flush (03 §7.2).
        const conexionDeVistas = {
          audiencia: { tipo: 'jugador' as const, jugadorId },
          derecho: () => {
            const r = resolverActor(conCredencialDeQuery(request), deps, gameId);
            return r !== undefined && puedeJugar(r.actor);
          },
        };
        // Presencia (Doc 1.10.6, D33): estar conectado es tener el cliente abierto. Abrirlo trae al héroe de vuelta al
        // mundo; cerrar el último socket lo desconecta, y sale del mundo 2:30 después si no vuelve antes.
        presencia(gameId, jugadorId, 'conectarse');

        socket.on('message', (data: Buffer) => {
          const mensaje = leerMensaje(socket, data);
          if (!mensaje) return;

          if (mensaje.accion === 'desuscribir') {
            hub.desuscribir(gameId, socket, mensaje.canal);
            difusorDeVistas.desuscribir(gameId, socket, mensaje.canal);
            socket.send(JSON.stringify({ tipo: 'desuscrito', canal: mensaje.canal }));
            return;
          }
          // Canales de vista: foto y deltas por el difusor. Los `admin/*` y los `vista/*` que el servidor aún no tiene no existen para un
          // jugador: ni se revela su nombre.
          if (analizarCanal(mensaje.canal, conexionDeVistas.audiencia) || mensaje.accion === 'resincronizar') {
            const fallo =
              mensaje.accion === 'resincronizar'
                ? difusorDeVistas.resincronizar(gameId, socket, mensaje.canal)
                : difusorDeVistas.suscribir(gameId, socket, conexionDeVistas, mensaje.canal, mensaje.ritmo);
            if (fallo) enviarError(socket, mensaje.canal, fallo.error, fallo.codigo);
            return;
          }
          if (CANAL_DE_VISTA.test(mensaje.canal)) {
            enviarError(socket, mensaje.canal, 'canal desconocido', 'canal_desconocido');
            return;
          }

          // La partida pudo cerrarse entre la conexión y este mensaje (`forzar` la reemplaza) — sin estado
          // que consultar, no hay nada que autorizar.
          const runner = deps.partidas.obtener(gameId);
          const heroe = runner?.getState().heroes.find((h) => h.jugadorId === jugadorId);
          if (!runner || !heroe || !puedeSuscribirseA(runner.getState(), heroe.id, mensaje.canal)) {
            enviarError(socket, mensaje.canal, 'no autorizado', 'no_autorizado');
            return;
          }
          hub.suscribir(gameId, socket, mensaje.canal);
          socket.send(JSON.stringify({ tipo: 'suscrito', canal: mensaje.canal }));
        });

        socket.on('close', () => {
          hub.desconectar(gameId, socket);
          difusorDeVistas.desconectar(gameId, socket);
          if (!hub.sigueConectado(gameId, jugadorId)) presencia(gameId, jugadorId, 'desconectarse');
        });
      }
    );
  });

  // --- Administración: canales de vista `admin/*` (03 §3) ---
  app.register(async (scoped) => {
    scoped.addHook('preValidation', async (request, reply) => {
      const { gameId } = request.params as ParametrosGameId;
      const resuelto = resolverActor(conCredencialDeQuery(request), deps, gameId);
      if (!resuelto) return sinSesion(reply);
      if (!puedeAdministrar(resuelto.actor)) return sinPermiso(reply, 'sin rol de administracion en esta partida');
      if (!deps.partidas.obtener(gameId)) return partidaNoAbierta(reply, gameId);
    });

    scoped.get<{ Params: ParametrosGameId }>('/admin/partidas/:gameId/tiempo-real', { websocket: true }, (socket: WebSocket, request) => {
      const { gameId } = request.params;
      const credencial = conCredencialDeQuery(request);
      // El permiso de administración puede cambiar en caliente: se vuelve a mirar en cada flush, no solo al conectar (03 §7.2).
      const conexion = {
        audiencia: { tipo: 'admin' as const },
        derecho: () => {
          const r = resolverActor(credencial, deps, gameId);
          return r !== undefined && puedeAdministrar(r.actor);
        },
      };

      socket.on('message', (data: Buffer) => {
        const mensaje = leerMensaje(socket, data);
        if (!mensaje) return;

        if (mensaje.accion === 'desuscribir') {
          difusorDeVistas.desuscribir(gameId, socket, mensaje.canal);
          socket.send(JSON.stringify({ tipo: 'desuscrito', canal: mensaje.canal }));
          return;
        }
        const fallo =
          mensaje.accion === 'resincronizar'
            ? difusorDeVistas.resincronizar(gameId, socket, mensaje.canal)
            : difusorDeVistas.suscribir(gameId, socket, conexion, mensaje.canal, mensaje.ritmo);
        if (fallo) enviarError(socket, mensaje.canal, fallo.error, fallo.codigo);
      });

      socket.on('close', () => difusorDeVistas.desconectar(gameId, socket));
    });
  });
}
