// La memoria de la niebla de guerra vista desde la PARTIDA, no desde el módulo: que el tick real la rellene,
// que no la pierda por el camino y que sobreviva a un `exportar`/`importar`. Los tests de la regla en sí
// están en `engine/__tests__/memoria.test.ts`; lo que aquí se congela es el cableado.
import { describe, expect, it } from 'vitest';
import { GameSession } from '../gameSession';
import { ACTOR, OPC, partidaConAsentamiento } from './fixtures';
import { celdasExploradas, estaExplorado, rejillaDe } from '../../engine/exploracion';
import { crearFaccion } from '../comandos/crearFaccion';

describe('memoriaPorFaccion en la partida', () => {
  it('una partida recién creada no recuerda nada', () => {
    expect(GameSession.crear('g1', { seed: 42 }).getState().memoriaPorFaccion).toEqual({});
  });

  it('el tick graba lo que la plaza vigila, y el mapa de la partida es el que trocea la rejilla', () => {
    const { sesion, faccionId } = partidaConAsentamiento();
    sesion.avanzarTick();

    const estado = sesion.getState();
    const rejilla = rejillaDe(sesion.getMapa().limites);
    const memoria = estado.memoriaPorFaccion[faccionId]!;
    const plaza = estado.asentamientos[0]!;

    expect(estaExplorado(memoria.exploracion, rejilla, plaza.posicion)).toBe(true);
    expect(estaExplorado(memoria.exploracion, rejilla, { x: 1800, y: 1800 })).toBe(false);
    expect(memoria.asentamientos).toEqual({});
  });

  it('lo explorado solo crece: veinte ticks despues sigue estando todo lo de antes', () => {
    const { sesion, faccionId } = partidaConAsentamiento();
    sesion.avanzarTick();
    const tras1 = celdasExploradas(sesion.getState().memoriaPorFaccion[faccionId]!.exploracion);

    for (let i = 0; i < 20; i++) sesion.avanzarTick();
    const tras21 = celdasExploradas(sesion.getState().memoriaPorFaccion[faccionId]!.exploracion);

    // Crece o se queda igual (la zona se ensancha al completarse edificios), pero nunca mengua.
    expect(tras21).toBeGreaterThanOrEqual(tras1);
    expect(tras1).toBeGreaterThan(0);
  });

  it('sobrevive a exportar/importar: es estado de partida, no algo que se recalcule al cargar', () => {
    const { sesion, faccionId } = partidaConAsentamiento();
    sesion.avanzarTick();
    const antes = sesion.getState().memoriaPorFaccion[faccionId]!;

    const recargada = GameSession.importar(JSON.parse(JSON.stringify(sesion.exportar())));
    expect(recargada.getState().memoriaPorFaccion[faccionId]).toEqual(antes);
  });
});

// La otra mitad de la niebla, la del jugador SIN bandera (`Jugador.exploracionPersonal`, Doc 1.3): el tick
// que la graba se prueba puro en `engine/__tests__/ubicacion.test.ts`; esto congela que fundar la funde en
// la Facción y la borra del jugador.
describe('exploracionPersonal se funde al conseguir bandera', () => {
  it('crear una Facción funde lo explorado sin bandera, y borra el registro personal', () => {
    const payload = GameSession.crear('g2', { seed: 42 }).exportar();
    // Simula a alguien que ya anduvo sin bandera antes de fundar: se inyecta el registro a mano para probar
    // la fusión aislada, sin depender de un tick real que vaya grabando `exploracionPersonal` poco a poco.
    const sesion = GameSession.importar({
      ...payload,
      state: {
        ...payload.state,
        jugadores: [
          { id: ACTOR, liderazgoBase: 0, ubicacion: { tipo: 'desconectado' as const, punto: { x: 400, y: 400 } }, exploracionPersonal: 'ff' },
        ],
      },
    });

    const r = sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, OPC);
    const estado = sesion.getState();

    expect(estado.memoriaPorFaccion[r.datos!.faccionId]!.exploracion).toBe('ff');
    expect(estado.jugadores.find((j) => j.id === ACTOR)!.exploracionPersonal).toBeUndefined();
  });
});
