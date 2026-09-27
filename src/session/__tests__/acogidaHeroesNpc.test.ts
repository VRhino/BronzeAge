// Los héroes bot nunca se quedan sin casa (`session/npcGobernanza.ts`, decisión del usuario 2026-09-27). Medido en
// la Era I: ~75 héroes huérfanos, casi todos de Facciones sin ninguna plaza.
import { describe, expect, it } from 'vitest';
import type { Faccion, Heroe } from '../../domain/types';
import { registrarDerrota } from '../../engine/faccion';
import { createRng } from '../../worldgen';
import { avanzarNpcGobernanza } from '../npcGobernanza';
import {
  contextoDeTest,
  crearEstadoDeTest,
  crearFacciones,
  crearMapaDeterminista,
  fundarAsentamientoDeTest,
  heroeDePrueba,
} from '../../engine/__tests__/fixtures';

const BOT = 'bot-derrotado';

/** La Facción 1 conserva su plaza; la 2 perdió la última ante la 1 y le queda un héroe bot, sin casa. */
function escenario(faccionesNpc?: string[]) {
  const mapa = crearMapaDeterminista(7);
  const uno = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
  const facciones: Faccion[] = uno.facciones.map((f) =>
    f.id === 'faccion-2' ? { ...f, ciudadanosIds: [BOT], derrotadaPor: 'faccion-1' } : f
  );
  const bot: Heroe = heroeDePrueba(BOT, { tipo: 'desconectado', punto: uno.asentamiento.posicion }, { controlador: 'bot' });
  const r = avanzarNpcGobernanza(crearEstadoDeTest([uno.asentamiento], facciones, { heroes: [bot] }), mapa, contextoDeTest(1, createRng(5)), {
    ...(faccionesNpc ? { faccionesIds: faccionesNpc } : {}),
    lanzarCampanas: false,
  });
  return { r, plaza: uno.asentamiento };
}

describe('acogida de los héroes bot', () => {
  it('la Facción que pierde su último asentamiento queda marcada con quién la derrotó', () => {
    const facciones = crearFacciones();
    expect(registrarDerrota(facciones, [], 'faccion-2', 'faccion-1').find((f) => f.id === 'faccion-2')!.derrotadaPor).toBe('faccion-1');
  });

  it('una Facción NPC derrotada por otra NPC se une a ella y desaparece; su héroe pasa a residir en la ganadora', () => {
    const { r, plaza } = escenario();
    expect(r.estado.facciones.some((f) => f.id === 'faccion-2')).toBe(false);
    expect(r.estado.facciones.find((f) => f.id === 'faccion-1')!.ciudadanosIds).toContain(BOT);
    expect(r.estado.asentamientos.find((a) => a.id === plaza.id)!.casasCompradas).toContain(BOT);
  });

  it('una Facción NPC que pierde su última plaza por colapso se disuelve y sus héroes bot desaparecen', () => {
    const mapa = crearMapaDeterminista(7);
    const uno = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
    const facciones: Faccion[] = uno.facciones.map((f) => (f.id === 'faccion-2' ? { ...f, ciudadanosIds: [BOT], derrotadaPor: null } : f));
    const bot = heroeDePrueba(BOT, { tipo: 'desconectado', punto: uno.asentamiento.posicion }, { controlador: 'bot' });
    const r = avanzarNpcGobernanza(crearEstadoDeTest([uno.asentamiento], facciones, { heroes: [bot] }), mapa, contextoDeTest(1, createRng(5)), {
      lanzarCampanas: false,
    });
    expect(r.estado.facciones.some((f) => f.id === 'faccion-2')).toBe(false);
    expect(r.estado.heroes.some((h) => h.id === BOT)).toBe(false);
  });

  it('si la ganadora es de un jugador, la derrotada no se le une', () => {
    const { r } = escenario(['faccion-2']);
    expect(r.estado.facciones.find((f) => f.id === 'faccion-2')!.ciudadanosIds).toContain(BOT);
    expect(r.estado.facciones.find((f) => f.id === 'faccion-1')!.ciudadanosIds).not.toContain(BOT);
  });
});
