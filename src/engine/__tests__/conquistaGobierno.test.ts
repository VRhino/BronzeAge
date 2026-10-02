// Al conquistar una plaza, las decisiones del gobierno derrotado caen con él (decisión del usuario, 2026-10-02): el
// nuevo dueño no hereda políticas, reserva manual, pausa de la auto-construcción ni recetas paradas.
import { describe, expect, it } from 'vitest';
import { aplicarConquista } from '../combate';
import { crearFacciones, crearMapaDeterminista, fundarAsentamientoDeTest, instanteDeTest } from './fixtures';

describe('conquista: el gobierno derrotado no deja decisiones', () => {
  it('limpia políticas activas, reserva manual, pausa de auto-construcción y recetas paradas', () => {
    const mapa = crearMapaDeterminista(7);
    const { asentamiento } = fundarAsentamientoDeTest(mapa, crearFacciones(), 'faccion-1', []);
    const gobernada = {
      ...asentamiento,
      politicasActivas: [{ id: 'p-1', politicaId: 'presion_fiscal', cargo: 'tesorero' as const, activadaEn: instanteDeTest(0), expiraEn: instanteDeTest(600) }],
      reservaManual: { madera: 150 },
      autoConstruccionPausada: true,
      recetasPausadas: ['lingoteCobre' as never],
    };
    const conquistada = aplicarConquista(gobernada, 'faccion-2', instanteDeTest(1));
    expect(conquistada.faccionId).toBe('faccion-2');
    expect(conquistada.politicasActivas).toEqual([]);
    expect(conquistada.reservaManual).toBeUndefined();
    expect(conquistada.autoConstruccionPausada).toBeUndefined();
    expect(conquistada.recetasPausadas).toBeUndefined();
  });
});
