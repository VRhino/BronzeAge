// Fundar con la Caravana de Fundación de un campamento (D9-D16, D30, M2): se compra con el fondo, nace sin destino, la engancha su
// titular y funda con `fundar` donde esté.
import { describe, expect, it } from 'vitest';
import { MERCENARIOS } from '../../constants';
import { costoRefundacion } from '../../engine/refundacion';
import { crearMapa } from '../../world/mapa';
import { GameSession } from '../gameSession';
import { crearHeroe } from '../comandos/crearHeroe';
import { crearFaccion } from '../comandos/crearFaccion';
import { salirDelCampamento } from '../comandos/presencia';
import { aportarARefundacion, comprarCaravanaDeRefundacion } from '../comandos/mercenarios';
import { adjuntarCaravana } from '../comandos/ejercitos';
import { fundar } from '../comandos/expansion';

const AVATAR = { cabezaId: '', peloId: '', barbaId: '', cejasId: '' };

/** Un héroe nacido en `mercenarios-0` con su Facción, el coste en el almacén, la caravana comprada y su columna en la puerta. */
function conCaravanaComprada() {
  const sesion0 = GameSession.crear('fundar', { seed: 42 });
  const heroeId = sesion0.ejecutar(crearHeroe, { displayName: 'Ana', campamentoId: 'mercenarios-0', classDefinitionId: 'Spear', genero: 'femenino', avatar: AVATAR }, { actor: 'j1' }).datos!.heroeId;
  const opc = { actor: heroeId };
  const faccionId = sesion0.ejecutar(crearFaccion, { nombre: 'Micenas' }, opc).datos!.faccionId;
  const p = sesion0.exportar();
  const sesion = GameSession.importar({ ...p, state: { ...p.state, heroes: p.state.heroes.map((h) => (h.id === heroeId ? { ...h, almacenPersonal: costoRefundacion() } : h)) } });
  for (const [recurso, cantidad] of Object.entries(costoRefundacion())) sesion.ejecutar(aportarARefundacion, { recurso, cantidad }, opc);
  const caravanaId = sesion.ejecutar(comprarCaravanaDeRefundacion, {}, opc).datos!.caravanaId;
  const ejercitoId = sesion.ejecutar(salirDelCampamento, { campamentoId: 'mercenarios-0', heroeId, escuadronIds: [], carga: {} }, opc).datos!.ejercitoId;
  return { sesion, heroeId, faccionId, caravanaId, ejercitoId, opc };
}

/** Lleva la columna a `distancia` del campamento, a un punto de tierra (caminar costaría ticks que este test no mide). */
function aDistancia(sesion: GameSession, ejercitoId: string, distancia: number): GameSession {
  const p = sesion.exportar();
  const mapa = crearMapa(p.state.mapa, p.state.estadoMapa);
  const centro = p.state.campamentosMercenarios[0]!.posicion;
  const punto = Array.from({ length: 36 }, (_, i) => ({ x: centro.x + distancia * Math.cos((i * Math.PI) / 18), y: centro.y + distancia * Math.sin((i * Math.PI) / 18) })).find(
    (q) => mapa.dentroDelMapa(q) && mapa.terrenoEn(q) !== 'agua' && mapa.terrenoEn(q) !== 'cima'
  )!;
  const mover = <T extends { id: string; posicionActual: { x: number; y: number } }>(x: T) => ({ ...x, posicionActual: punto });
  return GameSession.importar({
    ...p,
    state: {
      ...p.state,
      ejercitos: p.state.ejercitos.map((e) => (e.id === ejercitoId ? mover(e) : e)),
      caravanas: p.state.caravanas.map((c) => (c.estado === 'adjunta' ? mover(c) : c)),
    },
  });
}

describe('fundar con la caravana de un campamento', () => {
  it('la engancha el titular, funda donde está y entra a vivir en la plaza nueva', () => {
    const { sesion, heroeId, faccionId, caravanaId, ejercitoId, opc } = conCaravanaComprada();
    expect(sesion.ejecutar(adjuntarCaravana, { ejercitoId, caravanaId, heroeId }, opc).ok).toBe(true);

    const lejos = aDistancia(sesion, ejercitoId, 300);
    const r = lejos.ejecutar(fundar, {}, opc);
    expect(r.ok).toBe(true);

    const estado = lejos.getState();
    const plaza = estado.asentamientos.find((a) => a.id === r.datos!.asentamientoId)!;
    expect(plaza).toMatchObject({ faccionId, heroesFundadoresIds: [heroeId] });
    expect(estado.caravanas.some((c) => c.id === caravanaId), 'la caravana se gasta').toBe(false);
    expect(estado.ejercitos.some((e) => e.id === ejercitoId), 'la columna entra').toBe(false);
    expect(estado.heroes.find((h) => h.id === heroeId)!.ubicacion).toEqual({ tipo: 'asentamiento', asentamientoId: plaza.id });
    expect(estado.campamentosMercenarios[0]!.residentesIds, 'deja el campamento').not.toContain(heroeId);
  });

  it('no se funda junto a un campamento (D16), ni sin la caravana enganchada', () => {
    const { sesion, heroeId, caravanaId, ejercitoId, opc } = conCaravanaComprada();
    expect(sesion.ejecutar(fundar, {}, opc).codigoError, 'sin enganchar').toBe('fundacion.sin_caravana');
    sesion.ejecutar(adjuntarCaravana, { ejercitoId, caravanaId, heroeId }, opc);
    const cerca = aDistancia(sesion, ejercitoId, MERCENARIOS.radioExclusionFundar - 20);
    expect(cerca.ejecutar(fundar, {}, opc).ok).toBe(false);
  });
});
