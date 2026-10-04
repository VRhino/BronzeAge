// Fundar con la Caravana de Fundación de un campamento (D9-D16, D30, M2): se compra con el fondo, nace sin destino, la engancha su
// titular y funda con `fundar` donde esté.
import { describe, expect, it } from 'vitest';
import { MERCENARIOS } from '../../constants';
import { costoRefundacion } from '../../engine/refundacion';
import { crearMapa } from '../../world/mapa';
import { GameSession } from '../gameSession';
import { crearHeroe } from '../comandos/crearHeroe';
import { crearFaccion } from '../comandos/crearFaccion';
import { entrarEnCampamento, salirDelCampamento } from '../comandos/presencia';
import { aportarARefundacion, comprarCaravanaDeRefundacion } from '../comandos/mercenarios';
import { adjuntarCaravana, replegarEjercito } from '../comandos/ejercitos';
import { fundar } from '../comandos/expansion';
import { unirseEnCampo } from '../comandos/columna';
import { desconectarse } from '../comandos/presencia';
import { responderSolicitud, solicitarIngreso } from '../comandos/ingresoEnFaccion';
import { PRESENCIA } from '../../constants';

const AVATAR = { cabezaId: '', peloId: '', barbaId: '', cejasId: '' };

/** Un héroe nacido en `mercenarios-0` con su Facción, el coste en el almacén, la caravana comprada y su columna en la puerta. */
function conCaravanaComprada() {
  const sesion0 = GameSession.crear('fundar', { seed: 42 });
  const heroeId = sesion0.ejecutar(crearHeroe, { displayName: 'Ana', campamentoId: 'mercenarios-0', classDefinitionId: 'Spear', genero: 'femenino', avatar: AVATAR }, { actor: 'j1' }).datos!.heroeId;
  const opc = { actor: heroeId };
  const faccionId = sesion0.ejecutar(crearFaccion, { nombre: 'Micenas' }, opc).datos!.faccionId;
  const p = sesion0.exportar();
  const sesion = GameSession.importar({ ...p, state: { ...p.state, heroes: p.state.heroes.map((h) => (h.id === heroeId ? { ...h, almacenPersonal: costoRefundacion() } : h)) } });
  for (const [recurso, cantidad] of Object.entries(costoRefundacion())) sesion.ejecutar(aportarARefundacion, { recurso, cantidad, lado: 'almacen' }, opc);
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

describe('sin su titular (D13, D40)', () => {
  it('si el titular sale del mundo, vuelve sola, y otro ciudadano de su Facción la reclama por el camino', () => {
    const base = conCaravanaComprada();
    base.sesion.ejecutar(adjuntarCaravana, { ejercitoId: base.ejercitoId, caravanaId: base.caravanaId, heroeId: base.heroeId }, base.opc);
    const sesion = aDistancia(base.sesion, base.ejercitoId, 300);
    // Un segundo ciudadano, aceptado por el Rey (el titular), con su columna fuera.
    const otro = sesion.ejecutar(crearHeroe, { displayName: 'Bea', campamentoId: 'mercenarios-0', classDefinitionId: 'Spear', genero: 'femenino', avatar: AVATAR }, { actor: 'j2' }).datos!.heroeId;
    sesion.ejecutar(solicitarIngreso, { faccionId: base.faccionId }, { actor: otro });
    sesion.ejecutar(responderSolicitud, { faccionId: base.faccionId, heroeId: otro, aceptar: true }, base.opc);
    const suEjercito = sesion.ejecutar(salirDelCampamento, { campamentoId: 'mercenarios-0', heroeId: otro, escuadronIds: [], carga: {} }, { actor: otro }).datos!.ejercitoId;
    expect(sesion.ejecutar(adjuntarCaravana, { ejercitoId: suEjercito, caravanaId: base.caravanaId, heroeId: otro }, { actor: otro }).ok, 'la lleva su titular').toBe(false);

    sesion.ejecutar(desconectarse, { heroeId: base.heroeId }, base.opc);
    for (let i = 0; i <= Math.ceil(PRESENCIA.retardoDesconexionMs / 60_000) + 1; i++) sesion.avanzarTick();
    const caravana = sesion.getState().caravanas.find((c) => c.id === base.caravanaId)!;
    expect(caravana.estado).toBe('retornando');

    // Lleva su columna adonde va la caravana y la reclama.
    const p = sesion.exportar();
    const alli = GameSession.importar({ ...p, state: { ...p.state, ejercitos: p.state.ejercitos.map((e) => (e.id === suEjercito ? { ...e, posicionActual: caravana.posicionActual } : e)) } });
    expect(alli.ejecutar(adjuntarCaravana, { ejercitoId: suEjercito, caravanaId: base.caravanaId, heroeId: otro }, { actor: otro }).ok).toBe(true);
    expect(alli.getState().caravanas.find((c) => c.id === base.caravanaId)).toMatchObject({ estado: 'adjunta', titularId: otro });
  });
});

describe('fundar en grupo desde el campamento (M2, D21)', () => {
  it('el titular sale como ejército, otro ciudadano se le une en campo y los dos cofundan', () => {
    const sesion0 = GameSession.crear('grupo', { seed: 42 });
    const crear = (actor: string, nombre: string) =>
      sesion0.ejecutar(crearHeroe, { displayName: nombre, campamentoId: 'mercenarios-0', classDefinitionId: 'Spear', genero: 'femenino', avatar: AVATAR }, { actor }).datos!.heroeId;
    const ana = crear('j1', 'Ana');
    const bea = crear('j2', 'Bea');
    const faccionId = sesion0.ejecutar(crearFaccion, { nombre: 'Micenas' }, { actor: ana }).datos!.faccionId;
    sesion0.ejecutar(solicitarIngreso, { faccionId }, { actor: bea });
    sesion0.ejecutar(responderSolicitud, { faccionId, heroeId: bea, aceptar: true }, { actor: ana });
    const p = sesion0.exportar();
    const sesion = GameSession.importar({ ...p, state: { ...p.state, heroes: p.state.heroes.map((h) => (h.id === ana ? { ...h, almacenPersonal: costoRefundacion() } : h)) } });
    for (const [recurso, cantidad] of Object.entries(costoRefundacion())) sesion.ejecutar(aportarARefundacion, { recurso, cantidad, lado: 'almacen' }, { actor: ana });
    const caravanaId = sesion.ejecutar(comprarCaravanaDeRefundacion, {}, { actor: ana }).datos!.caravanaId;

    expect(
      sesion.ejecutar(salirDelCampamento, { campamentoId: 'mercenarios-0', heroeId: ana, escuadronIds: [], carga: {}, politicaDeUnion: 'aceptar' }, { actor: ana }).ok,
      'un ejército sale con rumbo'
    ).toBe(false);
    const ejercitoId = sesion.ejecutar(salirDelCampamento, { campamentoId: 'mercenarios-0', heroeId: ana, escuadronIds: [], carga: {}, politicaDeUnion: 'aceptar', objetivo: { tipo: 'punto', punto: { x: 1000, y: 1000 } } }, { actor: ana }).datos!.ejercitoId;
    expect(sesion.getState().ejercitos.find((e) => e.id === ejercitoId)).toMatchObject({ tipo: 'ejercito', estado: 'marchando' });
    sesion.ejecutar(adjuntarCaravana, { ejercitoId, caravanaId, heroeId: ana }, { actor: ana });
    sesion.ejecutar(salirDelCampamento, { campamentoId: 'mercenarios-0', heroeId: bea, escuadronIds: [], carga: {} }, { actor: bea });
    expect(sesion.ejecutar(unirseEnCampo, { ejercitoId, heroeId: bea }, { actor: bea }).ok).toBe(true);

    const lejos = aDistancia(sesion, ejercitoId, 300);
    const r = lejos.ejecutar(fundar, {}, { actor: ana });
    expect(r.ok).toBe(true);
    expect(lejos.getState().asentamientos.find((a) => a.id === r.datos!.asentamientoId)!.heroesFundadoresIds.sort()).toEqual([ana, bea].sort());
  });
});

describe('un ejército salido del campamento vuelve a él', () => {
  it('se repliega a su puerta, se queda como columna propia si va solo, y entra', () => {
    const { sesion, heroeId, opc } = conCaravanaComprada();
    // conCaravanaComprada sale como columna personal: entra y sale otra vez como ejército con rumbo.
    sesion.ejecutar(entrarEnCampamento, { campamentoId: 'mercenarios-0', heroeId }, opc);
    const ejercitoId = sesion.ejecutar(
      salirDelCampamento,
      { campamentoId: 'mercenarios-0', heroeId, escuadronIds: [], carga: {}, politicaDeUnion: 'aceptar', objetivo: { tipo: 'punto', punto: { x: 1000, y: 1000 } } },
      opc
    ).datos!.ejercitoId;
    for (let i = 0; i < 3; i++) sesion.avanzarTick();
    expect(sesion.ejecutar(replegarEjercito, { ejercitoId }, opc).ok).toBe(true);
    for (let i = 0; i < 20 && sesion.getState().ejercitos.find((e) => e.id === ejercitoId)?.estado !== 'estacionado'; i++) sesion.avanzarTick();

    const ejercito = sesion.getState().ejercitos.find((e) => e.id === ejercitoId)!;
    expect(ejercito).toMatchObject({ estado: 'estacionado', tipo: 'personal' });
    expect(sesion.ejecutar(entrarEnCampamento, { campamentoId: 'mercenarios-0', heroeId }, opc).ok).toBe(true);
  });
});
