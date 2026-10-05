// Los bots juegan desde fuera, como jugadores (Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md §2-§4): ven por la proyección,
// actúan con comandos y su autorización, y con el adaptador en proceso la misma semilla da la misma partida.
import { describe, expect, it } from 'vitest';
import { GameSession } from '../../session/gameSession';
import { faccionAsentadaDePrueba } from '../../session/__tests__/faccionAsentadaDePrueba';
import type { EventoDominio } from '../../domain/eventos';
import { puertoEnProceso } from '../puerto';
import { RunnerDeBots } from '../runner';
import { cerebroDeBot } from '../cerebro';

/** Tres Facciones de bots con el andamio del batch, y su runner. Con la semilla 7, dos nacen a la vista una de otra. */
async function mundo(ticks: number, horario: 'siempre' | 'por-semilla' = 'siempre') {
  const sesion = GameSession.crear('bots', { seed: 7 });
  for (const nombre of ['Alfa', 'Beta', 'Gamma']) sesion.ejecutar(faccionAsentadaDePrueba, { nombre });
  const bots = new RunnerDeBots(puertoEnProceso(sesion), cerebroDeBot, { semilla: 7, horario });
  for (const h of sesion.getState().heroes) bots.alta(h.id);
  const eventos: EventoDominio[] = [];
  for (let tick = 1; tick <= ticks; tick++) {
    const r = sesion.avanzarTick();
    await bots.trasTick(tick, r.eventos);
    eventos.push(...sesion.getState().eventosDominio.filter((e) => e.version >= r.version).reverse());
  }
  return { sesion, eventos };
}

describe('bots: juegan con los comandos de un jugador', () => {
  it('el Rey nombra Gobernador, el Gobernador se hace Tesorero y el Tesorero aparta la madera', async () => {
    // Un turno cada uno (cada 5 ticks): cada bot decide con la vista con la que empezó a pensar.
    const { sesion } = await mundo(20);

    for (const plaza of sesion.getState().asentamientos) {
      expect(plaza.cargos.gobernadorId, plaza.id).toBeTruthy();
      expect(plaza.cargos.tesoreroId).toBe(plaza.cargos.gobernadorId);
      expect(plaza.reservaManual?.madera).toBe(150);
    }
  });

  it('salen a cazar el campamento de bandidos de su plaza, lo atacan con `atacar` y vuelven', async () => {
    const { sesion, eventos } = await mundo(600);

    expect(eventos.some((e) => e.codigo === 'combate.campamento_destruido')).toBe(true);
    expect(eventos.some((e) => e.codigo === 'ejercito.repliegue')).toBe(true);
    expect(sesion.getState().ejercitos.every((e) => e.estado !== 'estacionado'), 'nadie se queda plantado').toBe(true);
  });

  it('el explorador inspecciona plazas ajenas desde cerca: lo que sabe, lo ha ido a mirar', async () => {
    const { eventos } = await mundo(300);

    expect(eventos.some((e) => e.codigo === 'asentamiento.observado')).toBe(true);
  });

  it('misma semilla, misma partida: los bots piensan en orden de id, cada uno con su RNG', async () => {
    expect(JSON.stringify((await mundo(200)).sesion.exportar())).toBe(JSON.stringify((await mundo(200)).sesion.exportar()));
  });
});

describe('bots: la Caravana de Fundación desde la plaza (Doc 1.8)', () => {
  it('un residente sin cargo la lanza, sale con ella enganchada, la lleva a un sitio y funda una plaza nueva', async () => {
    const sesion = GameSession.crear('bots', { seed: 7 });
    sesion.ejecutar(faccionAsentadaDePrueba, { nombre: 'Alfa' });
    const p = sesion.exportar();
    // Nivel 2 con el almacén lleno y cupo de Facción para una segunda plaza.
    const listo = GameSession.importar({
      ...p,
      state: {
        ...p.state,
        facciones: p.state.facciones.map((f) => ({ ...f, nivel: 3 })),
        asentamientos: p.state.asentamientos.map((a) => ({
          ...a,
          nivel: 2,
          nivelActual: 2,
          almacen: Object.fromEntries(Object.entries(a.almacen).map(([r, item]) => [r, { ...item, cantidad: item.capacidad }])),
        })),
      },
    });
    const bots = new RunnerDeBots(puertoEnProceso(listo), cerebroDeBot, { semilla: 7, horario: 'siempre' });
    for (const h of listo.getState().heroes) bots.alta(h.id);
    const codigos = new Set<string>();
    for (let tick = 1; tick <= 400 && listo.getState().asentamientos.length < 2; tick++) {
      const r = listo.avanzarTick();
      await bots.trasTick(tick, r.eventos);
      for (const e of listo.getState().eventosDominio.filter((x) => x.version >= r.version)) codigos.add(e.codigo);
    }

    expect(codigos.has('expansion.caravana_lanzada')).toBe(true);
    expect(codigos.has('ejercito.caravana_adjuntada')).toBe(true);
    expect(codigos.has('fundacion.asentamiento_fundado')).toBe(true);
    expect(listo.getState().asentamientos).toHaveLength(2);
    expect(listo.getState().caravanas.filter((c) => c.tipo === 'construccion'), 'la caravana se gastó al fundar').toEqual([]);
  });
});

describe('sesiones de los bots (D55)', () => {
  it('cada bot juega unas horas al día: fuera de su sesión se desconecta y sale del mundo como un humano', async () => {
    const { sesion, eventos } = await mundo(24 * 60, 'por-semilla');
    const heroes = sesion.getState().heroes.map((h) => h.id);
    const salen = new Set(eventos.filter((e) => e.codigo === 'jugador.sale_del_mundo').map((e) => (e.payload as { heroeId: string }).heroeId));
    const vuelven = new Set(eventos.filter((e) => e.codigo === 'jugador.vuelve_al_mundo').map((e) => (e.payload as { heroeId: string }).heroeId));

    expect(salen.size, 'todos salen del mundo en algún momento del día').toBe(heroes.length);
    expect(vuelven.size, 'y vuelven al empezar su sesión').toBeGreaterThan(0);
    const fuera = sesion.getState().heroes.filter((h) => h.fuera).length;
    expect(fuera, 'a cualquier hora, la mayoría está fuera').toBeGreaterThan(heroes.length / 2);
  });
});

describe('puerto en proceso', () => {
  it('pasa por la autorización de un jugador: un bot no puede hacer lo que su cargo no le permite', async () => {
    const sesion = GameSession.crear('bots', { seed: 7 });
    sesion.ejecutar(faccionAsentadaDePrueba, { nombre: 'Alfa' });
    const plaza = sesion.getState().asentamientos[0]!;
    const noRey = plaza.heroesFundadoresIds[1]!;

    const r = await puertoEnProceso(sesion).actuar(noRey, 'asignarCargoLocal', { asentamientoId: plaza.id, cargo: 'gobernador', heroeId: noRey });

    expect(r.ok).toBe(false);
    expect(r.noAutorizado).toBeDefined();
    expect(sesion.getState().asentamientos[0]!.cargos.gobernadorId).toBeNull();
  });
});
