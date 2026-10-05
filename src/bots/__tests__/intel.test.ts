// Los bots y la intel de las tabernas (Doc 5.12.10): el Rey o el Gobernador con taberna y oro de sobra compra el informe de la plaza
// ajena que conoce, con los comandos de un jugador, y no repite antes del cooldown.
import { describe, expect, it } from 'vitest';
import { GameSession } from '../../session/gameSession';
import { faccionAsentadaDePrueba } from '../../session/__tests__/faccionAsentadaDePrueba';
import { puertoEnProceso } from '../puerto';
import { RunnerDeBots } from '../runner';
import { cerebroDeBot } from '../cerebro';
import { instanteDeTick } from '../../session/estado';

/** Una Facción de bots con taberna y oro, y una plaza rival lejos de la que tiene ficha en su memoria. */
async function mundoConRival(ticks: number, oro: number) {
  const base = GameSession.crear('intel', { seed: 7 });
  base.ejecutar(faccionAsentadaDePrueba, { nombre: 'Alfa' });
  const payload = base.exportar();
  const plaza = payload.state.asentamientos[0]!;
  const faccion = payload.state.facciones[0]!;
  const almacen = Object.fromEntries(Object.entries(plaza.almacen).map(([recurso, item]) => [recurso, recurso === 'oro' ? { ...item, cantidad: oro, capacidad: Math.max(oro, item.capacidad) } : item]));
  const rival = { ...plaza, id: 'plaza-rival', nombre: 'Troya', faccionId: 'f-rival', posicion: { x: plaza.posicion.x + 500, y: plaza.posicion.y } };
  const taberna = { id: 'taberna-test', tipo: 'taberna' as const, posicion: { x: 6, y: 6 }, estado: 'activo' as const, nivelInterno: 1 };
  const ficha = { asentamientoId: rival.id, nombre: 'Troya', faccionId: 'f-rival', posicion: rival.posicion, nivel: rival.nivel, conocidoEn: instanteDeTick(0) };
  const memoria = payload.state.memoriaPorFaccion[faccion.id] ?? { exploracion: '', asentamientos: {} };
  const sesion = GameSession.importar({
    ...payload,
    state: {
      ...payload.state,
      facciones: [...payload.state.facciones, { ...faccion, id: 'f-rival', nombre: 'Troya', reyId: null, ciudadanosIds: [] }],
      asentamientos: [{ ...plaza, almacen, edificios: [...plaza.edificios, taberna] }, rival],
      memoriaPorFaccion: { ...payload.state.memoriaPorFaccion, [faccion.id]: { ...memoria, asentamientos: { [rival.id]: ficha } } },
    },
  });
  const bots = new RunnerDeBots(puertoEnProceso(sesion), cerebroDeBot, { semilla: 7, horario: 'siempre' });
  for (const h of sesion.getState().heroes) bots.alta(h.id);
  for (let tick = 1; tick <= ticks; tick++) await bots.trasTick(tick, sesion.avanzarTick().eventos);
  return { sesion, faccionId: faccion.id };
}

describe('bots: intel', () => {
  it('el Rey compra el informe de la plaza rival que conoce, y es un solo informe aunque pasen los turnos', async () => {
    const { sesion, faccionId } = await mundoConRival(40, 2000);
    const estado = sesion.getState();
    expect(estado.memoriaPorFaccion[faccionId]!.informes?.['plaza-rival']).toMatchObject({ faccionId: 'f-rival', nombre: 'Troya' });
    expect(estado.eventosDominio.filter((e) => e.codigo === 'asentamiento.informe_pedido')).toHaveLength(1);
  });

  it('sin oro de sobra no compra', async () => {
    const { sesion, faccionId } = await mundoConRival(40, 20);
    expect(sesion.getState().memoriaPorFaccion[faccionId]?.informes).toBeUndefined();
  });
});
