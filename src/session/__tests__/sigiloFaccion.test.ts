// Sigilo de Facción (Doc 2.8.1): se elige al crear, es único y no cambia.
import { describe, expect, it } from 'vitest';
import type { Sigilo } from '../../domain/types';
import { GameSession } from '../gameSession';
import { crearFaccion } from '../comandos/crearFaccion';
import { conHeroe } from './fixtures';

const SIGILO: Sigilo = {
  formaId: 'aspis',
  campoId: 'cruz',
  emblemaId: 'leon',
  colorPrimarioId: 'azul',
  colorSecundarioId: 'blanco',
  colorEmblemaId: 'oro',
  orlaId: 'lisa',
  colorOrlaId: 'negro',
};

function partida(): GameSession {
  return conHeroe(conHeroe(GameSession.crear('partida-sigilo', { seed: 1 }), 'ana'), 'bea');
}

describe('crearFaccion con sigilo', () => {
  it('guarda el elegido y lo lleva en el evento', () => {
    const sesion = partida();
    const r = sesion.ejecutar(crearFaccion, { nombre: 'Micenas', sigilo: SIGILO }, { actor: 'ana' });
    expect(r.ok).toBe(true);
    expect(sesion.getState().facciones[0]!.sigilo).toEqual(SIGILO);
    expect((r.eventos[0]!.payload as { sigilo: Sigilo }).sigilo).toEqual(SIGILO);
  });

  it('sin sigilo recibe uno libre, y dos Facciones no lo repiten', () => {
    const sesion = partida();
    sesion.ejecutar(crearFaccion, { nombre: 'Micenas' }, { actor: 'ana' });
    sesion.ejecutar(crearFaccion, { nombre: 'Troya' }, { actor: 'bea' });
    const [a, b] = sesion.getState().facciones;
    expect(a!.sigilo).toBeDefined();
    expect(b!.sigilo).toBeDefined();
    expect(b!.sigilo).not.toEqual(a!.sigilo);
  });

  it('rechaza el duplicado exacto pero acepta uno parecido', () => {
    const sesion = partida();
    sesion.ejecutar(crearFaccion, { nombre: 'Micenas', sigilo: SIGILO }, { actor: 'ana' });
    const igual = sesion.ejecutar(crearFaccion, { nombre: 'Troya', sigilo: { ...SIGILO } }, { actor: 'bea' });
    expect(igual.ok).toBe(false);
    expect(igual.codigoError).toBe('faccion.sigilo_duplicado');
    const parecido = sesion.ejecutar(crearFaccion, { nombre: 'Troya', sigilo: { ...SIGILO, colorSecundarioId: 'oro' } }, { actor: 'bea' });
    expect(parecido.ok).toBe(true);
  });

  it('rechaza un sigilo que no es del catálogo', () => {
    const sesion = partida();
    const r = sesion.ejecutar(crearFaccion, { nombre: 'Micenas', sigilo: { ...SIGILO, emblemaId: 'bandidos' } }, { actor: 'ana' });
    expect(r.ok).toBe(false);
    expect(r.codigoError).toBe('faccion.sigilo_invalido');
    expect(sesion.getState().facciones).toEqual([]);
  });
});
