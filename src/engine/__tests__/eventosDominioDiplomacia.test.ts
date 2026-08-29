// Fase A5 (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md): `diplomacia.ts` migrado.
import { describe, expect, it } from 'vitest';
import { instanteDeTest } from './fixtures';
import type { Asentamiento, RelacionPolitica } from '../../domain/types';
import { avanzarTributos } from '../diplomacia';
import type { PayloadTributoPagado } from '../diplomacia';
import { almacenSintetico } from './tradeFixtures';

function asentamiento(id: string, faccionId: string, recursos: Record<string, number>): Asentamiento {
  return { id, faccionId, almacen: almacenSintetico(recursos) } as unknown as Asentamiento;
}

describe('eventos de dominio — diplomacia.ts', () => {
  it('un vasallaje con tributo pactado y stock disponible produce diplomacia.tributo_pagado', () => {
    const señora = asentamiento('senora', 'f-senora', { madera: 0 });
    const vasallo = asentamiento('vasallo', 'f-vasallo', { madera: 100 });
    const relacion: RelacionPolitica = {
      id: 'relacion-1',
      tipo: 'vasallaje',
      faccionAId: 'f-senora',
      faccionBId: 'f-vasallo',
      tributo: { recurso: 'madera', cantidadPorMinuto: 10 },
      creadoEn: instanteDeTest(0),
      estado: 'activa',
    };

    const resultado = avanzarTributos([relacion], [señora, vasallo]);

    expect(resultado.eventos).toHaveLength(1);
    const evento = resultado.eventos[0]!;
    if (typeof evento === 'string') throw new Error('esperaba evento migrado');
    expect(evento.codigo).toBe('diplomacia.tributo_pagado');
    const p = evento.payload as PayloadTributoPagado;
    expect(p.pagadorId).toBe('vasallo');
    expect(p.señoraId).toBe('senora');
    expect(p.recurso).toBe('madera');
    expect(p.cantidad).toBe(10);
  });

  it('sin stock del recurso pactado no produce ningún evento', () => {
    const señora = asentamiento('senora', 'f-senora', { madera: 0 });
    const vasallo = asentamiento('vasallo', 'f-vasallo', { madera: 0 });
    const relacion: RelacionPolitica = {
      id: 'relacion-1',
      tipo: 'vasallaje',
      faccionAId: 'f-senora',
      faccionBId: 'f-vasallo',
      tributo: { recurso: 'madera', cantidadPorMinuto: 10 },
      creadoEn: instanteDeTest(0),
      estado: 'activa',
    };

    const resultado = avanzarTributos([relacion], [señora, vasallo]);

    expect(resultado.eventos).toHaveLength(0);
  });
});
