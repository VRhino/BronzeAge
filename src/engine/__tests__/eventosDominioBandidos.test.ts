// Fase A5 (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md): `bandidos.ts` migrado.
import { describe, expect, it } from 'vitest';
import { instanteDeTest } from './fixtures';
import type { CampamentoBandido, Caravana } from '../../domain/types';
import { createRng } from '../../worldgen';
import { avanzarAtaquesBandidos, avanzarSpawnBandidos } from '../bandidos';
import type { PayloadCampamentoAparece, PayloadCaravanaEscapa, PayloadCaravanaInterceptada } from '../bandidos';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest } from './fixtures';

function campamento(poder: number, posicion = { x: 0, y: 0 }): CampamentoBandido {
  return { id: 'campamento-1', posicion, bosqueId: 'bosque-1', asentamientoId: 'asentamiento-x', poder };
}

function caravanaEnTransito(posicion = { x: 0, y: 0 }): Caravana {
  return {
    id: 'caravana-1',
    tipo: 'comercial',
    origenAsentamientoId: 'origen',
    destinoAsentamientoId: 'destino',
    contenido: { madera: 10 },
    posicionActual: posicion,
    progreso: 0.5,
    estado: 'en_transito',
  };
}

describe('eventos de dominio — bandidos.ts', () => {
  it('avanzarSpawnBandidos: asentamiento sin campamento produce bandidos.campamento_aparece', () => {
    const mapa = crearMapaDeterminista(7);
    const { asentamiento } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);

    const resultado = avanzarSpawnBandidos([], instanteDeTest(0), [], [asentamiento], mapa, instanteDeTest(1), 1);

    expect(resultado.campamentos).toHaveLength(1);
    expect(resultado.eventos).toHaveLength(1);
    const evento = resultado.eventos[0]!;
    if (typeof evento === 'string') throw new Error('esperaba evento migrado');
    expect(evento.codigo).toBe('bandidos.campamento_aparece');
    const p = evento.payload as PayloadCampamentoAparece;
    expect(p.asentamientoObjetivoId).toBe(asentamiento.id);
    expect(p.campamentoId).toBe(resultado.campamentos[0]!.id);
  });

  it('avanzarAtaquesBandidos: campamento con poder de sobra intercepta y destruye la caravana', () => {
    // poder muy por encima de defensaBaseCaravana (15): incluso el jitter mínimo (×0.85) gana siempre.
    const camp = campamento(100);
    const caravana = caravanaEnTransito();

    const resultado = avanzarAtaquesBandidos([camp], [caravana], createRng(1));

    expect(resultado.caravanas).toHaveLength(0);
    expect(resultado.eventos).toHaveLength(1);
    const evento = resultado.eventos[0]!;
    if (typeof evento === 'string') throw new Error('esperaba evento migrado');
    expect(evento.codigo).toBe('bandidos.caravana_interceptada');
    const p = evento.payload as PayloadCaravanaInterceptada;
    expect(p.campamentoId).toBe('campamento-1');
    expect(p.caravanaId).toBe('caravana-1');
  });

  it('avanzarAtaquesBandidos: campamento muy débil deja escapar la caravana', () => {
    // poder muy por debajo de defensaBaseCaravana (15): incluso el jitter máximo (×1.15) pierde siempre.
    const camp = campamento(1);
    const caravana = caravanaEnTransito();

    const resultado = avanzarAtaquesBandidos([camp], [caravana], createRng(1));

    expect(resultado.caravanas).toHaveLength(1);
    expect(resultado.eventos).toHaveLength(1);
    const evento = resultado.eventos[0]!;
    if (typeof evento === 'string') throw new Error('esperaba evento migrado');
    expect(evento.codigo).toBe('bandidos.caravana_escapa');
    const p = evento.payload as PayloadCaravanaEscapa;
    expect(p.caravanaId).toBe('caravana-1');
  });
});
