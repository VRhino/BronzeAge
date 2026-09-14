// Movilizar exige residir (Doc 5.15.2, fase 2 del Héroe): la tropa de un héroe está en su campamento, y el
// campamento está en su residencia. La revisión 2026-09-08 dejaba salir a un no-residente con escuadras
// "posadas" en la plaza tras conquistar o guarnecer; ya no existen esas escuadras sueltas.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, Escuadron } from '../../domain/types';
import { movilizarEjercito, MovilizacionInvalidaError } from '../ejercitos';
import { crearFacciones, crearMapaDeterminista, escuadronDePrueba, fundarAsentamientoDeTest, instanteDeTest, posicionRecomendable } from './fixtures';

const mapa = crearMapaDeterminista(42);

function plaza(faccionId: string, existentes: Asentamiento[] = []): Asentamiento {
  const { asentamiento } = fundarAsentamientoDeTest(mapa, crearFacciones(), faccionId, existentes, 0, posicionRecomendable(mapa, existentes));
  return { ...asentamiento, poblacion: { pesants: 300, artesanos: 0, nobleza: 0 } };
}

describe('movilizarEjercito exige residir', () => {
  const origen = plaza('faccion-1'); // reside jugador-faccion-1-1
  const destino = plaza('faccion-2', [origen]);
  const salir = (heroeId: string, campamento: Escuadron[]) =>
    movilizarEjercito(
      origen,
      campamento,
      undefined,
      heroeId,
      [`esc-${heroeId}`],
      { tipo: 'asentamiento', id: destino.id },
      [origen, destino],
      mapa,
      'ejercito-test',
      instanteDeTest(0)
    );

  it('el residente saca a campaña las escuadras de su campamento', () => {
    const r = salir('jugador-faccion-1-1', [escuadronDePrueba('esc-jugador-faccion-1-1', 'jugador-faccion-1-1', 'milicia_lanceros', 20)]);
    expect(r.ejercito.escuadronIds).toEqual(['esc-jugador-faccion-1-1']);
  });

  it('un no-residente NO moviliza desde aquí, aunque tenga escuadras delante', () => {
    expect(() => salir('jugador-ocupante', [escuadronDePrueba('esc-jugador-ocupante', 'jugador-ocupante', 'milicia_lanceros', 20)])).toThrow(
      MovilizacionInvalidaError
    );
  });
});
