// Fase A5 (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md): `faccion.ts` migrado.
import { describe, expect, it } from 'vitest';
import { NIVEL_FACCION } from '../../constants';
import { aplicarExperiencia, avanzarNivelesFaccion, crearFaccion } from '../faccion';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest } from './fixtures';
import { fundarAsentamiento } from '../settlement';
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

  it('quien da la experiencia sube el nivel en el momento, con su evento, y no deja nada para el barrido del tick', () => {
    const faccion = { ...crearFaccion('faccion-1', 'Micenas'), experiencia: NIVEL_FACCION.xpParaNivel[0]! - 5 };

    const sinCruzar = aplicarExperiencia([faccion], [{ faccionId: 'faccion-1', delta: 1, razon: 'prueba' }]);
    expect(sinCruzar.facciones[0]!.nivel).toBe(1);
    expect(sinCruzar.eventos).toHaveLength(0);

    const cruzando = aplicarExperiencia([faccion], [{ faccionId: 'faccion-1', delta: 10, razon: 'prueba' }]);
    expect(cruzando.facciones[0]!.nivel, 'el nivel sube con la experiencia, no en el tick siguiente').toBe(2);
    expect(cruzando.eventos.map((e) => (typeof e === 'string' ? e : e.codigo))).toEqual(['faccion.nivel_subio']);
  });

  it('fundar un asentamiento nuevo que cruza un umbral sube el nivel en la fundación', () => {
    const mapa = crearMapaDeterminista(1);
    const base = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
    // Nivel 2 (el cap de nivel 1 es una sola plaza) y a una fundación de cruzar al 3.
    const cerca = { ...base.facciones[0]!, nivel: 2, experiencia: NIVEL_FACCION.xpParaNivel[1]! - NIVEL_FACCION.xp.fundacion };
    const facciones = base.facciones.map((f) => (f.id === cerca.id ? cerca : f));

    const resultado = fundarAsentamiento(mapa, facciones, 'faccion-1', { x: 1300, y: 1300 }, ['heroe-2'], [base.asentamiento], base.asentamiento.fundadoEn);

    expect(resultado.facciones.find((f) => f.id === 'faccion-1')!.nivel).toBe(3);
    expect(resultado.eventos.map((e) => (typeof e === 'string' ? e : e.codigo))).toEqual(['faccion.nivel_subio']);
  });
});
