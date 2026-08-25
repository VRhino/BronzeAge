// Fase A5 (Docs/Arquitectura/4_Plan_Evolucion_Tareas.md): `politicas.ts` migrado. `avanzarPoliticas` es
// directamente testeable sin correr un tick completo — se construye un `Asentamiento` con una política ya
// vencida y se llama a la función a mano.
import { describe, expect, it } from 'vitest';
import type { Asentamiento, PoliticaActiva } from '../../domain/types';
import { avanzarPoliticas, type PayloadPoliticaExpirada } from '../politicas';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest } from './fixtures';

function conPoliticaVencida(asentamiento: Asentamiento): Asentamiento {
  const activa: PoliticaActiva = {
    id: `politica-${asentamiento.id}-0`,
    politicaId: 'edicto_cosecha',
    cargo: 'gobernador',
    activadaEnTick: 0,
    expiraEnTick: 5,
  };
  return { ...asentamiento, politicasActivas: [activa] };
}

describe('eventos de dominio — politicas.ts', () => {
  it('una política vencida produce politica.expirada con su id/nombre/cargo', () => {
    const mapa = crearMapaDeterminista(7);
    const { asentamiento } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
    const conVencida = conPoliticaVencida(asentamiento);

    const resultado = avanzarPoliticas(conVencida, 10);

    expect(resultado.asentamiento.politicasActivas).toHaveLength(0);
    expect(resultado.eventos).toHaveLength(1);
    const evento = resultado.eventos[0]!;
    if (typeof evento === 'string') throw new Error('esperaba un evento migrado, llegó texto plano ("legado")');
    expect(evento.codigo).toBe('politica.expirada');
    expect(evento.mensaje).toContain('expira');
    const p = evento.payload as PayloadPoliticaExpirada;
    expect(p.politicaId).toBe('edicto_cosecha');
    expect(p.cargo).toBe('gobernador');
    expect(p.politicaNombre).toBeTruthy();
  });

  it('una política vigente no produce ningún evento', () => {
    const mapa = crearMapaDeterminista(7);
    const { asentamiento } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
    const conVigente = conPoliticaVencida(asentamiento);

    const resultado = avanzarPoliticas(conVigente, 3);

    expect(resultado.eventos).toHaveLength(0);
  });
});
