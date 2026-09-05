// Superficie de ADMINISTRACIÓN (`/admin/*`), Fase C3. Gobierna la partida como objeto: crearla/reabrirla,
// avanzar su tick, inspeccionar su estado completo.
//
// Requiere `administrador_global` (para lo que es de instancia) o una `Membresia` de administración en esa
// partida concreta (`administrador_partida`/`moderador`). La política de qué rol alcanza qué está en
// `acceso/rolesDePartida.ts`; aquí solo se cablea a rutas y códigos HTTP.
//
// Antes de C3 estos tres endpoints vivían sin prefijo y SIN autenticar (`POST /partidas`, `/tick`,
// `GET /partidas/:gameId`): cualquiera con acceso a red podía crear una partida, avanzarla o leerla entera.
// Separar las superficies es lo que permite cerrarlos sin romper la de jugador.
import type { FastifyInstance } from 'fastify';
import type { RegionId } from '../../domain/types';
import { esVigente, puedeAdministrar, puedeCrearPartida, puedeDescartarPartida, puedeGestionarMembresias, rolEnPartida } from '../../acceso/rolesDePartida';
import type { RolTecnico } from '../../acceso/tipos';
import type { ActorDeComando } from '../../session/comandos/autorizacion';
import { PartidaYaAbiertaError } from '../registroDePartidas';
import { recogerMetricas } from '../metricas';
import { eventosDesde, vistaAdminDeEstado } from '../../session/estado';
import { ESQUEMA_SESION_AUTH } from '../openapi';
import { auditarRechazoDeEsquema, ejecutarComandoHttp, ESQUEMA_EJECUTAR_COMANDO, type EjecutarComandoBody } from './comandos';
import { enviarMapa, ESQUEMA_MAPA } from './mapa';
import { ERROR_RESPUESTA, PARAMS_GAME_ID, QUERY_DESDE, RESUMEN_PARTIDA_RESPUESTA } from './esquemas';
import {
  mensajeDe,
  partidaNoAbierta,
  resolverActor,
  resumenDe,
  sinPermiso,
  sinSesion,
  type DependenciasDeRutas,
  type ParametrosGameId,
} from './contexto';

const SEGURIDAD_ADMIN = [{ [ESQUEMA_SESION_AUTH]: [] }];

interface CrearPartidaBody {
  gameId: string;
  seed: number;
  region?: RegionId;
  /** Descarta la partida abierta en este proceso (si la hay) y crea una limpia — operación destructiva: la
   * interfaz debe confirmarlo con el usuario antes de pedirlo. Exige `administrador_partida` o
   * `administrador_global`; un `moderador` NO puede (doc 5: sin acceso a regeneración de mundo). */
  forzar?: boolean;
}

const REGIONES: readonly RegionId[] = ['greciaContinental', 'anatolia', 'egeo', 'nilo', 'mesopotamia'];

const ESQUEMA_LISTAR_PARTIDAS = {
  description:
    'Partidas descubribles en este despliegue (Fase C12): lee el directorio de snapshots, no solo lo abierto ' +
    'en este proceso — una partida guardada antes de un reinicio sigue apareciendo. Exige administrador_global, ' +
    'mismo criterio que crear una partida: no hay Membresia de administración de partida posible sobre algo ' +
    'que todavía no se ha abierto en este proceso.',
  tags: ['admin'],
  security: SEGURIDAD_ADMIN,
  response: {
    200: {
      type: 'object',
      properties: {
        partidas: {
          type: 'array',
          items: {
            type: 'object',
            properties: {
              gameId: { type: 'string' },
              instante: { type: 'number' },
              version: { type: 'number' },
              mapaId: { type: 'string' },
              guardadoEn: { type: 'string' },
            },
            required: ['gameId', 'instante', 'version', 'mapaId', 'guardadoEn'],
          },
        },
      },
      required: ['partidas'],
    },
    401: ERROR_RESPUESTA,
    403: ERROR_RESPUESTA,
  },
} as const;

const ESQUEMA_CREAR_PARTIDA = {
  description: 'Crea una partida nueva, o retoma la que ya hubiera en disco para ese gameId. Exige administrador_global.',
  tags: ['admin'],
  security: SEGURIDAD_ADMIN,
  body: {
    type: 'object',
    required: ['gameId', 'seed'],
    additionalProperties: false,
    properties: {
      gameId: { type: 'string', minLength: 1 },
      seed: { type: 'number' },
      region: { type: 'string', enum: REGIONES },
      forzar: { type: 'boolean' },
    },
  },
  response: {
    201: RESUMEN_PARTIDA_RESPUESTA,
    400: ERROR_RESPUESTA,
    401: ERROR_RESPUESTA,
    403: ERROR_RESPUESTA,
    409: ERROR_RESPUESTA,
  },
} as const;

const ESQUEMA_TICK = {
  description: 'Avanza un tick de la partida (auto-comercio + turno del NPC de gobernanza incluidos).',
  tags: ['admin'],
  security: SEGURIDAD_ADMIN,
  params: PARAMS_GAME_ID,
  response: { 401: ERROR_RESPUESTA, 403: ERROR_RESPUESTA, 404: ERROR_RESPUESTA, 409: ERROR_RESPUESTA },
} as const;

const ESQUEMA_ESTADO_COMPLETO = {
  description:
    'Estado COMPLETO de la partida, sin proyectar por audiencia (todas las Facciones, log global). ' +
    'Sin `mapa` (Fase C11): trae `mapaId` en su lugar — el mapa real se pide una vez por ' +
    'GET .../mapa/:mapaId, cacheable para siempre. ' +
    'Cuerpo de la respuesta no modelado en este esquema por su tamaño y forma variable (Fase C6, doc 4).',
  tags: ['admin'],
  security: SEGURIDAD_ADMIN,
  params: PARAMS_GAME_ID,
  response: { 401: ERROR_RESPUESTA, 403: ERROR_RESPUESTA, 404: ERROR_RESPUESTA },
} as const;

const ESQUEMA_EVENTOS = {
  description:
    'Eventos de dominio con version > `desde` (Fase C13), en orden cronológico — cursor incremental para no ' +
    'volver a mandar el histórico completo tras cada aviso por WebSocket. `desde` ausente u omitido equivale a 0.',
  tags: ['admin'],
  security: SEGURIDAD_ADMIN,
  params: PARAMS_GAME_ID,
  querystring: QUERY_DESDE,
  response: { 400: ERROR_RESPUESTA, 401: ERROR_RESPUESTA, 403: ERROR_RESPUESTA, 404: ERROR_RESPUESTA },
} as const;

const ESQUEMA_EXPORTAR = {
  description:
    'Snapshot completo de la partida para descargar (Fase C12) — mismo formato que se persiste en disco tras ' +
    'cada comando (`PartidaExportada`), no un formato aparte para exportar. Antes corría en el navegador ' +
    '(`GameStore.exportarSimulacion`, retirado); ahora lo sirve el servidor, que es quien tiene el estado real.',
  tags: ['admin'],
  security: SEGURIDAD_ADMIN,
  params: PARAMS_GAME_ID,
  response: { 401: ERROR_RESPUESTA, 403: ERROR_RESPUESTA, 404: ERROR_RESPUESTA },
} as const;

/** Roles que un administrador de partida puede otorgar por esta superficie. `jugador` no está: se obtiene por
 * la superficie de jugador (necesita un `jugadorId`). `administrador_global` tampoco: es de instancia,
 * configurado por variable de entorno (`ADMINISTRADORES`), no repartible por partida. `servicio_npc` es
 * interno. */
const ROLES_OTORGABLES: readonly RolTecnico[] = ['administrador_partida', 'moderador', 'observador'];

const ESQUEMA_METRICAS = {
  description:
    'Metricas de operacion del proceso (Fase E3): duracion de tick, tamano de cola, recuento de comandos por ' +
    'resultado, conexiones y rafagas de catch-up. Numeros crudos, sin interpretar. NO es por partida: ' +
    'describe este proceso, asi que exige administrador global y no membresia de una partida.',
  tags: ['admin'],
  security: SEGURIDAD_ADMIN,
  response: {
    200: {
      type: 'object',
      properties: {
        momento: { type: 'string' },
        proceso: {
          type: 'object',
          properties: {
            arribaSegundos: { type: 'number' },
            memoriaMb: { type: 'object', properties: { rss: { type: 'number' }, heapUsado: { type: 'number' }, heapTotal: { type: 'number' } } },
            partidasAbiertas: { type: 'number' },
          },
        },
        comandos: {
          type: 'object',
          properties: { aceptados: { type: 'number' }, autorizacion: { type: 'number' }, esquema: { type: 'number' }, dominio: { type: 'number' }, persistencia: { type: 'number' } },
        },
        auditoriaFallida: { type: 'number' },
        partidas: {
          type: 'array',
          items: {
            type: 'object',
            properties: {
              gameId: { type: 'string' },
              tick: { type: 'number' },
              version: { type: 'number' },
              colaPendiente: { type: 'number' },
              ticksEjecutados: { type: 'number' },
              tickMsUltimo: { type: 'number' },
              tickMsMedio: { type: 'number' },
              tickMsMaximo: { type: 'number' },
              ultimaRafagaTicks: { type: 'number' },
              mayorRafagaTicks: { type: 'number' },
              relojDeMundoActivo: { type: 'boolean' },
              conexiones: { type: 'number' },
            },
          },
        },
      },
      required: ['momento', 'proceso', 'comandos', 'auditoriaFallida', 'partidas'],
    },
  },
} as const;

const ESQUEMA_AUDITORIA = {
  description:
    'Registro de auditoria de comandos de esta partida (Fase E2): quien pidio que, cuando, y con que ' +
    'resultado — incluidos los RECHAZADOS, que es donde se ve el abuso y de lo que `eventosDominio` no ' +
    'sabe nada. Solo administracion: es un registro de actividad de personas, no estado de juego.',
  tags: ['admin'],
  security: SEGURIDAD_ADMIN,
  params: PARAMS_GAME_ID,
  querystring: {
    type: 'object',
    properties: {
      desde: { type: 'string', description: 'ISO 8601 de reloj de PARED; descarta lo anterior.' },
      actor: { type: 'string', description: 'Solo las lineas de este actor.' },
      // `'true'`/`'false'` como TEXTO, no `type: 'boolean'`: este servidor corre con `coerceTypes: false`
      // (Fase C9, ver `api.ts`), asi que un query param —que siempre llega como texto— nunca se convierte
      // solo. Declararlo booleano hacia que `?soloRechazos=true` fallara la validacion con un 400.
      soloRechazos: { type: 'string', enum: ['true', 'false'], description: 'Solo lo rechazado — la vista de moderacion.' },
    },
    additionalProperties: false,
  },
  response: {
    200: {
      type: 'object',
      properties: {
        entradas: {
          type: 'array',
          items: {
            type: 'object',
            properties: {
              formatoVersion: { type: 'number' },
              momento: { type: 'string' },
              instante: { type: 'number' },
              gameId: { type: 'string' },
              actor: { type: 'string' },
              comando: { type: 'string' },
              resultado: { type: 'string' },
              causa: { type: 'string' },
              detalle: { type: 'string' },
              version: { type: 'number' },
            },
          },
        },
        corruptas: {
          type: 'number',
          description:
            'Lineas ilegibles descartadas al leer (un corte de luz a mitad de escritura). Viaja siempre, y no ' +
            'solo cuando es > 0: quien lee tiene que poder distinguir un registro completo de uno con agujeros.',
        },
      },
      required: ['entradas', 'corruptas'],
    },
  },
} as const;

const ESQUEMA_LISTAR_MEMBRESIAS = {
  description:
    'Membresías técnicas de esta partida (cierre de Fase C): quién tiene rol de administración/observación, ' +
    'vigente o revocado. `vigente` aplica el filtro de `hasta` sobre el reloj del servidor.',
  tags: ['admin'],
  security: SEGURIDAD_ADMIN,
  params: PARAMS_GAME_ID,
  response: {
    200: {
      type: 'object',
      properties: {
        membresias: {
          type: 'array',
          items: {
            type: 'object',
            properties: {
              usuarioId: { type: 'string' },
              jugadorId: { type: ['string', 'null'] },
              rol: { type: 'string' },
              desde: { type: 'string' },
              hasta: { type: 'string' },
              vigente: { type: 'boolean' },
            },
            required: ['usuarioId', 'rol', 'desde', 'vigente'],
          },
        },
      },
      required: ['membresias'],
    },
    401: ERROR_RESPUESTA,
    403: ERROR_RESPUESTA,
    404: ERROR_RESPUESTA,
  },
} as const;

const ESQUEMA_OTORGAR_MEMBRESIA = {
  description:
    'Otorga a un usuario un rol técnico sobre esta partida (cierre de Fase C). El usuario debe haber iniciado ' +
    'sesión alguna vez (404 si no); repetir sobre un usuario que ya tiene membresía es 409, no un cambio de rol.',
  tags: ['admin'],
  security: SEGURIDAD_ADMIN,
  params: PARAMS_GAME_ID,
  body: {
    type: 'object',
    required: ['usuarioId', 'rol'],
    additionalProperties: false,
    properties: {
      usuarioId: { type: 'string', minLength: 1 },
      rol: { type: 'string', enum: ROLES_OTORGABLES },
    },
  },
  response: {
    201: {
      type: 'object',
      properties: { usuarioId: { type: 'string' }, rol: { type: 'string' }, desde: { type: 'string' } },
      required: ['usuarioId', 'rol', 'desde'],
    },
    400: ERROR_RESPUESTA,
    401: ERROR_RESPUESTA,
    403: ERROR_RESPUESTA,
    404: ERROR_RESPUESTA,
    409: ERROR_RESPUESTA,
  },
} as const;

const ESQUEMA_REVOCAR_MEMBRESIA = {
  description:
    'Revoca la membresía técnica de un usuario en esta partida poniéndole `hasta` (no borra el historial, ' +
    'doc 5). 404 si ese usuario no tenía ninguna. No revoca al `administrador_global` de instancia: su acceso ' +
    'no sale de una `Membresia`.',
  tags: ['admin'],
  security: SEGURIDAD_ADMIN,
  params: {
    type: 'object',
    properties: { gameId: { type: 'string' }, usuarioId: { type: 'string' } },
    required: ['gameId', 'usuarioId'],
  },
  response: {
    200: { type: 'object', properties: { revocada: { type: 'boolean' } }, required: ['revocada'] },
    401: ERROR_RESPUESTA,
    403: ERROR_RESPUESTA,
    404: ERROR_RESPUESTA,
  },
} as const;

const ESQUEMA_COMANDOS_ADMIN = {
  ...ESQUEMA_EJECUTAR_COMANDO,
  description:
    'Ejecuta un comando de partida con rol de administración. La matriz de autorización sigue mandando: casi ' +
    'todos los comandos exigen rol jugador y aquí se rechazan (403 rol_insuficiente) — tener acceso técnico ' +
    'no concede autoridad dentro del juego.',
  tags: ['admin'],
  security: SEGURIDAD_ADMIN,
  params: PARAMS_GAME_ID,
  response: { 400: ERROR_RESPUESTA, 401: ERROR_RESPUESTA, 403: ERROR_RESPUESTA, 404: ERROR_RESPUESTA, 409: ERROR_RESPUESTA },
} as const;

export function registrarRutasDeAdmin(app: FastifyInstance, deps: DependenciasDeRutas): void {
  /** Descubrimiento (Fase C12) — ver `ESQUEMA_LISTAR_PARTIDAS`. Sin `gameId` que resolver: `resolverActor`
   * sin tercer argumento solo comprueba sesión + `esAdministradorGlobal`, ninguna `Membresia`. */
  app.get('/admin/partidas', { schema: ESQUEMA_LISTAR_PARTIDAS }, async (request, reply) => {
    const resuelto = resolverActor(request, deps);
    if (!resuelto) return sinSesion(reply);
    if (!puedeCrearPartida(resuelto.actor)) return sinPermiso(reply, 'listar partidas exige rol administrador_global');

    return reply.send({ partidas: await deps.partidas.listar() });
  });

  /**
   * Crea una partida nueva, o RETOMA la que ya hubiera en disco para ese `gameId`. Operación de INSTANCIA:
   * solo `administrador_global`, porque en una partida que aún no existe no hay `Membresia` posible.
   *
   * Al crearla, otorga a quien la crea `Membresia` de `administrador_partida` sobre ella. Sin eso, un
   * administrador global no podría ejecutar ni siquiera los comandos que la matriz le reserva
   * (`alternarFaccionNpc`), porque esa matriz razona sobre roles DE PARTIDA.
   */
  app.post<{ Body: CrearPartidaBody }>('/admin/partidas', { schema: ESQUEMA_CREAR_PARTIDA }, async (request, reply) => {
    const { gameId, seed, region, forzar } = request.body;
    const resuelto = resolverActor(request, deps, gameId);
    if (!resuelto) return sinSesion(reply);
    if (!puedeCrearPartida(resuelto.actor)) {
      return sinPermiso(reply, 'crear o reabrir una partida exige rol administrador_global');
    }
    if (forzar && !puedeDescartarPartida(resuelto.actor)) {
      return sinPermiso(reply, 'descartar una partida exige rol administrador_partida o administrador_global');
    }

    const runner = forzar
      ? await deps.partidas.descartarYCrear(gameId, { seed, region })
      : await deps.partidas.abrir(gameId, { seed, region }).catch((err: unknown) => {
          if (err instanceof PartidaYaAbiertaError) return undefined;
          throw err;
        });
    if (!runner) return reply.code(409).send({ error: `la partida '${gameId}' ya está abierta en este proceso.` });

    otorgarAdministracion(deps, resuelto.actor.usuarioId, gameId);
    return reply.code(201).send(resumenDe(runner));
  });

  app.post<{ Params: ParametrosGameId }>('/admin/partidas/:gameId/tick', { schema: ESQUEMA_TICK }, async (request, reply) => {
    const acceso = exigirAdministracion(request, reply, deps);
    if (!acceso.ok) return acceso.respuesta;

    try {
      const resultado = await acceso.runner.avanzarTick();
      deps.hub.difundir(acceso.runner.gameId, resultado.eventos);
      return reply.send({ ...resumenDe(acceso.runner), resultado });
    } catch (err) {
      // La única forma en que `avanzarTick` puede rechazar (no `resultado.ok === false`, que ya viene dentro
      // de `resultado`) es un fallo de la capa de persistencia.
      return reply.code(409).send({ error: mensajeDe(err) });
    }
  });

  /** Estado COMPLETO de la partida, sin proyección: todas las facciones, log global. Es exactamente por eso
   * que vive tras `/admin/*` — para un jugador sería una fuga (proyecciones por audiencia: Fase C4).
   * Sin `mapa` (Fase C11): viaja `mapaId`, y el mapa real se pide una vez por `GET .../mapa/:mapaId`.
   * `preciosReferencia` y la geometría por frame (`zonas`/`zonasFusionadas`/`trazadoPorAsentamiento`, Fase
   * C10) se fusionan aquí, no en `vistaAdminDeEstado`: son las piezas impuras (caché en `RunnerDePartida`,
   * ver sus comentarios) que esa función pura no puede calcular. */
  app.get<{ Params: ParametrosGameId }>('/admin/partidas/:gameId', { schema: ESQUEMA_ESTADO_COMPLETO }, async (request, reply) => {
    const acceso = exigirAdministracion(request, reply, deps);
    if (!acceso.ok) return acceso.respuesta;
    return reply.send({
      ...vistaAdminDeEstado(acceso.runner.getState()),
      preciosReferencia: acceso.runner.preciosReferencia(),
      ...acceso.runner.geometriaAsentamientos(),
    });
  });

  /** Cursor de eventos (Fase C13) — ver `ESQUEMA_EVENTOS`. `desde` inválido (no numérico: el esquema ya lo
   * filtra por `pattern`; negativo o `NaN` tras parsear no) es un 400, no un 500 silencioso. */
  app.get<{ Params: ParametrosGameId; Querystring: { desde?: string } }>(
    '/admin/partidas/:gameId/eventos',
    { schema: ESQUEMA_EVENTOS },
    async (request, reply) => {
      const acceso = exigirAdministracion(request, reply, deps);
      if (!acceso.ok) return acceso.respuesta;

      const desde = Number(request.query.desde ?? '0');
      if (!Number.isInteger(desde) || desde < 0) return reply.code(400).send({ error: '`desde` debe ser un entero no negativo.' });

      return reply.send({ eventos: eventosDesde(acceso.runner.getState(), desde) });
    }
  );

  /** Descarga del snapshot completo (Fase C12) — ver `ESQUEMA_EXPORTAR`. */
  app.get<{ Params: ParametrosGameId }>('/admin/partidas/:gameId/exportar', { schema: ESQUEMA_EXPORTAR }, async (request, reply) => {
    const acceso = exigirAdministracion(request, reply, deps);
    if (!acceso.ok) return acceso.respuesta;

    reply.header('Content-Disposition', `attachment; filename="${acceso.runner.gameId}.json"`);
    return reply.send(acceso.runner.exportar());
  });

  /** El mapa como asset (Fase C11) — ver `mapa.ts`. Misma comprobación de administración que el resto de esta
   * superficie: el mapa no es secreto, pero la partida sí exige sesión para entrar en su gameId. */
  app.get<{ Params: ParametrosGameId & { mapaId: string } }>('/admin/partidas/:gameId/mapa/:mapaId', { schema: ESQUEMA_MAPA }, async (request, reply) => {
    const acceso = exigirAdministracion(request, reply, deps);
    if (!acceso.ok) return acceso.respuesta;
    enviarMapa(reply, acceso.runner);
  });

  /**
   * Comandos ejecutados como administrador. La matriz sigue mandando: casi todos los comandos son de rol
   * `jugador` y aquí se rechazarán con `rol_insuficiente`, que es lo correcto — tener acceso técnico no
   * concede autoridad dentro del juego (doc 5). Hoy solo `alternarFaccionNpc` admite administración.
   */
  app.post<{ Params: ParametrosGameId; Body: EjecutarComandoBody }>(
    '/admin/partidas/:gameId/comandos',
    { schema: ESQUEMA_COMANDOS_ADMIN, onError: auditarRechazoDeEsquema(deps) },
    async (request, reply) => {
      const acceso = exigirAdministracion(request, reply, deps);
      if (!acceso.ok) return acceso.respuesta;

      const rol = rolEnPartida(acceso.actorInstancia) as RolTecnico;
      const actor: ActorDeComando = { rol, jugadorId: acceso.actorInstancia.membresia?.jugadorId ?? null };
      // Un administrador sin personaje en la partida queda registrado como `admin:<usuarioId>`, para que su
      // huella en el log no se confunda con la de un jugador.
      const actorId = acceso.actorInstancia.membresia?.jugadorId ?? `admin:${acceso.actorInstancia.usuarioId}`;
      return ejecutarComandoHttp(reply, acceso.runner, request.body, actor, actorId, deps.hub, deps.auditoria);
    }
  );

  /**
   * Metricas de operacion (Fase E3).
   *
   * **Administrador GLOBAL, y no por partida**: describe el PROCESO —memoria, uptime, todas las partidas que
   * tiene abiertas— asi que concederlo por membresia de una partida filtraria la actividad de las demas. Es
   * la misma linea que ya separa crear una partida de administrarla.
   *
   * Sin autenticar seria mas comodo para un scraper de metricas, y es justo por eso que no se hace: expone
   * cuantas partidas corren, cuanta gente hay conectada y cuando el servidor va justo.
   */
  app.get('/admin/metricas', { schema: ESQUEMA_METRICAS }, async (request, reply) => {
    const resuelto = resolverActor(request, deps);
    if (!resuelto) return sinSesion(reply);
    if (!deps.administradores.esAdministradorGlobal(resuelto.usuario.id)) {
      return sinPermiso(reply, 'se requiere administrador global');
    }
    return reply.send(recogerMetricas({ partidas: deps.partidas, auditoria: deps.auditoria, hub: deps.hub, ahora: deps.ahora }));
  });

  /**
   * Auditoria de comandos (Fase E2). Exige administracion, igual que el estado completo: son datos de
   * ACTIVIDAD DE PERSONAS (quien intento que y cuando), mas sensibles que el propio estado de juego, y no hay
   * ninguna lectura equivalente en `/jugador/*` a proposito — un jugador no audita a los demas.
   *
   * Lee del archivo, no de memoria: el registro sobrevive al reinicio del proceso, que es la mitad de su
   * razon de ser.
   */
  app.get<{ Params: ParametrosGameId; Querystring: { desde?: string; actor?: string; soloRechazos?: 'true' | 'false' } }>(
    '/admin/partidas/:gameId/auditoria',
    { schema: ESQUEMA_AUDITORIA },
    async (request, reply) => {
      const acceso = exigirAdministracion(request, reply, deps);
      if (!acceso.ok) return acceso.respuesta;

      const { desde, actor, soloRechazos } = request.query;
      // `leer` drena antes: `registrar` escribe sin esperar, asi que sin eso el comando que acaba de
      // ejecutarse podria no estar todavia en el archivo — justo el que se va a consultar.
      return reply.send(await deps.auditoria.leer(request.params.gameId, { desde, actor, soloRechazos: soloRechazos === 'true' }));
    }
  );

  /** Membresías técnicas de la partida (cierre de Fase C) — ver `ESQUEMA_LISTAR_MEMBRESIAS`. */
  app.get<{ Params: ParametrosGameId }>('/admin/partidas/:gameId/membresias', { schema: ESQUEMA_LISTAR_MEMBRESIAS }, async (request, reply) => {
    const acceso = exigirAdministracion(request, reply, deps);
    if (!acceso.ok) return acceso.respuesta;

    const ahora = deps.ahora();
    const membresias = deps.identidad.repositorio.listarMembresiasDePartida(acceso.runner.gameId).map((m) => ({
      usuarioId: m.usuarioId,
      jugadorId: m.jugadorId,
      rol: m.rol,
      desde: m.desde,
      ...(m.hasta !== undefined ? { hasta: m.hasta } : {}),
      vigente: esVigente(m, ahora),
    }));
    return reply.send({ membresias });
  });

  /** Otorgar rol técnico (cierre de Fase C) — ver `ESQUEMA_OTORGAR_MEMBRESIA`. Exige `administrador_partida`
   * o `administrador_global`: un `moderador` administra la partida pero no reparte accesos. */
  app.post<{ Params: ParametrosGameId; Body: { usuarioId: string; rol: RolTecnico } }>(
    '/admin/partidas/:gameId/membresias',
    { schema: ESQUEMA_OTORGAR_MEMBRESIA },
    async (request, reply) => {
      const acceso = exigirAdministracion(request, reply, deps);
      if (!acceso.ok) return acceso.respuesta;
      if (!puedeGestionarMembresias(acceso.actorInstancia)) {
        return sinPermiso(reply, 'otorgar membresías exige rol administrador_partida o administrador_global');
      }

      const { usuarioId, rol } = request.body;
      if (!deps.identidad.repositorio.obtenerUsuario(usuarioId)) {
        return reply.code(404).send({ error: `no existe el usuario '${usuarioId}' (¿ha iniciado sesión alguna vez?)` });
      }
      if (deps.identidad.repositorio.obtenerMembresia(usuarioId, acceso.runner.gameId)) {
        return reply.code(409).send({ error: `el usuario '${usuarioId}' ya tiene una membresía en esta partida; revócala antes de cambiar el rol` });
      }

      const desde = deps.ahora();
      // `jugadorId: null` — ninguno de los roles otorgables por esta vía requiere un `Jugador` (ese lo crea la
      // superficie de jugador al unirse).
      deps.identidad.repositorio.otorgarMembresia({ usuarioId, gameId: acceso.runner.gameId, jugadorId: null, rol, desde });
      return reply.code(201).send({ usuarioId, rol, desde });
    }
  );

  /** Revocar membresía (cierre de Fase C) — ver `ESQUEMA_REVOCAR_MEMBRESIA`. */
  app.delete<{ Params: { gameId: string; usuarioId: string } }>(
    '/admin/partidas/:gameId/membresias/:usuarioId',
    { schema: ESQUEMA_REVOCAR_MEMBRESIA },
    async (request, reply) => {
      const acceso = exigirAdministracion(request, reply, deps);
      if (!acceso.ok) return acceso.respuesta;
      if (!puedeGestionarMembresias(acceso.actorInstancia)) {
        return sinPermiso(reply, 'revocar membresías exige rol administrador_partida o administrador_global');
      }

      const revocada = deps.identidad.repositorio.revocarMembresia(request.params.usuarioId, request.params.gameId, deps.ahora());
      if (!revocada) return reply.code(404).send({ error: `el usuario '${request.params.usuarioId}' no tiene membresía en esta partida` });
      return reply.send({ revocada: true });
    }
  );
}

/** `Membresia` de administración para quien abre la partida. Idempotente: reabrir una partida ya
 * administrada por ese usuario no duplica ni degrada nada. */
function otorgarAdministracion(deps: DependenciasDeRutas, usuarioId: string, gameId: string): void {
  if (deps.identidad.repositorio.obtenerMembresia(usuarioId, gameId)) return;
  deps.identidad.repositorio.otorgarMembresia({
    usuarioId,
    gameId,
    jugadorId: null,
    rol: 'administrador_partida',
    desde: deps.ahora(),
  });
}

type AccesoAdmin =
  | { ok: true; runner: import('../runnerDePartida').RunnerDePartida; actorInstancia: import('../../acceso/rolesDePartida').ActorDeInstancia }
  | { ok: false; respuesta: unknown };

/** Sesión + partida abierta + permiso de administración, en el orden en que importa: sin sesión no se
 * revela si la partida existe. */
function exigirAdministracion(
  request: { headers: Record<string, unknown>; params: unknown },
  reply: Parameters<typeof sinSesion>[0],
  deps: DependenciasDeRutas
): AccesoAdmin {
  const { gameId } = request.params as ParametrosGameId;
  const resuelto = resolverActor(request as never, deps, gameId);
  if (!resuelto) return { ok: false, respuesta: sinSesion(reply) };
  if (!puedeAdministrar(resuelto.actor)) {
    return { ok: false, respuesta: sinPermiso(reply, 'sin rol de administracion en esta partida') };
  }
  const runner = deps.partidas.obtener(gameId);
  if (!runner) return { ok: false, respuesta: partidaNoAbierta(reply, gameId) };
  return { ok: true, runner, actorInstancia: resuelto.actor };
}
