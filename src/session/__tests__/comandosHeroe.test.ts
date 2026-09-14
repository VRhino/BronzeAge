// El héroe como personaje: su alta (`crearHeroe`), la de las Facciones NPC con sus héroes bot (`crearFaccionNpc`),
// y los comandos que hace sobre sí mismo (puntos y loadouts, doc 02 §4.2).
import { describe, expect, it } from 'vitest';
import type { Heroe } from '../../domain/types';
import { GameSession } from '../gameSession';
import { crearHeroe } from '../comandos/crearHeroe';
import { crearFaccionNpc } from '../comandos/crearFaccionNpc';
import { borrarLoadout, guardarLoadout, repartirPuntos } from '../comandos/heroe';
import { HEROE, LIDERAZGO } from '../../constants';
import { escuadronDePrueba } from '../../engine/__tests__/fixtures';

const PARAMS = { displayName: 'Ana', classDefinitionId: 'Spear', genero: 'femenino' as const, avatar: { cabezaId: 'c1', peloId: 'p1', barbaId: '', cejasId: 'e1' } };

/** Partida con el héroe de `jugador-1`, retocado vía `importar` (hoy nada da puntos de atributo, como en Conquest;
 * decisión del usuario 2026-09-14). */
function heroeCon(cambios: (heroeId: string) => Partial<Heroe>) {
  const creada = GameSession.crear('heroe', { seed: 42 });
  const heroeId = creada.ejecutar(crearHeroe, PARAMS, { actor: 'jugador-1' }).datos!.heroeId;
  const payload = creada.exportar();
  const sesion = GameSession.importar({
    ...payload,
    state: { ...payload.state, heroes: payload.state.heroes.map((h) => (h.id === heroeId ? { ...h, ...cambios(heroeId) } : h)) },
  });
  return { sesion, heroeId, heroe: () => sesion.getState().heroes.find((h) => h.id === heroeId)! };
}

/** Dos escuadras de leva (7 de Liderazgo cada una, Doc 5.11.1). */
const dosDeLeva = (heroeId: string) => [escuadronDePrueba('esc-1', heroeId, 'milicia_lanceros'), escuadronDePrueba('esc-2', heroeId, 'lanceros_mimbre')];

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

  it('nace como en Conquest: nivel 1, sin puntos, 500 de bronce y el loadout "Default" vacío y activo', () => {
    const { heroe } = heroeCon(() => ({}));

    expect(heroe()).toMatchObject({ nivel: 1, puntosDeAtributoSinGastar: 0, puntosDePerkSinGastar: 0, inventario: [] });
    expect(heroe().monedasHeroe).toEqual({ bronce: HEROE.bronceInicial, plata: 0, oro: 0 });
    expect(heroe().loadouts).toEqual([
      { id: `${heroe().id}-loadout-default`, displayName: 'Default', squadIds: [], perksSeleccionados: [], activo: true },
    ]);
    expect(Object.values(heroe().equipamiento).every((hueco) => hueco === null)).toBe(true);
  });
});

describe('repartirPuntos', () => {
  it('cada punto suma 1 al atributo y sale de la bolsa', () => {
    const { sesion, heroeId, heroe } = heroeCon(() => ({ puntosDeAtributoSinGastar: 3 }));

    expect(sesion.ejecutar(repartirPuntos, { atributos: { fuerza: 2, vitalidad: 1 } }, { actor: heroeId }).ok).toBe(true);

    expect(heroe().atributosBase).toEqual({ fuerza: 2, destreza: 0, armadura: 0, vitalidad: 1 });
    expect(heroe().puntosDeAtributoSinGastar).toBe(0);
  });

  it('no se gastan más puntos de los que hay, ni se reparten negativos, ni se pasa del tope', () => {
    const { sesion, heroeId, heroe } = heroeCon(() => ({ puntosDeAtributoSinGastar: 2 }));
    const repartir = (atributos: Record<string, number>) => sesion.ejecutar(repartirPuntos, { atributos }, { actor: heroeId }).codigoError;

    expect(repartir({ fuerza: 3 })).toBe('heroe.invalido');
    expect(repartir({ fuerza: 3, destreza: -1 })).toBe('heroe.invalido');
    expect(repartir({})).toBe('heroe.invalido');
    expect(heroe().puntosDeAtributoSinGastar, 'un rechazo no gasta nada').toBe(2);

    const alTope = heroeCon(() => ({ puntosDeAtributoSinGastar: 2, atributosBase: { fuerza: HEROE.topeAtributo, destreza: 0, armadura: 0, vitalidad: 0 } }));
    expect(alTope.sesion.ejecutar(repartirPuntos, { atributos: { fuerza: 1 } }, { actor: alTope.heroeId }).codigoError).toBe('heroe.invalido');
  });
});

describe('loadouts (Doc 5.16.5)', () => {
  it('guarda uno nuevo, inactivo, y devuelve el Liderazgo que suma', () => {
    const { sesion, heroeId, heroe } = heroeCon((id) => ({ escuadrones: dosDeLeva(id) }));

    const r = sesion.ejecutar(guardarLoadout, { displayName: 'Incursión', squadIds: ['esc-1', 'esc-2'], perksSeleccionados: [] }, { actor: heroeId });

    expect(r.datos).toEqual({ loadoutId: expect.any(String), liderazgoTotal: 2 * LIDERAZGO.costePorEscalon[1]! });
    expect(heroe().loadouts.find((l) => l.id === r.datos!.loadoutId)).toMatchObject({ displayName: 'Incursión', squadIds: ['esc-1', 'esc-2'], activo: false });
  });

  it('marcar uno activo desactiva los demás, y reescribirlo por id no crea otro', () => {
    const { sesion, heroeId, heroe } = heroeCon((id) => ({ escuadrones: dosDeLeva(id) }));
    const loadoutId = sesion.ejecutar(guardarLoadout, { displayName: 'A', squadIds: ['esc-1'], perksSeleccionados: [], activo: true }, { actor: heroeId })
      .datos!.loadoutId;

    expect(heroe().loadouts.filter((l) => l.activo).map((l) => l.id)).toEqual([loadoutId]);

    sesion.ejecutar(guardarLoadout, { loadoutId, displayName: 'B', squadIds: ['esc-2'], perksSeleccionados: [] }, { actor: heroeId });
    expect(heroe().loadouts).toHaveLength(2);
    expect(heroe().loadouts.find((l) => l.id === loadoutId)).toMatchObject({ displayName: 'B', squadIds: ['esc-2'], activo: true });
  });

  it('rechaza escuadras ajenas, pasarse de Liderazgo, perks que no tiene y loadouts que no existen', () => {
    const { sesion, heroeId } = heroeCon((id) => ({ escuadrones: dosDeLeva(id), liderazgoBase: LIDERAZGO.costePorEscalon[1]! }));
    const guardar = (params: Parameters<typeof guardarLoadout>[3]) => sesion.ejecutar(guardarLoadout, params, { actor: heroeId }).codigoError;

    expect(guardar({ displayName: 'x', squadIds: ['esc-de-otro'], perksSeleccionados: [] })).toBe('heroe.invalido');
    expect(guardar({ displayName: 'x', squadIds: ['esc-1', 'esc-2'], perksSeleccionados: [] }), 'dos de leva no caben en 7').toBe('heroe.invalido');
    expect(guardar({ displayName: 'x', squadIds: [], perksSeleccionados: [3] })).toBe('heroe.invalido');
    expect(guardar({ loadoutId: 'no-existe', displayName: 'x', squadIds: [], perksSeleccionados: [] })).toBe('heroe.invalido');
    expect(guardar({ displayName: '  ', squadIds: [], perksSeleccionados: [] })).toBe('heroe.invalido');
  });

  it('borrar lo quita, y borrar uno que no existe se rechaza', () => {
    const { sesion, heroeId, heroe } = heroeCon(() => ({}));
    const porDefecto = heroe().loadouts[0]!.id;

    expect(sesion.ejecutar(borrarLoadout, { loadoutId: porDefecto }, { actor: heroeId }).ok).toBe(true);
    expect(heroe().loadouts).toEqual([]);
    expect(sesion.ejecutar(borrarLoadout, { loadoutId: porDefecto }, { actor: heroeId }).codigoError).toBe('heroe.invalido');
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
