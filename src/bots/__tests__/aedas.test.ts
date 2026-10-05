// Los bots y los Aedas (Doc 6.7): el Rey compra a los itinerantes lo que le revelan y quien dirige la plaza pone a sus
// residentes a cantar épicas, todo con los comandos de un jugador.
import { describe, expect, it } from 'vitest';
import { instante } from '../../domain/tiempo';
import { precioDeVenta } from '../../engine/tecnologia';
import { instanteDeTick } from '../../session/estado';
import { GameSession } from '../../session/gameSession';
import { faccionAsentadaDePrueba } from '../../session/__tests__/faccionAsentadaDePrueba';
import { puertoEnProceso } from '../puerto';
import { RunnerDeBots } from '../runner';
import { cerebroDeBot } from '../cerebro';

const DOS_DIAS = 2 * 24 * 60 * 60_000;

/** Una Facción de bots con un Aeda itinerante parado en su plaza, un Aeda residente, el cobre y la cantería con el logro cumplido, y oro de sobra. */
async function mundoConAedas(ticks: number, oro = true) {
  const base = GameSession.crear('aedas', { seed: 7 });
  base.ejecutar(faccionAsentadaDePrueba, { nombre: 'Alfa' });
  const payload = base.exportar();
  const plaza = payload.state.asentamientos[0]!;
  const faccionId = plaza.faccionId;
  const hace = instante(instanteDeTick(0) - DOS_DIAS);
  // Con riqueza de sobra: el doble de la tarifa de la cantería y el cobre (oro y madera) más el precio del Aeda.
  const almacen = oro
    ? Object.fromEntries(Object.entries(plaza.almacen).map(([recurso, item]) => [recurso, recurso === 'oro' || recurso === 'madera' ? { ...item, cantidad: 2000, capacidad: 2000 } : item]))
    : plaza.almacen;
  const asentamientos = [
    {
      ...plaza,
      almacen,
      nivel: 2,
      nivelActual: 2,
      poblacion: { ...plaza.poblacion, nobleza: 5 },
      edificios: [...plaza.edificios, { ...plaza.edificios[0]!, id: 'palacio-test', tipo: 'palacio' as const, estado: 'activo' as const, nivelInterno: 1 }],
    },
  ];
  const tecnologia = {
    ...payload.state.tecnologia,
    logros: { metalurgia_cobre: hace, canteria: hace },
    primeros: { metalurgia_cobre: { faccionId: 'otra-faccion', en: hace } },
  };
  const aedas = [{ id: 'aeda-1', posicion: plaza.posicion, enAsentamientoId: plaza.id, hasta: instante(instanteDeTick(0) + DOS_DIAS), progreso: 0, recientes: [plaza.id] }];
  const aedasResidentes = {
    aedas: [{ id: 'aeda-r1', nombre: 'Femio', asentamientoId: plaza.id, faccionId, llegadaEn: hace }],
    siguiente: 2,
    esperaDesde: {},
    cumplidas: {},
  };
  const sesion = GameSession.importar({ ...payload, state: { ...payload.state, asentamientos, tecnologia, aedas, aedasResidentes } });
  const bots = new RunnerDeBots(puertoEnProceso(sesion), cerebroDeBot, { semilla: 7, horario: 'siempre' });
  for (const h of sesion.getState().heroes) bots.alta(h.id);
  for (let tick = 1; tick <= ticks; tick++) await bots.trasTick(tick, sesion.avanzarTick().eventos);
  return { sesion, faccionId, plazaId: plaza.id };
}

describe('bots: Aedas', () => {
  it('el Rey compra la tecnología revelada y el residente canta la épica de otra, no de la que se acaba de comprar', async () => {
    const { sesion, faccionId } = await mundoConAedas(30);
    const estado = sesion.getState();
    expect(estado.tecnologia.porFaccion[faccionId]!.aparecidas).toContain('metalurgia_cobre');
    expect(estado.eventosDominio.some((e) => e.codigo === 'aedas.venta')).toBe(true);
    expect(estado.aedasResidentes!.aedas[0]!.epica?.tecnologiaId).toBe('canteria');
  });

  it('sin oro de sobra no compra: la compra se salta el hito, no la adopción', async () => {
    const { sesion, faccionId } = await mundoConAedas(30, false);

    expect(sesion.getState().eventosDominio.some((e) => e.codigo === 'aedas.venta')).toBe(false);
    expect(sesion.getState().tecnologia.porFaccion[faccionId]?.aparecidas ?? []).not.toContain('metalurgia_cobre');
    expect(precioDeVenta('metalurgia_cobre')).toBeGreaterThan(0);
  });
});
