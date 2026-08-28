// Fase A5 (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md): `expansion.ts` migrado.
import { describe, expect, it } from 'vitest';
import type { Caravana } from '../../domain/types';
import { avanzarCaravanasFundacion } from '../expansion';
import type { PayloadAsentamientoFundado, PayloadCaravanaFundacionPerdida, PayloadFundacionFallida } from '../expansion';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, posicionRecomendable } from './fixtures';

function caravanaFundacionLlegando(origenId: string, destino: { x: number; y: number }, overrides: Partial<Caravana> = {}): Caravana {
  return {
    id: 'caravana-fundacion-1',
    tipo: 'construccion',
    origenAsentamientoId: origenId,
    contenido: {},
    posicionActual: destino,
    progreso: 0.9999999,
    destinoPosicion: destino,
    jugadoresFundadoresIds: ['jugador-faccion-1-1'],
    ruta: [{ x: 0, y: 0 }, destino],
    ...overrides,
  };
}

describe('eventos de dominio — expansion.ts', () => {
  it('caravana cuyo origen ya no existe produce expansion.caravana_perdida', () => {
    const mapa = crearMapaDeterminista(7);
    const caravana = caravanaFundacionLlegando('asentamiento-inexistente', { x: 500, y: 500 });

    const resultado = avanzarCaravanasFundacion([caravana], mapa, crearFacciones(), [], 1);

    expect(resultado.eventos).toHaveLength(1);
    const evento = resultado.eventos[0]!;
    if (typeof evento === 'string') throw new Error('esperaba evento migrado');
    expect(evento.codigo).toBe('expansion.caravana_perdida');
    const p = evento.payload as PayloadCaravanaFundacionPerdida;
    expect(p.caravanaId).toBe('caravana-fundacion-1');
  });

  it('caravana que llega a un destino viable funda un asentamiento y produce expansion.asentamiento_fundado', () => {
    const mapa = crearMapaDeterminista(7);
    const facciones = crearFacciones();
    const { asentamiento: origen, facciones: faccionesTrasFundar } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);
    const destino = posicionRecomendable(mapa, [origen], 40);
    const caravana = caravanaFundacionLlegando(origen.id, destino);
    // Nivel 2 de Facción: cap de fundación sube a 2 (nivel 1 = 1, ya ocupado por `origen`) — sin esto,
    // `fundarAsentamiento` rechaza por cap alcanzado en vez de fundar.
    const faccionesConCap = faccionesTrasFundar.map((f) => (f.id === 'faccion-1' ? { ...f, nivel: 2 } : f));

    const resultado = avanzarCaravanasFundacion([caravana], mapa, faccionesConCap, [origen], 1);

    const evento = resultado.eventos.find((e) => typeof e !== 'string' && e.codigo === 'expansion.asentamiento_fundado');
    expect(evento).toBeDefined();
    const p = (evento as { payload: unknown }).payload as PayloadAsentamientoFundado;
    expect(p.caravanaId).toBe('caravana-fundacion-1');
    expect(resultado.asentamientos.some((a) => a.id === p.asentamientoId)).toBe(true);
  });

  it('caravana que llega a un destino ya ocupado produce expansion.fundacion_fallida', () => {
    const mapa = crearMapaDeterminista(7);
    const facciones = crearFacciones();
    const { asentamiento: origen, facciones: faccionesTrasFundar } = fundarAsentamientoDeTest(mapa, facciones, 'faccion-1', []);
    // Destino = la posición del propio origen: cae dentro de su propia zona de influencia, rechazo garantizado.
    const caravana = caravanaFundacionLlegando(origen.id, origen.posicion);

    const resultado = avanzarCaravanasFundacion([caravana], mapa, faccionesTrasFundar, [origen], 1);

    expect(resultado.eventos).toHaveLength(1);
    const evento = resultado.eventos[0]!;
    if (typeof evento === 'string') throw new Error('esperaba evento migrado');
    expect(evento.codigo).toBe('expansion.fundacion_fallida');
    const p = evento.payload as PayloadFundacionFallida;
    expect(p.caravanaId).toBe('caravana-fundacion-1');
    expect(p.razon).toBeTruthy();
  });
});
