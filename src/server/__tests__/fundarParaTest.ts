// FIXTURE DE TESTS. Una plaza para el héroe de un jugador, sin el flujo de juego: fundar a pie no existe (D19) y fundar de verdad pide la
// caravana de un campamento con su fondo, que los tests de servidor no miden. Se para el servidor, se funda sobre la partida guardada con
// la fixture de sesión y se vuelve a abrir —con la misma identidad, para que las sesiones y membresías sigan valiendo— y a cargar la
// partida, que un proceso nuevo no tiene abierta.
import type { FastifyInstance } from 'fastify';
import { crearAlmacenEnDisco } from '../almacen/enDisco';
import { cargarPartida, guardarPartida } from '../persistenciaPartida';
import { enPie } from '../../session/__tests__/fixtures';
import { fundarAsentamiento } from '../../session/__tests__/fundarDePrueba';

export async function fundarParaTest(opciones: {
  app: FastifyInstance;
  /** Crea el servidor de nuevo, con la misma identidad y el mismo directorio. */
  reabrir: () => Promise<FastifyInstance>;
  directorio: string;
  gameId: string;
  auth: { authorization: string };
  faccionId: string;
  punto: { x: number; y: number };
}): Promise<{ app: FastifyInstance; asentamientoId: string }> {
  const { gameId } = opciones;
  const heroeId = (await opciones.app.inject({ method: 'GET', url: `/v1/jugador/partidas/${gameId}`, headers: opciones.auth })).json().heroeId as string;
  await opciones.app.close();

  const almacen = crearAlmacenEnDisco(opciones.directorio);
  const { sesion } = (await cargarPartida(almacen, gameId))!;
  const enSitio = enPie(sesion, heroeId, opciones.punto);
  const fundada = enSitio.ejecutar(fundarAsentamiento, { faccionId: opciones.faccionId }, { actor: heroeId });
  if (!fundada.ok) throw new Error(`fundarParaTest: ${fundada.codigoError}`);
  await guardarPartida(almacen, enSitio, new Date().toISOString(), { forzar: true });

  const app = await opciones.reabrir();
  const admin = await app.inject({ method: 'POST', url: '/v1/sesiones', headers: { authorization: 'dev jefa' } });
  await app.inject({ method: 'POST', url: '/v1/admin/partidas', headers: { authorization: `sesion ${admin.json().sesionId}` }, payload: { gameId, seed: 42 } });
  return { app, asentamientoId: fundada.datos!.asentamientoId };
}
