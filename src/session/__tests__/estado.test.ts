// `idDeMapa`/`vistaAdminDeEstado` (Fase C11, doc 9: "el mapa deja de ser estado, es un asset"). Sobre estado
// GENUINO (misma fixture que el resto de `session/`), no un `MapaGenerado` fabricado a mano — así estas
// pruebas fallan de verdad si cambia la forma real de `GameSessionState.mapa`.
import { describe, expect, it } from 'vitest';
import { GameSession } from '../gameSession';
import { idDeMapa, vistaAdminDeEstado } from '../estado';

describe('idDeMapa', () => {
  it('es estable: la misma partida produce el mismo id en llamadas repetidas', () => {
    const sesion = GameSession.crear('g1', { seed: 42 });
    const estado = sesion.getState();

    expect(idDeMapa(estado.mapa)).toBe(idDeMapa(estado.mapa));
  });

  it('no cambia al avanzar el tick: el mapa es inmutable durante toda la partida', () => {
    const sesion = GameSession.crear('g1', { seed: 42 });
    const idAntes = idDeMapa(sesion.getState().mapa);

    sesion.avanzarTick('2026-01-01T00:00:00.000Z');

    expect(idDeMapa(sesion.getState().mapa)).toBe(idAntes);
  });

  it('dos seeds distintas producen ids distintos', () => {
    const a = GameSession.crear('a', { seed: 1 });
    const b = GameSession.crear('b', { seed: 2 });

    expect(idDeMapa(a.getState().mapa)).not.toBe(idDeMapa(b.getState().mapa));
  });

  it('incluye la seed en texto legible, no es un hash opaco', () => {
    const sesion = GameSession.crear('g1', { seed: 12345 });
    expect(idDeMapa(sesion.getState().mapa)).toContain('12345');
  });
});

describe('vistaAdminDeEstado', () => {
  it('quita `mapa` y añade `mapaId`, sin tocar el resto del estado', () => {
    const sesion = GameSession.crear('g1', { seed: 7 });
    const estado = sesion.getState();

    const vista = vistaAdminDeEstado(estado);

    expect(vista).not.toHaveProperty('mapa');
    expect(vista.mapaId).toBe(idDeMapa(estado.mapa));
    expect(vista.tick).toBe(estado.tick);
    expect(vista.asentamientos).toBe(estado.asentamientos);
  });
});
