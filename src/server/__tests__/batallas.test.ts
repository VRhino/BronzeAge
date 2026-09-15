// La superficie del servidor de batalla (doc 02 §3.2-§3.4) con `app.inject()`: la credencial de servidor, recoger el
// ticket, asignar y empezar, y que cada jugador recoja SU token y solo el suyo.
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { FastifyInstance } from 'fastify';
import { crearServidor } from '../api';
import { crearAlmacenEnDisco } from '../almacen/enDisco';
import { guardarPartida } from '../persistenciaPartida';
import type { ServidorDeBatalla } from '../identidad/servidoresDeBatalla';
import { GameSession } from '../../session/gameSession';
import { REGISTRO_COMANDOS } from '../../session/comandos/registro';
import { frenteACampamento } from '../../session/__tests__/fixtures';
import { SCHEMA_VERSION, type BattleTicket } from '../../contratos/v1/dto';

const ADMINS = [{ proveedor: 'dev', sujetoId: 'jefa' }];
const S1: ServidorDeBatalla = { id: 's1', token: 'secreto-1' };
const COMO_S1 = { authorization: 'batalla-servidor secreto-1' };

let directorio: string;
let app: FastifyInstance | undefined;

beforeEach(async () => {
  directorio = await mkdtemp(join(tmpdir(), 'bronzeage-batallas-'));
});

afterEach(async () => {
  await app?.close();
  app = undefined;
  await rm(directorio, { recursive: true, force: true });
});

async function login(servidor: FastifyInstance, sujetoId: string) {
  const res = await servidor.inject({ method: 'POST', url: '/v1/sesiones', headers: { authorization: `dev ${sujetoId}` } });
  return { headers: { authorization: `sesion ${res.json().sesionId}` }, usuarioId: res.json().usuarioId as string };
}

/**
 * Una partida guardada con una batalla abierta: el fundador ataca un campamento, y ese héroe es el de `ana`. Se guarda
 * en disco y el admin la reabre, que es como un proceso recoge una partida que ya existía.
 */
async function conBatallaAbierta(servidores: ServidorDeBatalla[] = [S1]) {
  const servidor = crearServidor({ directorio, administradoresGlobales: ADMINS, servidoresBatalla: servidores });
  app = servidor;
  const ana = await login(servidor, 'ana');
  const { sesion, fundador } = frenteACampamento();
  const battleId = sesion.ejecutar(REGISTRO_COMANDOS.atacar, { heroeId: fundador, objetivo: { tipo: 'campamento', id: 'camp-1' } }, { actor: fundador }).datos!
    .battleId;
  const payload = sesion.exportar();
  const heroes = payload.state.heroes.map((h) => (h.id === fundador ? { ...h, jugadorId: ana.usuarioId } : h));
  await guardarPartida(crearAlmacenEnDisco(directorio), GameSession.importar({ ...payload, state: { ...payload.state, heroes } }), new Date().toISOString());

  const admin = await login(servidor, 'jefa');
  await servidor.inject({ method: 'POST', url: '/v1/admin/partidas', headers: admin.headers, payload: { gameId: 'test', seed: 42 } });
  await servidor.inject({ method: 'POST', url: '/v1/jugador/partidas/test/membresia', headers: ana.headers });
  return { servidor, battleId, fundador, ana: ana.headers, admin: admin.headers };
}

function asignacion(battleId: string, heroeId: string, intentoAsignacionId = 'intento-1') {
  return {
    schemaVersion: SCHEMA_VERSION,
    battleId,
    ticketRevision: 0,
    intentoAsignacionId,
    instancia: { host: 'batalla-1.example', puerto: 7777, protocolo: 'udp' },
    tokensParticipante: [{ heroeId, token: 'token-de-ana', expiraEn: '2026-09-15T12:00:00Z' }],
  };
}

describe('la credencial de servidor de batalla (doc 02 §3.3)', () => {
  it('sin credencial, con una sesión de jugador o con un token no declarado: 401', async () => {
    const { servidor, ana } = await conBatallaAbierta();
    for (const headers of [{}, ana, { authorization: 'batalla-servidor otro' }]) {
      expect((await servidor.inject({ method: 'GET', url: '/v1/batallas/pendientes', headers })).statusCode).toBe(401);
    }
  });

  it('sin SERVIDORES_BATALLA no entra ningún servidor', async () => {
    app = crearServidor({ directorio, administradoresGlobales: ADMINS });
    expect((await app.inject({ method: 'GET', url: '/v1/batallas/pendientes', headers: COMO_S1 })).statusCode).toBe(401);
  });
});

describe('recoger, asignar y empezar (doc 02 §3.2-§3.3)', () => {
  it('la batalla abierta aparece pendiente, con su ticket y sin incorporaciones', async () => {
    const { servidor, battleId } = await conBatallaAbierta();

    const pendientes = await servidor.inject({ method: 'GET', url: '/v1/batallas/pendientes', headers: COMO_S1 });
    const ticket = await servidor.inject({ method: 'GET', url: `/v1/batallas/${battleId}/ticket`, headers: COMO_S1 });
    const incorporaciones = await servidor.inject({ method: 'GET', url: `/v1/batallas/${battleId}/incorporaciones`, headers: COMO_S1 });

    expect(pendientes.json().batallas).toEqual([{ battleId, gameId: 'test', ticketRevision: 0 }]);
    expect(ticket.json()).toMatchObject({ battleId, gameId: 'test', contextoEstrategico: { tipo: 'campamento_bandidos' } });
    expect(incorporaciones.json()).toEqual({ incorporaciones: [] });
  });

  it('gana la primera asignación: repetirla no cambia nada y otra distinta es 409', async () => {
    const { servidor, battleId, fundador } = await conBatallaAbierta();
    const asignar = (intento: string) =>
      servidor.inject({ method: 'POST', url: `/v1/batallas/${battleId}/asignacion`, headers: COMO_S1, payload: asignacion(battleId, fundador, intento) });

    const primera = await asignar('intento-1');
    expect(primera.statusCode).toBe(200);
    expect(primera.json().estado).toBe('asignada');
    expect((await asignar('intento-1')).statusCode, 'reintentar lo mismo es idempotente').toBe(200);
    const otra = await asignar('intento-2');
    expect(otra.statusCode).toBe(409);
    expect(otra.json().error).toBe('batalla.ya_asignada');
    expect((await servidor.inject({ method: 'GET', url: '/v1/batallas/pendientes', headers: COMO_S1 })).json().batallas).toEqual([]);
  });

  it('una versión de contrato que no toca es 409 con la esperada; una forma inválida, 400', async () => {
    const { servidor, battleId, fundador } = await conBatallaAbierta();
    const enviar = (payload: object) => servidor.inject({ method: 'POST', url: `/v1/batallas/${battleId}/asignacion`, headers: COMO_S1, payload });

    const otraVersion = await enviar({ ...asignacion(battleId, fundador), schemaVersion: SCHEMA_VERSION + 1 });
    expect(otraVersion.statusCode).toBe(409);
    expect(otraVersion.json().schemaVersionEsperada).toBe(SCHEMA_VERSION);
    const { instancia: _, ...sinInstancia } = asignacion(battleId, fundador);
    expect((await enviar(sinInstancia)).statusCode).toBe(400);
  });

  it('el inicio solo lo confirma el servidor que tiene la asignación', async () => {
    const { servidor, battleId, fundador } = await conBatallaAbierta([S1, { id: 's2', token: 'secreto-2' }]);
    await servidor.inject({ method: 'POST', url: `/v1/batallas/${battleId}/asignacion`, headers: COMO_S1, payload: asignacion(battleId, fundador) });
    const inicio = { schemaVersion: SCHEMA_VERSION, battleId, ticketRevision: 0, intentoAsignacionId: 'intento-1' };

    const ajeno = await servidor.inject({ method: 'POST', url: `/v1/batallas/${battleId}/inicio`, headers: { authorization: 'batalla-servidor secreto-2' }, payload: inicio });
    const propio = await servidor.inject({ method: 'POST', url: `/v1/batallas/${battleId}/inicio`, headers: COMO_S1, payload: inicio });

    expect(ajeno.statusCode).toBe(409);
    expect(propio.statusCode).toBe(200);
    expect(propio.json().estado).toBe('en_curso');
  });
});

describe('el resultado (doc 02 §3.3)', () => {
  it('se aplica una vez: repetir el mismo es 200 sin cambios, y otro distinto es 409', async () => {
    const { servidor, battleId, fundador } = await conBatallaAbierta();
    const base = { schemaVersion: SCHEMA_VERSION, battleId, ticketRevision: 0, intentoAsignacionId: 'intento-1' };
    await servidor.inject({ method: 'POST', url: `/v1/batallas/${battleId}/asignacion`, headers: COMO_S1, payload: asignacion(battleId, fundador) });
    await servidor.inject({ method: 'POST', url: `/v1/batallas/${battleId}/inicio`, headers: COMO_S1, payload: base });
    const ticket = (await servidor.inject({ method: 'GET', url: `/v1/batallas/${battleId}/ticket`, headers: COMO_S1 })).json() as BattleTicket;
    const escuadras = [...ticket.bandos.atacante.participantes.flatMap((p) => p.escuadras), ...ticket.bandos.defensor.escuadrasSinHeroe];
    const resultado = {
      ...base,
      resultId: 'resultado-1',
      inicio: '2026-09-15T10:00:00Z',
      fin: '2026-09-15T10:05:00Z',
      ganador: 'atacante',
      razon: 'aniquilacion',
      objetivos: [],
      porEscuadra: escuadras.map((s) => ({ squadId: s.squadId, desplegados: s.efectivosAutorizados, supervivientesAlCierre: s.efectivosAutorizados, muertos: 0, xpGanada: 0 })),
      porHeroe: [{ heroeId: fundador, participo: true, sobrevivioAlCierre: true, xpGanada: 0 }],
      versionServidor: 'conquest-test',
    };
    const enviar = (payload: object) => servidor.inject({ method: 'POST', url: `/v1/batallas/${battleId}/resultado`, headers: COMO_S1, payload });

    const primero = await enviar(resultado);
    expect(primero.statusCode).toBe(200);
    expect(primero.json()).toEqual({ battleId, estado: 'aplicada' });
    expect((await enviar(resultado)).statusCode, 'reintentar lo mismo es idempotente').toBe(200);
    expect((await enviar({ ...resultado, ganador: 'defensor' })).statusCode).toBe(409);
  });
});

describe('el token de cada jugador (doc 02 §3.4)', () => {
  it('lo recoge el jugador que combate, y no antes de que haya asignación', async () => {
    const { servidor, battleId, fundador, ana } = await conBatallaAbierta();
    const url = `/v1/jugador/partidas/test/batallas/${battleId}/asignacion`;

    expect((await servidor.inject({ method: 'GET', url, headers: ana })).statusCode).toBe(404);
    await servidor.inject({ method: 'POST', url: `/v1/batallas/${battleId}/asignacion`, headers: COMO_S1, payload: asignacion(battleId, fundador) });
    const res = await servidor.inject({ method: 'GET', url, headers: ana });

    expect(res.statusCode).toBe(200);
    expect(res.json()).toEqual({
      battleId,
      instancia: { host: 'batalla-1.example', puerto: 7777, protocolo: 'udp' },
      token: 'token-de-ana',
      expiraEn: '2026-09-15T12:00:00Z',
    });
  });

  it('quien no combate no lo recoge, y el estado del admin no lo lleva', async () => {
    const { servidor, battleId, fundador, admin } = await conBatallaAbierta();
    await servidor.inject({ method: 'POST', url: `/v1/batallas/${battleId}/asignacion`, headers: COMO_S1, payload: asignacion(battleId, fundador) });
    const bruno = await login(servidor, 'bruno');
    await servidor.inject({ method: 'POST', url: '/v1/jugador/partidas/test/membresia', headers: bruno.headers });

    const ajeno = await servidor.inject({ method: 'GET', url: `/v1/jugador/partidas/test/batallas/${battleId}/asignacion`, headers: bruno.headers });
    const estadoAdmin = await servidor.inject({ method: 'GET', url: '/v1/admin/partidas/test', headers: admin });

    expect(ajeno.statusCode).toBe(403);
    expect(estadoAdmin.statusCode).toBe(200);
    expect(estadoAdmin.body).not.toContain('token-de-ana');
  });
});
