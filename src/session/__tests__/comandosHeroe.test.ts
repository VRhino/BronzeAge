// Alta del héroe de un jugador (`crearHeroe`) y de las Facciones NPC con sus héroes bot (`crearFaccionNpc`).
import { describe, expect, it } from 'vitest';
import { GameSession } from '../gameSession';
import { crearHeroe } from '../comandos/crearHeroe';
import { crearFaccionNpc } from '../comandos/crearFaccionNpc';

const PARAMS = { displayName: 'Ana', classDefinitionId: 'Spear', genero: 'femenino' as const, avatar: { cabezaId: 'c1', peloId: 'p1', barbaId: '', cejasId: 'e1' } };

describe('crearHeroe', () => {
  it('crea el héroe del jugador, lo hace aparecer con su columna, y solo uno por partida', () => {
    const sesion = GameSession.crear('heroe', { seed: 42 });
    const r = sesion.ejecutar(crearHeroe, PARAMS, { actor: 'jugador-1' });

    expect(r.ok).toBe(true);
    const heroe = sesion.getState().heroes.find((h) => h.id === r.datos!.heroeId)!;
    expect(heroe).toMatchObject({ jugadorId: 'jugador-1', controlador: 'humano', displayName: 'Ana', ubicacion: { tipo: 'columna' } });
    expect(sesion.getState().ejercitos.find((e) => e.liderId === heroe.id)?.origenAsentamientoId).toBe('');

    // Otra vez, como jugador o ya como su héroe: rechazado.
    expect(sesion.ejecutar(crearHeroe, PARAMS, { actor: 'jugador-1' }).codigoError).toBe('heroe.ya_existe');
    expect(sesion.ejecutar(crearHeroe, PARAMS, { actor: heroe.id }).codigoError).toBe('heroe.ya_existe');
    expect(sesion.ejecutar(crearHeroe, { ...PARAMS, displayName: '  ' }, { actor: 'jugador-2' }).codigoError).toBe('heroe.nombre_vacio');
  });
});

describe('crearFaccionNpc', () => {
  it('crea la Facción ya asentada y gobernada por el NPC, con sus héroes bot y el primero como Rey', () => {
    const sesion = GameSession.crear('npc', { seed: 1 });
    const r = sesion.ejecutar(crearFaccionNpc, { nombre: 'Tirinto' });

    expect(r.ok).toBe(true);
    const estado = sesion.getState();
    const faccion = estado.facciones.find((f) => f.id === r.datos!.faccionId)!;
    const asentamiento = estado.asentamientos.find((a) => a.id === r.datos!.asentamientoId)!;
    const bots = estado.heroes.filter((h) => asentamiento.heroesFundadoresIds.includes(h.id));

    expect(estado.faccionesNpcIds).toEqual([faccion.id]);
    expect(asentamiento.faccionId).toBe(faccion.id);
    expect(bots).toHaveLength(5);
    expect(bots.every((h) => h.controlador === 'bot' && h.jugadorId === null && faccion.ciudadanosIds.includes(h.id))).toBe(true);
    expect(bots.every((h) => h.ubicacion.tipo === 'asentamiento' && h.ubicacion.asentamientoId === asentamiento.id)).toBe(true);
    expect(faccion.reyId).toBe(bots[0]!.id);

    expect(sesion.ejecutar(crearFaccionNpc, { nombre: 'tirinto' }).codigoError).toBe('faccion.nombre_duplicado');
  });
});
