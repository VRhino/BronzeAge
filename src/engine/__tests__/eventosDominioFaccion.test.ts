// Fase A5 (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md): `faccion.ts` migrado.
import { describe, expect, it } from 'vitest';
import { NIVEL_FACCION } from '../../constants';
import { avanzarNivelesFaccion, crearFaccion } from '../faccion';
import type { PayloadFaccionNivelSubio } from '../faccion';

describe('eventos de dominio — faccion.ts', () => {
  it('experiencia por encima del primer umbral produce faccion.nivel_subio', () => {
    const faccion = { ...crearFaccion('faccion-1', 'Micenas'), experiencia: NIVEL_FACCION.xpParaNivel[0]! };

    const resultado = avanzarNivelesFaccion([faccion]);

    expect(resultado.eventos).toHaveLength(1);
    const evento = resultado.eventos[0]!;
    if (typeof evento === 'string') throw new Error('esperaba evento migrado');
    expect(evento.codigo).toBe('faccion.nivel_subio');
    const p = evento.payload as PayloadFaccionNivelSubio;
    expect(p.faccionId).toBe('faccion-1');
    expect(p.nivelNuevo).toBe(2);
    expect(resultado.facciones[0]!.nivel).toBe(2);
  });

  it('sin experiencia suficiente no produce ningún evento', () => {
    const faccion = crearFaccion('faccion-1', 'Micenas');

    const resultado = avanzarNivelesFaccion([faccion]);

    expect(resultado.eventos).toHaveLength(0);
  });
});
