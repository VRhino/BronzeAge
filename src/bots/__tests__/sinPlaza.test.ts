// El arranque de los bots-héroe en los campamentos (D53-D59): llegan a un mundo sin Facciones, eligen campamento, se buscan
// Facción, cazan en el anillo, juntan el fondo, compran la caravana y fundan. Como un jugador, por el puerto.
import { describe, expect, it } from 'vitest';
import { GameSession } from '../../session/gameSession';
import type { EventoDominio } from '../../domain/eventos';
import type { CampamentoMercenarios, Faccion } from '../../domain/types';
import { crearFaccion as crearFaccionEngine } from '../../engine/faccion';
import { puertoEnProceso } from '../puerto';
import { RunnerDeBots, type Perfil } from '../runner';
import { darDeAlta } from '../llegadas';
import { cerebroDeBot } from '../cerebro';
import { costoRefundacion } from '../../engine/refundacion';
import { distancia } from '../../world/geometria';

/** Llegan `grupos` (cada uno con sus perfiles) en el tick 1, al campamento con menos residentes, y juegan `ticks`. */
async function mundo(ticks: number, grupos: Perfil['tipo'][][], campamento = (c: CampamentoMercenarios) => c, previas: Faccion[] = []) {
  const inicial = GameSession.crear('sin-plaza', { seed: 7 }).exportar();
  const sesion = GameSession.importar({ ...inicial, state: { ...inicial.state, facciones: [...inicial.state.facciones, ...previas], campamentosMercenarios: inicial.state.campamentosMercenarios.map(campamento) } });
  const puerto = puertoEnProceso(sesion);
  const bots = new RunnerDeBots(puerto, cerebroDeBot, { semilla: 7, horario: 'siempre' });
  let n = 0;
  for (const grupo of grupos) {
    const perfil = grupo[0]!;
    await darDeAlta(puerto, bots, { tick: 0, grupo: 0, perfil, cuantos: grupo.length }, () => `Bot ${++n}`);
  }
  const eventos: EventoDominio[] = [];
  for (let tick = 1; tick <= ticks; tick++) {
    const r = sesion.avanzarTick();
    await bots.trasTick(tick, r.eventos);
    eventos.push(...sesion.getState().eventosDominio.filter((e) => e.version >= r.version).reverse());
  }
  return { sesion, eventos };
}

const codigos = (eventos: readonly EventoDominio[]) => new Set(eventos.map((e) => e.codigo));

describe('bots sin plaza', () => {
  it('los amigos montan una Facción: el líder la crea, los otros piden entrar y el Rey los acepta', async () => {
    const { sesion } = await mundo(30, [['amigos', 'amigos', 'amigos']]);
    const faccion = sesion.getState().facciones[0]!;

    expect(sesion.getState().facciones).toHaveLength(1);
    expect(faccion.ciudadanosIds).toHaveLength(3);
  });

  it('si ya hay una Facción con ese nombre (restos de una corrida anterior), crea la suya con el id al final en vez de quedarse sin ella', async () => {
    const { sesion } = await mundo(30, [['solitario']], (c) => c, [crearFaccionEngine('faccion-vieja', 'Casa de Bot 1')]);

    expect(sesion.getState().facciones.map((f) => f.nombre).sort()).toEqual(['Casa de Bot 1', 'Casa de Bot 1 (heroe-0)']);
  });

  it('salen a buscar sin tropa y abren los alijos que ven (D60-D63)', async () => {
    const { eventos } = await mundo(6 * 60, [['amigos', 'amigos', 'amigos']]);

    expect(codigos(eventos).has('mercenarios.prestamo')).toBe(true);
    expect(eventos.filter((e) => e.codigo === 'alijo.abierto').length).toBeGreaterThan(0);
  });

  it('con el fondo completo, uno compra la caravana, explora el anillo, salen juntos con escolta y fundan: los tres son cofundadores (M2)', async () => {
    // El fondo ya puesto a nombre del primer héroe que llegará (`heroe-0`, el líder, que creará la Facción).
    const { sesion, eventos } = await mundo(8 * 60, [['amigos', 'amigos', 'amigos']], (c) => (c.id === 'mercenarios-0' ? { ...c, fondos: { 'heroe-0': costoRefundacion() } } : c));

    const plaza = sesion.getState().asentamientos[0];
    expect(codigos(eventos).has('mercenarios.caravana_refundacion')).toBe(true);
    expect(plaza, 'funda').toBeDefined();
    expect(plaza!.heroesFundadoresIds).toHaveLength(3);
  });

  it('ningún ejército se queda parado en el mapa: el que acaba o se queda sin tropa se repliega, y los grupos de cinco vuelven a casa', async () => {
    const { sesion } = await mundo(3 * 1440, [Array(5).fill('amigos'), Array(5).fill('amigos'), Array(5).fill('amigos'), Array(5).fill('amigos'), ['solitario'], ['solitario']]);
    const estado = sesion.getState();
    const lugares = [...estado.asentamientos.map((a) => a.posicion), ...estado.campamentosMercenarios.map((c) => c.posicion)];
    const parados = estado.ejercitos.filter((e) => e.estado === 'estacionado' && !lugares.some((l) => distancia(l, e.posicionActual) <= 10));

    expect(parados.map((e) => e.id)).toEqual([]);
  }, 120_000);
});
