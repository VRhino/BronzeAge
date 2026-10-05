// La Caravana de Fundación lanzada desde una plaza (Doc 1.8, D30): la misma que la de un campamento. Nace parada en la plaza con su titular,
// que sale con su columna, la engancha, la lleva a un sitio y funda con `fundar`; se desarma a mano en la puerta; sin titular, vuelve sola.
import { describe, expect, it } from 'vitest';
import { FUNDACION, MERCENARIOS } from '../../constants';
import { costoCaravanaFundacion } from '../../engine/expansion';
import { evaluarViabilidadFundacion } from '../../engine/settlement';
import { crearMapa } from '../../world/mapa';
import { GameSession } from '../gameSession';
import { adjuntarCaravana, soltarCaravana } from '../comandos/ejercitos';
import { desarmarCaravanaFundacion, fundar, lanzarCaravanaFundacion } from '../comandos/expansion';
import { desconectarse, salirAlMundo } from '../comandos/presencia';
import { OPC, partidaConAsentamiento } from './fixtures';

/** La partida de la fixture con la plaza en nivel 2, el almacén lleno y la Facción con cupo para fundar otra. */
function conPlazaDeNivel2() {
  const base = partidaConAsentamiento();
  const p = base.sesion.exportar();
  const sesion = GameSession.importar({
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
  return { ...base, sesion };
}

/** Un punto fundable a esta distancia de la plaza: fuera de toda zona y a más de `radioExclusionFundar` de cualquier campamento. */
function sitioFundable(sesion: GameSession, desde: { x: number; y: number }, distancia: number) {
  const p = sesion.exportar();
  const mapa = crearMapa(p.state.mapa, p.state.estadoMapa);
  const lejosDeCampamentos = (q: { x: number; y: number }) => p.state.campamentosMercenarios.every((c) => Math.hypot(c.posicion.x - q.x, c.posicion.y - q.y) >= MERCENARIOS.radioExclusionFundar);
  const punto = Array.from({ length: 36 }, (_, i) => ({ x: Math.round(desde.x + distancia * Math.cos((i * Math.PI) / 18)), y: Math.round(desde.y + distancia * Math.sin((i * Math.PI) / 18)) })).find(
    (q) => lejosDeCampamentos(q) && evaluarViabilidadFundacion(mapa, q, p.state.asentamientos, p.state.facciones[0]!.id).fundable
  );
  if (!punto) throw new Error('el fixture no encontró un sitio fundable');
  return punto;
}

/** Lleva la columna y la caravana enganchada a un punto (caminar costaría ticks que este test no mide). */
function llevarA(sesion: GameSession, ejercitoId: string, punto: { x: number; y: number }): GameSession {
  const p = sesion.exportar();
  const mover = <T extends { posicionActual: { x: number; y: number } }>(x: T): T => ({ ...x, posicionActual: punto });
  return GameSession.importar({
    ...p,
    state: {
      ...p.state,
      ejercitos: p.state.ejercitos.map((e) => (e.id === ejercitoId ? mover(e) : e)),
      caravanas: p.state.caravanas.map((c) => (c.estado === 'adjunta' ? mover(c) : c)),
    },
  });
}

describe('lanzar y llevar la Caravana de Fundación de una plaza', () => {
  it('nace parada en la plaza, sin destino y con su titular, y cobra el coste del almacén', () => {
    const { sesion, asentamientoId, faccionId, fundador } = conPlazaDeNivel2();
    const antes = sesion.getState().asentamientos[0]!.almacen['madera']!.cantidad;
    const r = sesion.ejecutar(lanzarCaravanaFundacion, { origenAsentamientoId: asentamientoId }, OPC);
    expect(r.ok).toBe(true);

    const estado = sesion.getState();
    const caravana = estado.caravanas.find((c) => c.id === r.datos!.caravanaId)!;
    expect(caravana).toMatchObject({ tipo: 'construccion', estado: 'disponible', titularId: fundador, faccionId, origenAsentamientoId: asentamientoId });
    expect(caravana.posicionActual).toEqual(estado.asentamientos[0]!.posicion);
    expect(estado.asentamientos[0]!.almacen['madera']!.cantidad).toBe(antes - costoCaravanaFundacion()['madera']!);
  });

  it('el titular sale, la engancha, la lleva y funda donde está; la caravana se gasta y todos residen en la plaza nueva', () => {
    const { sesion, asentamientoId, fundador } = conPlazaDeNivel2();
    const caravanaId = sesion.ejecutar(lanzarCaravanaFundacion, { origenAsentamientoId: asentamientoId }, OPC).datos!.caravanaId;
    const ejercitoId = sesion.ejecutar(salirAlMundo, { asentamientoId, heroeId: fundador, escuadronIds: [], carga: {} }, OPC).datos!.ejercitoId;
    expect(sesion.ejecutar(adjuntarCaravana, { ejercitoId, caravanaId, heroeId: fundador }, OPC).ok, 'la engancha en la puerta').toBe(true);

    const plaza = sesion.getState().asentamientos[0]!;
    const lejos = llevarA(sesion, ejercitoId, sitioFundable(sesion, plaza.posicion, 250));
    const r = lejos.ejecutar(fundar, {}, OPC);
    expect(r.ok).toBe(true);

    const estado = lejos.getState();
    const nueva = estado.asentamientos.find((a) => a.id === r.datos!.asentamientoId)!;
    expect(nueva).toMatchObject({ faccionId: plaza.faccionId, heroesFundadoresIds: [fundador] });
    expect(estado.caravanas.some((c) => c.id === caravanaId)).toBe(false);
    expect(estado.ejercitos.some((e) => e.id === ejercitoId)).toBe(false);
  });

  it('no se funda dentro de la zona de la propia plaza, ni sin la caravana enganchada', () => {
    const { sesion, asentamientoId, fundador } = conPlazaDeNivel2();
    const caravanaId = sesion.ejecutar(lanzarCaravanaFundacion, { origenAsentamientoId: asentamientoId }, OPC).datos!.caravanaId;
    const ejercitoId = sesion.ejecutar(salirAlMundo, { asentamientoId, heroeId: fundador, escuadronIds: [], carga: {} }, OPC).datos!.ejercitoId;
    expect(sesion.ejecutar(fundar, {}, OPC).codigoError, 'sin enganchar').toBe('fundacion.sin_caravana');
    sesion.ejecutar(adjuntarCaravana, { ejercitoId, caravanaId, heroeId: fundador }, OPC);
    expect(sesion.ejecutar(fundar, {}, OPC).codigoError, 'a la puerta de su propia plaza').toBe('fundacion.invalida');
  });

  it('solo cabe una por cupo: con el cupo del Cap lleno por la caravana lanzada, otra se rechaza', () => {
    const { sesion, asentamientoId } = conPlazaDeNivel2();
    const p = sesion.exportar();
    const nivel1 = GameSession.importar({ ...p, state: { ...p.state, facciones: p.state.facciones.map((f) => ({ ...f, nivel: 1 })) } });
    expect(nivel1.ejecutar(lanzarCaravanaFundacion, { origenAsentamientoId: asentamientoId }, OPC).ok, 'cap 1 y ya hay una plaza').toBe(false);
  });
});

describe('desarmar y abandonar', () => {
  it('el titular la desarma en la puerta y recupera el coste; otro no puede', () => {
    const { sesion, asentamientoId, vecino } = conPlazaDeNivel2();
    const caravanaId = sesion.ejecutar(lanzarCaravanaFundacion, { origenAsentamientoId: asentamientoId }, OPC).datos!.caravanaId;
    expect(sesion.ejecutar(desarmarCaravanaFundacion, { caravanaId }, { actor: vecino }).ok, 'no es el titular').toBe(false);

    const sinCoste = sesion.getState().asentamientos[0]!.almacen['madera']!.cantidad;
    expect(sesion.ejecutar(desarmarCaravanaFundacion, { caravanaId }, OPC).ok).toBe(true);
    const estado = sesion.getState();
    expect(estado.caravanas).toEqual([]);
    expect(estado.asentamientos[0]!.almacen['madera']!.cantidad).toBe(sinCoste + costoCaravanaFundacion()['madera']!);
  });

  it('suelta en el camino, se queda donde está y no se desarma allí; caduca y devuelve a la plaza', () => {
    const { sesion, asentamientoId, fundador } = conPlazaDeNivel2();
    const caravanaId = sesion.ejecutar(lanzarCaravanaFundacion, { origenAsentamientoId: asentamientoId }, OPC).datos!.caravanaId;
    const ejercitoId = sesion.ejecutar(salirAlMundo, { asentamientoId, heroeId: fundador, escuadronIds: [], carga: {} }, OPC).datos!.ejercitoId;
    sesion.ejecutar(adjuntarCaravana, { ejercitoId, caravanaId, heroeId: fundador }, OPC);
    const lejos = llevarA(sesion, ejercitoId, sitioFundable(sesion, sesion.getState().asentamientos[0]!.posicion, 250));
    expect(lejos.ejecutar(soltarCaravana, { ejercitoId, caravanaId, heroeId: fundador }, OPC).ok).toBe(true);
    expect(lejos.ejecutar(desarmarCaravanaFundacion, { caravanaId }, OPC).ok, 'lejos de su puerta').toBe(false);

    const antes = lejos.getState().asentamientos[0]!.almacen['madera']!.cantidad;
    for (let i = 0; i < FUNDACION.caducidadCaravanaHoras * 60 + 2; i++) lejos.avanzarTick();
    const estado = lejos.getState();
    expect(estado.caravanas.some((c) => c.id === caravanaId), 'caducó').toBe(false);
    expect(estado.asentamientos[0]!.almacen['madera']!.cantidad).toBeGreaterThanOrEqual(antes + costoCaravanaFundacion()['madera']!);
  });

  it('si el titular sale del mundo con ella, vuelve sola a su plaza y allí se desarma', () => {
    const { sesion, asentamientoId, fundador } = conPlazaDeNivel2();
    const caravanaId = sesion.ejecutar(lanzarCaravanaFundacion, { origenAsentamientoId: asentamientoId }, OPC).datos!.caravanaId;
    const ejercitoId = sesion.ejecutar(salirAlMundo, { asentamientoId, heroeId: fundador, escuadronIds: [], carga: {} }, OPC).datos!.ejercitoId;
    sesion.ejecutar(adjuntarCaravana, { ejercitoId, caravanaId, heroeId: fundador }, OPC);
    // Dentro de la zona de su propia plaza, donde ningún bandido la toca: lo que se prueba es que vuelve, no que sobreviva al camino.
    const plaza = sesion.getState().asentamientos[0]!.posicion;
    const lejos = llevarA(sesion, ejercitoId, { x: plaza.x + 25, y: plaza.y });

    lejos.ejecutar(desconectarse, { heroeId: fundador }, OPC);
    const codigos: string[] = [];
    for (let i = 0; i < 400 && lejos.getState().caravanas.some((c) => c.id === caravanaId); i++) codigos.push(...lejos.avanzarTick().eventos.map((e) => e.codigo));
    expect(lejos.getState().caravanas.some((c) => c.id === caravanaId), 'llegó a casa y se desarmó').toBe(false);
    expect(codigos).toContain('fundacion.caravana_vuelve');
    expect(codigos).toContain('fundacion.caravana_devuelta');
  });
});
