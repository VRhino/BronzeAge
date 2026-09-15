// Piezas que comparten las dos superficies HTTP (`admin.ts`, `jugador.ts`): resolver quién pide algo y
// traducir las respuestas negativas a códigos de estado.
//
// La traducción a HTTP vive aquí y solo aquí. La DECISIÓN de si alguien puede hacer algo es de
// `acceso/rolesDePartida.ts` (superficie) y `session/comandos/autorizacion.ts` (comando concreto); esto es
// el cableado entre aquellas y Fastify.
import type { FastifyReply, FastifyRequest } from 'fastify';
import type { ContextoAutenticacion } from '../../acceso/servicioAutenticacion';
import { resolverSesion } from '../../acceso/servicioAutenticacion';
import { esVigente, type ActorDeInstancia } from '../../acceso/rolesDePartida';
import type { Usuario } from '../../acceso/tipos';
import { credencialOpcionalDesdeCabecera } from '../identidad/cabeceraAutorizacion';
import type { DirectorioDeAdministradores } from '../identidad/administradoresGlobales';
import type { ServidorDeBatalla } from '../identidad/servidoresDeBatalla';
import type { RegistroDePartidas } from '../registroDePartidas';
import type { RunnerDePartida } from '../runnerDePartida';
import type { HubDeDifusion } from '../difusion/hub';
import type { RegistroDeAuditoria } from '../auditoria';
import { idDeMapa, instanteDeTick } from '../../session/estado';
import type { Instante } from '../../domain/tiempo';

export interface DependenciasDeRutas {
  identidad: ContextoAutenticacion;
  administradores: DirectorioDeAdministradores;
  partidas: RegistroDePartidas;
  /** Reloj inyectado, como en el resto del proyecto — decide la vigencia de sesiones y membresías. */
  ahora: () => string;
  /** Conexiones WebSocket activas (Fase C5) — a quién difundir tras un comando o un tick. */
  hub: HubDeDifusion;
  /** Registro de auditoría de comandos (Fase E2). Se escribe desde `ejecutarComandoHttp`, que es el único
   * punto por el que pasan TODOS los comandos de las dos superficies, aceptados y rechazados. */
  auditoria: RegistroDeAuditoria;
  /** Código de invitación exigido en `POST /v1/registro`. `undefined` = registro abierto. */
  codigoRegistro?: string;
  /** Servidores de batalla de Conquest que pueden hablar por `/v1/batallas/*` (doc 02 §3.3). Vacío = ninguno. */
  servidoresBatalla: readonly ServidorDeBatalla[];
}

export interface ParametrosGameId {
  gameId: string;
}

export interface ResumenPartida {
  gameId: string;
  /** Instante de MUNDO de la partida (doc 10) — `instanteDeTick(estado.tick)`, derivado, no almacenado. La
   * ÚNICA referencia temporal del contrato (Fase D cerrada): viaja en TODA respuesta con resumen para que el
   * cliente sepa la hora de mundo. El `tick` interno del motor no sale de aquí. */
  instante: Instante;
  version: number;
  /** Identidad del mapa vigente (Fase C11, doc 9) — nunca el mapa en sí. Presente en TODA respuesta que
   * incluya un resumen (crear partida, tick, comando) para que el cliente sepa, sin una petición aparte, si
   * el mapa que tiene cacheado sigue siendo el vigente. Solo cambia si `regenerarMundo`/`forzar` reemplaza la
   * partida por otra semilla. */
  mapaId: string;
}

export function resumenDe(runner: RunnerDePartida): ResumenPartida {
  const estado = runner.getState();
  return {
    gameId: runner.gameId,
    instante: instanteDeTick(estado.tick),
    version: estado.version,
    mapaId: idDeMapa(estado.mapa),
  };
}

export function mensajeDe(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

/**
 * Resuelve el actor de una petición: sesión -> `Usuario`, más si la instancia lo reconoce como administrador
 * global y qué `Membresia` VIGENTE tiene en la partida indicada (si se indica alguna).
 *
 * Devuelve `undefined` sin sesión válida; nunca lanza. Una `Membresia` revocada (`hasta` en el pasado) se
 * ignora igual que si no existiera — el campo existía desde A6 y hasta C3 nadie lo miraba.
 */
export function resolverActor(
  request: FastifyRequest,
  deps: DependenciasDeRutas,
  gameId?: string
): { usuario: Usuario; actor: ActorDeInstancia } | undefined {
  const resuelto = resolverSesion(credencialOpcionalDesdeCabecera(request.headers.authorization), deps.identidad);
  if (!resuelto) return undefined;

  const ahora = deps.ahora();
  const membresia = gameId === undefined ? undefined : deps.identidad.repositorio.obtenerMembresia(resuelto.usuario.id, gameId);
  return {
    usuario: resuelto.usuario,
    actor: {
      usuarioId: resuelto.usuario.id,
      esAdministradorGlobal: deps.administradores.esAdministradorGlobal(resuelto.usuario.id),
      membresia: membresia && esVigente(membresia, ahora) ? membresia : undefined,
    },
  };
}

export function sinSesion(reply: FastifyReply) {
  return reply.code(401).send({ error: 'sesion ausente, invalida o expirada' });
}

export function sinPermiso(reply: FastifyReply, detalle: string) {
  return reply.code(403).send({ error: detalle });
}

export function partidaNoAbierta(reply: FastifyReply, gameId: string) {
  return reply.code(404).send({ error: `la partida '${gameId}' no está abierta en este proceso.` });
}
