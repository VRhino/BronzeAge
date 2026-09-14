// Fase A5 (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md): `tropas.ts` migrado. `avanzarMantenimientoTropas`
// es directamente testeable: se construye un escuadrón con moral ya colapsada y se llama a la función a mano.
import { describe, expect, it } from 'vitest';
import type { Asentamiento } from '../../domain/types';
import { avanzarMantenimientoTropas, type PayloadTropasDesercion } from '../tropas';
import { crearFacciones, crearMapaDeterminista, escuadronDePrueba, fundarAsentamientoDeTest } from './fixtures';

/** Sin trigo: `factorSuministro` cae a 0, la moral no se recupera y la deserción se dispara este mismo tick. */
function sinTrigo(asentamiento: Asentamiento): Asentamiento {
  return { ...asentamiento, almacen: { ...asentamiento.almacen, trigo: { cantidad: 0, capacidad: 1000 } } };
}

describe('eventos de dominio — tropas.ts', () => {
  it('un escuadrón con moral colapsada y sin ración produce tropas.desercion', () => {
    const mapa = crearMapaDeterminista(7);
    const { asentamiento } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
    const campamento = [
      escuadronDePrueba(`escuadron-${asentamiento.id}-0`, 'jugador-1', 'lancerosPesants', 10, { moral: 0, nombre: 'Lanceros de jugador-1' }),
    ];

    const resultado = avanzarMantenimientoTropas(sinTrigo(asentamiento), campamento);

    expect(resultado.eventos).toHaveLength(1);
    const evento = resultado.eventos[0]!;
    if (typeof evento === 'string') throw new Error('esperaba un evento migrado, llegó texto plano ("legado")');
    expect(evento.codigo).toBe('tropas.desercion');
    const p = evento.payload as PayloadTropasDesercion;
    expect(p.escuadronId).toBe('escuadron-' + asentamiento.id + '-0');
    expect(p.escuadronNombre).toBe('Lanceros de jugador-1');
    expect(p.desertores).toBeGreaterThan(0);
  });

  it('sin escuadrones no produce ningún evento', () => {
    const mapa = crearMapaDeterminista(7);
    const { asentamiento } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);

    const resultado = avanzarMantenimientoTropas(asentamiento, []);

    expect(resultado.eventos).toHaveLength(0);
  });
});
