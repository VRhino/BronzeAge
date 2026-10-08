// Un ejército entra en un campamento de mercenarios (2026-10-08): se desarma en la puerta y cada héroe entra como lo haría solo —el
// residente, normal; el que no reside, de visita con su columna aparcada—. Lo ordena su Líder.
import { describe, expect, it } from 'vitest';
import { GameSession } from '../gameSession';
import { crearHeroe } from '../comandos/crearHeroe';
import { crearFaccion } from '../comandos/crearFaccion';
import { responderSolicitud, solicitarIngreso } from '../comandos/ingresoEnFaccion';
import { REGISTRO_COMANDOS } from '../comandos/registro';

const AVATAR = { cabezaId: '', peloId: '', barbaId: '', cejasId: '' };
type Tipo = keyof typeof REGISTRO_COMANDOS;
const ejecutarEn = (sesion: GameSession) => (tipo: Tipo, heroeId: string, params: Record<string, unknown> = {}) =>
  sesion.ejecutar(REGISTRO_COMANDOS[tipo] as never, { heroeId, ...params } as never, { actor: heroeId });

/** Ana y Bea, de la misma Facción, con un ejército de las dos quieto a la puerta de `mercenarios-0`. `bea` reside allí salvo `visita`. */
function ejercitoDeDos(visita = false) {
  const base = GameSession.crear('entra', { seed: 42 });
  const crear = (actor: string, nombre: string) =>
    base.ejecutar(crearHeroe, { displayName: nombre, campamentoId: 'mercenarios-0', classDefinitionId: 'Spear', genero: 'femenino', avatar: AVATAR }, { actor }).datos!.heroeId;
  const ana = crear('j1', 'Ana');
  const bea = crear('j2', 'Bea');
  const faccionId = base.ejecutar(crearFaccion, { nombre: 'Micenas' }, { actor: ana }).datos!.faccionId;
  base.ejecutar(solicitarIngreso, { faccionId }, { actor: bea });
  base.ejecutar(responderSolicitud, { faccionId, heroeId: bea, aceptar: true }, { actor: ana });
  const ejecutar = ejecutarEn(base);
  ejecutar('convocarEjercito', ana, { politicaDeUnion: 'aceptar', escuadronIds: [], carga: {} });
  ejecutar('unirseAConvocatoria', bea, { convocatoriaId: base.getState().convocatorias![0]!.id, escuadronIds: [], carga: {} });
  expect(ejecutar('partirConvocatoria', ana).ok).toBe(true);

  // Bea "reside en otro sitio": se la saca de los residentes del campamento una vez fuera.
  const p = base.exportar();
  const sesion = visita
    ? GameSession.importar({ ...p, state: { ...p.state, campamentosMercenarios: p.state.campamentosMercenarios.map((c) => ({ ...c, residentesIds: c.residentesIds.filter((r) => r !== bea) })) } })
    : base;
  return { sesion, ana, bea, ejecutar: ejecutarEn(sesion), estado: () => sesion.getState() };
}

describe('un ejército entra en un campamento', () => {
  it('los residentes entran normal: el ejército deja de existir y los dos quedan dentro', () => {
    const { ana, bea, ejecutar, estado } = ejercitoDeDos();
    const ejercitoId = estado().ejercitos[0]!.id;

    const r = ejecutar('entrarEnCampamento', ana, { campamentoId: 'mercenarios-0' });

    expect(r.ok).toBe(true);
    expect(estado().ejercitos.find((e) => e.id === ejercitoId)).toBeUndefined();
    expect(estado().ejercitos, 'sin carro que sobre, no queda ninguna columna aparcada').toHaveLength(0);
    for (const h of [ana, bea]) expect(estado().heroes.find((x) => x.id === h)!.ubicacion).toEqual({ tipo: 'mercenarios', campamentoId: 'mercenarios-0' });
    expect(r.eventos.map((e) => e.codigo)).toContain('ejercito.entra_en_campamento');
  });

  it('quien no reside entra de visita: su columna queda aparcada a la puerta, intacta', () => {
    const { ana, bea, ejecutar, estado } = ejercitoDeDos(true);

    expect(ejecutar('entrarEnCampamento', ana, { campamentoId: 'mercenarios-0' }).ok).toBe(true);

    expect(estado().heroes.find((x) => x.id === bea)!.ubicacion).toEqual({ tipo: 'mercenarios', campamentoId: 'mercenarios-0' });
    const suya = estado().ejercitos.find((e) => e.participantes.some((p) => p.heroeId === bea))!;
    expect(suya).toMatchObject({ tipo: 'personal', estado: 'estacionado' });
    expect(suya.participantes).toHaveLength(1);
    expect(estado().ejercitos.some((e) => e.participantes.some((p) => p.heroeId === ana)), 'la residente no deja columna').toBe(false);
  });

  it('solo el Líder lo hace entrar, y en la puerta', () => {
    const { sesion, ana, bea, ejecutar, estado } = ejercitoDeDos();
    const rechazo = ejecutar('entrarEnCampamento', bea, { campamentoId: 'mercenarios-0' });
    expect(rechazo.ok === false && rechazo.detalleError).toMatch(/Solo el Líder/);
    expect(estado().ejercitos).toHaveLength(1);

    const p = sesion.exportar();
    const lejos = GameSession.importar({
      ...p,
      state: { ...p.state, ejercitos: p.state.ejercitos.map((e) => ({ ...e, posicionActual: { x: e.posicionActual.x + 500, y: e.posicionActual.y } })) },
    });
    expect(ejecutarEn(lejos)('entrarEnCampamento', ana, { campamentoId: 'mercenarios-0' }).ok).toBe(false);
  });
});
