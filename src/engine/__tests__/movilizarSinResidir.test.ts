// Reclutamiento/movimiento desatado de residencia (Doc 5.4/5.8, revisión 2026-09-08): `movilizarEjercito`
// deja de exigir residir — basta con tener escuadrones vivos propios ya posados en la plaza (guarnición tras
// conquistar/guarnecer). Reclutar/cambiar roster sí sigue atado a residir (ver tropas.test.ts).
import { describe, expect, it } from 'vitest';
import type { Asentamiento, Escuadron } from '../../domain/types';
import { movilizarEjercito, MovilizacionInvalidaError } from '../ejercitos';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, instanteDeTest, posicionRecomendable } from './fixtures';

const mapa = crearMapaDeterminista(42);

function plaza(faccionId: string, existentes: Asentamiento[] = []): Asentamiento {
  const { asentamiento } = fundarAsentamientoDeTest(mapa, crearFacciones(), faccionId, existentes, 0, posicionRecomendable(mapa, existentes));
  return { ...asentamiento, poblacion: { pesants: 300, artesanos: 0, nobleza: 0 } };
}

const escuadronDe = (heroeId: string): Escuadron => ({
  id: `esc-${heroeId}`,
  nombre: `Milicia de ${heroeId}`,
  heroeId,
  origen: 'pesants',
  cantidad: 20,
  veterania: 0,
  moral: 100,
  tropaId: 'milicia_lanceros',
});

describe('movilizarEjercito sin residir', () => {
  const origen = plaza('faccion-1'); // el jugador NO es fundador ni casa-comprada aquí
  const destino = plaza('faccion-2', [origen]);

  it('un no-residente CON escuadrones propios posados aquí puede sacarlos a campaña', () => {
    const conGuarnicionAjena: Asentamiento = { ...origen, escuadrones: [escuadronDe('jugador-ocupante')] };
    expect(() =>
      movilizarEjercito(
        conGuarnicionAjena,
        undefined,
        'jugador-ocupante',
        ['esc-jugador-ocupante'],
        { tipo: 'asentamiento', id: destino.id },
        [conGuarnicionAjena, destino],
        mapa,
        'ejercito-test-0',
        instanteDeTest(0)
      )
    ).not.toThrow();
  });

  it('un no-residente SIN escuadrones propios aquí NO puede movilizar', () => {
    expect(() =>
      movilizarEjercito(
        origen,
        undefined,
        'jugador-forastero',
        ['esc-x'],
        { tipo: 'asentamiento', id: destino.id },
        [origen, destino],
        mapa,
        'ejercito-test-1',
        instanteDeTest(0)
      )
    ).toThrow(MovilizacionInvalidaError);
  });
});
