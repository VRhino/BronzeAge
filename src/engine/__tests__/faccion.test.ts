// Residencia y membresía (Doc 2.1/2.5): un jugador reside en UN solo asentamiento —es lo que le permite tener como mucho un
// escuadrón de cada tropa (`Escuadron.heroeId`, `reclutarTropa`)—, y entra en una Facción pidiéndolo al Rey (D46).
import { describe, expect, it } from 'vitest';
import type { Asentamiento, Faccion } from '../../domain/types';
import { crearFacciones, crearMapaDeterminista, escuadronDePrueba, heroeDePrueba, posicionRecomendable, instanteDeTest } from './fixtures';
import { fundarAsentamiento } from '../settlement';
import { cambiarResidencia, FaccionInvalidaError, otorgarCiudadania, responderSolicitud, solicitarIngreso } from '../faccion';
import { campamentoDe } from '../tropa';

/** El cap de fundación en nivel 1 es 1 asentamiento por Facción (`CAP_FUNDACION_POR_NIVEL`, constants.ts) —
 * para probar residencia cruzada entre DOS asentamientos de la misma Facción, se sube el nivel a mano tras
 * fundar el primero (no es una regla que este test esté validando, solo un requisito previo del motor). */
function fundarDosAsentamientosDeFaccion(): { asentamientoA: Asentamiento; asentamientoB: Asentamiento; facciones: Faccion[] } {
  const mapa = crearMapaDeterminista(1);
  const facciones = crearFacciones();

  const { asentamiento: asentamientoA, facciones: faccionesTrasA } = fundarAsentamiento(
    mapa,
    facciones,
    'faccion-1',
    posicionRecomendable(mapa),
    ['jugador-a'],
    [],
    instanteDeTest(0)
  );
  const faccionesNivel2 = faccionesTrasA.map((f) => (f.id === 'faccion-1' ? { ...f, nivel: 2 } : f));
  const { asentamiento: asentamientoB, facciones: faccionesTrasB } = fundarAsentamiento(
    mapa,
    faccionesNivel2,
    'faccion-1',
    posicionRecomendable(mapa, [asentamientoA]),
    ['jugador-b'],
    [asentamientoA],
    instanteDeTest(0)
  );

  return { asentamientoA, asentamientoB, facciones: faccionesTrasB };
}

describe('solicitudes de ingreso (D46, D49)', () => {
  const [faccion] = crearFacciones();

  it('pedir entra en la lista una vez; en una Facción NPC no se entra; un ciudadano no pide', () => {
    const pedida = solicitarIngreso(faccion!, [faccion!], 'nuevo', false);
    expect(pedida.solicitudesIds).toEqual(['nuevo']);
    expect(solicitarIngreso(pedida, [pedida], 'nuevo', false)).toBe(pedida);
    expect(() => solicitarIngreso(faccion!, [faccion!], 'nuevo', true)).toThrow(FaccionInvalidaError);
    const conCiudadano = otorgarCiudadania(faccion!, 'ya');
    expect(() => solicitarIngreso(conCiudadano, [conCiudadano], 'ya', false)).toThrow(FaccionInvalidaError);
  });

  it('aceptada da ciudadanía y quita la solicitud; denegada solo la quita', () => {
    const pedida = solicitarIngreso(faccion!, [faccion!], 'nuevo', false);
    expect(responderSolicitud([pedida], pedida.id, 'nuevo', true)[0]).toMatchObject({ solicitudesIds: [], ciudadanosIds: expect.arrayContaining(['nuevo']) });
    const denegada = responderSolicitud([pedida], pedida.id, 'nuevo', false)[0]!;
    expect(denegada.solicitudesIds).toEqual([]);
    expect(denegada.ciudadanosIds).not.toContain('nuevo');
  });
});

describe('cambiarResidencia (Doc 2.5/2.6, comando nuevo)', () => {
  it('mueve al jugador: fuera de la vieja (fundador y casa), dentro de la nueva', () => {
    const { asentamientoA, asentamientoB, facciones } = fundarDosAsentamientosDeFaccion();
    const conCargo: Asentamiento = { ...asentamientoA, cargos: { ...asentamientoA.cargos, gobernadorId: 'jugador-a' } };

    const { origen, destino } = cambiarResidencia(facciones, [conCargo, asentamientoB], asentamientoB.id, 'jugador-a');

    expect(origen!.heroesFundadoresIds).not.toContain('jugador-a');
    expect(origen!.casasCompradas).not.toContain('jugador-a');
    expect(origen!.cargos.gobernadorId).toBeNull(); // cargo local vacío al mudarse
    expect(destino.casasCompradas).toContain('jugador-a');
  });

  it('el campamento se muda con el héroe: sus escuadras pasan a la residencia nueva (Doc 5.15.2)', () => {
    const { asentamientoA, asentamientoB, facciones } = fundarDosAsentamientosDeFaccion();
    const heroes = [
      heroeDePrueba('jugador-a', { tipo: 'asentamiento', asentamientoId: asentamientoA.id }, { escuadrones: [escuadronDePrueba('e1', 'jugador-a')] }),
    ];
    expect(campamentoDe(asentamientoA, heroes).map((e) => e.id)).toEqual(['e1']);

    const { origen, destino } = cambiarResidencia(facciones, [asentamientoA, asentamientoB], asentamientoB.id, 'jugador-a');

    expect(campamentoDe(origen!, heroes)).toEqual([]);
    expect(campamentoDe(destino, heroes).map((e) => e.id)).toEqual(['e1']);
  });

  it('un ciudadano sin plaza (vive en un campamento) entra sin dejar ninguna', () => {
    const { asentamientoA, asentamientoB, facciones } = fundarDosAsentamientosDeFaccion();
    const conC = facciones.map((f) => (f.id === asentamientoB.faccionId ? otorgarCiudadania(f, 'jugador-c') : f));
    const { origen, destino } = cambiarResidencia(conC, [asentamientoA, asentamientoB], asentamientoB.id, 'jugador-c');
    expect(origen).toBeUndefined();
    expect(destino.casasCompradas).toContain('jugador-c');
  });

  it('un jugador de otra Facción no puede residir aquí', () => {
    const { asentamientoA, asentamientoB, facciones } = fundarDosAsentamientosDeFaccion();
    expect(() => cambiarResidencia(facciones, [asentamientoA, asentamientoB], asentamientoB.id, 'jugador-de-faccion-2')).toThrow(
      FaccionInvalidaError
    );
  });
});
