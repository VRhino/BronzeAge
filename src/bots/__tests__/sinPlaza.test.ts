// El arranque de los bots-héroe en los campamentos (D53-D59): llegan a un mundo sin Facciones, eligen campamento, se buscan
// Facción, cazan en el anillo, juntan el fondo, compran la caravana y fundan. Como un jugador, por el puerto.
import { describe, expect, it } from 'vitest';
import { GameSession } from '../../session/gameSession';
import type { EventoDominio } from '../../domain/eventos';
import type { CampamentoMercenarios } from '../../domain/types';
import { puertoEnProceso } from '../puerto';
import { RunnerDeBots, type Perfil } from '../runner';
import { cerebroDeBot } from '../cerebro';
import { costoRefundacion } from '../../engine/refundacion';

/** Llegan `grupos` (cada uno con sus perfiles) en el tick 1, al campamento con menos residentes, y juegan `ticks`. */
function mundo(ticks: number, grupos: Perfil['tipo'][][], campamento = (c: CampamentoMercenarios) => c) {
  const inicial = GameSession.crear('sin-plaza', { seed: 7 }).exportar();
  const sesion = GameSession.importar({ ...inicial, state: { ...inicial.state, campamentosMercenarios: inicial.state.campamentosMercenarios.map(campamento) } });
  const puerto = puertoEnProceso(sesion);
  const bots = new RunnerDeBots(puerto, cerebroDeBot, { semilla: 7, horario: 'siempre' });
  let n = 0;
  for (const grupo of grupos) {
    const campamento = [...puerto.campamentos()].sort((a, b) => a.residentes - b.residentes || (a.id < b.id ? -1 : 1))[0]!;
    let lider: string | undefined;
    for (const tipo of grupo) {
      n++;
      const id = puerto.crearHeroe(`cuenta-${n}`, {
        displayName: `Bot ${n}`,
        campamentoId: campamento.id,
        classDefinitionId: 'Spear',
        genero: 'masculino',
        avatar: { cabezaId: '', peloId: '', barbaId: '', cejasId: '' },
      })!;
      lider ??= id;
      bots.alta(id, tipo === 'amigos' ? { tipo, lider } : { tipo });
    }
  }
  const eventos: EventoDominio[] = [];
  for (let tick = 1; tick <= ticks; tick++) {
    const r = sesion.avanzarTick();
    bots.trasTick(tick, r.eventos);
    eventos.push(...sesion.getState().eventosDominio.filter((e) => e.version >= r.version).reverse());
  }
  return { sesion, eventos };
}

const codigos = (eventos: readonly EventoDominio[]) => new Set(eventos.map((e) => e.codigo));

describe('bots sin plaza', () => {
  it('los amigos montan una Facción: el líder la crea, los otros piden entrar y el Rey los acepta', () => {
    const { sesion } = mundo(30, [['amigos', 'amigos', 'amigos']]);
    const faccion = sesion.getState().facciones[0]!;

    expect(sesion.getState().facciones).toHaveLength(1);
    expect(faccion.ciudadanosIds).toHaveLength(3);
  });

  it('salen a buscar sin tropa y abren los alijos que ven (D60-D63)', () => {
    const { eventos } = mundo(6 * 60, [['amigos', 'amigos', 'amigos']]);

    expect(codigos(eventos).has('mercenarios.prestamo')).toBe(true);
    expect(eventos.filter((e) => e.codigo === 'alijo.abierto').length).toBeGreaterThan(0);
  });

  it('con el fondo completo, uno compra la caravana, explora el anillo, salen juntos con escolta y fundan: los tres son cofundadores (M2)', () => {
    // El fondo ya puesto a nombre del primer héroe que llegará (`heroe-0`, el líder, que creará la Facción).
    const { sesion, eventos } = mundo(8 * 60, [['amigos', 'amigos', 'amigos']], (c) => (c.id === 'mercenarios-0' ? { ...c, fondos: { 'heroe-0': costoRefundacion() } } : c));

    const plaza = sesion.getState().asentamientos[0];
    expect(codigos(eventos).has('mercenarios.caravana_refundacion')).toBe(true);
    expect(plaza, 'funda').toBeDefined();
    expect(plaza!.heroesFundadoresIds).toHaveLength(3);
  });
});
